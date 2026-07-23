//! JSON Schema emission — the cross-language contract.
//!
//! The base schema is **derived** from [`Record`] by `schemars`, so it cannot
//! drift from the struct: rename a field and the derived schema renames with it.
//! It is then *enriched* with the descriptions, closed sets and formats a bare
//! derive cannot know. The enrichment looks every property up by name and
//! **panics if it is missing**, so a rename fails the build loudly instead of
//! silently dropping documentation.
use crate::{Record, Stability, Type, CONTRACT_VERSION, PRECEDENCE};
use serde_json::{json, Map, Value};

/// The canonical schema identifier, served as a plain GET endpoint.
pub const SCHEMA_ID: &str = "https://argenv-opencommons.github.io/argenv/v1/contract.schema.json";

/// Documentation for each field of a record.
const FIELD_DOCS: &[(&str, &str)] = &[
    ("key", "Transport-free identity of the input, snake_case. The join key across every source and every generated binding."),
    ("type", "The kind of value. Also fixes how many values the argument form takes: none for bool, one otherwise."),
    ("default", "Value used when nothing supplies the input, in its natural JSON form."),
    ("allowed", "Accepted tokens; meaningful for the enum and list kinds."),
    ("separators", "Characters that separate items in a list value, so a consumer can split it."),
    ("required", "Whether the program requires this input."),
    ("stability", "How much a consumer may rely on this input."),
    ("since", "Version in which the input was introduced."),
    ("deprecation", "Present only when stability is deprecated: when, what replaces it, and how to migrate."),
    ("group", "Free-form grouping tag for documentation and help output."),
    ("maps_to", "A namespaced key in another configuration surface this input bridges to."),
    ("example", "A copy-pasteable usage example."),
    ("observe", "A field in the program's resolved-configuration output that this input visibly changes; the hook for confirming a declaration empirically."),
    ("reviewed", "Date a human last verified this entry. Hand-written: it asserts human judgement that cannot be derived."),
    ("summary", "What the input does, in prose."),
    ("modified", "Date the declaration was last edited. Stamped by tooling from version control, never hand-written."),
    ("source", "Project and revision this record came from; populated when records from several documents are merged."),
    ("env", "How the value may arrive from the environment."),
    ("arg", "How the value may arrive from the argument vector."),
];

const KEY_PATTERN: &str = r"^[a-z][a-z0-9_]*$";
const ENV_NAME_PATTERN: &str = r"^[A-Za-z_][A-Za-z0-9_]*$";
const LONG_PATTERN: &str = r"^[a-z][a-z0-9-]*$";
const SHORT_PATTERN: &str = r"^[A-Za-z0-9]$";
const VERSION_PATTERN: &str = r"^\d+(\.\d+){0,2}([-+][0-9A-Za-z.-]+)?$";
const DATE_PATTERN: &str = r"^\d{4}-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])$";

/// Build the JSON Schema for a contract document.
///
/// # Panics
/// If enrichment names a property the derived schema does not have, which means
/// the wire types were renamed without updating the documentation table.
pub fn json_schema() -> Value {
    let derived =
        serde_json::to_value(schemars::schema_for!(Record)).expect("the Record schema serialises");

    // schemars emits nested types as `$ref`s into a definitions map. Hoist that
    // map to the envelope root so every reference still resolves.
    let definitions = derived
        .get("definitions")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let mut props = derived
        .get("properties")
        .and_then(Value::as_object)
        .cloned()
        .expect("the derived Record schema has properties");

    for (field, doc) in FIELD_DOCS {
        let entry = props.get_mut(*field).unwrap_or_else(|| {
            panic!("enrichment names unknown Record field `{field}` — was it renamed?")
        });
        if let Some(obj) = entry.as_object_mut() {
            obj.insert("description".into(), json!(doc));
        }
    }

    set_enum(&mut props, "type", Type::ALL);
    set_enum(&mut props, "stability", Stability::ALL);
    set_pattern(&mut props, "key", KEY_PATTERN);
    set_pattern(&mut props, "since", VERSION_PATTERN);
    set_pattern(&mut props, "reviewed", DATE_PATTERN);
    set_pattern(&mut props, "modified", DATE_PATTERN);

    let mut definitions = definitions;
    enrich_bindings(&mut definitions);

    let input_schema = json!({
        "type": "object",
        "title": "Input record",
        "description": "One input a program accepts, and the doors it can arrive through.",
        "required": ["key", "type"],
        // Unknown properties are permitted on purpose: the contract grows by
        // adding optional fields, and a validator pinned to this version must
        // keep accepting documents that carry a field it has not heard of.
        // Authoring typos are caught by the checker, which reports unrecognised
        // fields explicitly rather than refusing the document.
        "additionalProperties": true,
        "properties": props,
    });

    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": SCHEMA_ID,
        "title": "ArgEnv contract",
        "description":
            "A declared, machine-readable contract for a program's invocation \
             surface: its argument vector and its environment.",
        "type": "object",
        "required": ["contract_version", "inputs"],
        "additionalProperties": false,
        "definitions": definitions,
        "properties": {
            "contract_version": {
                "type": "integer",
                "const": CONTRACT_VERSION,
                "description": "Major version of this contract format."
            },
            "source": {
                "type": "string",
                "description": "Emitting project and revision, e.g. \"myapp@a1b2c3d\". Stamped by the build."
            },
            "generated": {
                "type": "string",
                "format": "date-time",
                "description": "Emission timestamp (RFC 3339, UTC). Stamped by the emitter."
            },
            "precedence": {
                "type": "array",
                "items": { "enum": PRECEDENCE },
                "description": "Which binding wins when more than one supplies a value, strongest first."
            },
            "inputs": {
                "type": "array",
                "description": "Every input this program accepts.",
                "items": input_schema
            }
        }
    })
}

/// One field of a nested binding: its name, its description, and the pattern its
/// values must match, when it has one.
type BindingField = (&'static str, &'static str, Option<&'static str>);

/// A nested binding type: the name schemars gave its definition, and its fields.
type BindingDocs = (&'static str, &'static [BindingField]);

/// Document and constrain the nested binding types, which schemars emits as
/// separate definitions.
fn enrich_bindings(definitions: &mut Value) {
    let docs: &[BindingDocs] = &[
        (
            "EnvBinding",
            &[
                ("name", "The variable name.", Some(ENV_NAME_PATTERN)),
                ("aliases", "Other names still honoured, so a legacy name is recognised rather than reported as a mistake.", None),
            ],
        ),
        (
            "ArgBinding",
            &[
                ("long", "The long form, without dashes.", Some(LONG_PATTERN)),
                ("short", "The short form, without its dash.", Some(SHORT_PATTERN)),
                ("arity", "How many values the flag consumes: 0 for a boolean whose presence is the value, 1 otherwise. Derived from the type and written out explicitly so no consumer has to re-derive it.", None),
                ("value_name", "The placeholder shown in help for the value.", None),
                ("negatable", "Whether --no-<long> is accepted.", None),
                ("repeatable", "Whether the flag may be repeated, accumulating values.", None),
            ],
        ),
    ];

    for (type_name, fields) in docs {
        let Some(def) = definitions.get_mut(*type_name) else {
            panic!("the derived schema has no definition for `{type_name}`");
        };
        let Some(props) = def.get_mut("properties").and_then(Value::as_object_mut) else {
            panic!("`{type_name}` has no properties");
        };
        for (field, doc, pattern) in *fields {
            let entry = props
                .get_mut(*field)
                .unwrap_or_else(|| panic!("`{type_name}` has no field `{field}`"));
            if let Some(obj) = entry.as_object_mut() {
                obj.insert("description".into(), json!(doc));
                if let Some(p) = pattern {
                    obj.insert("pattern".into(), json!(p));
                }
            }
        }
    }
}

fn set_enum(props: &mut Map<String, Value>, field: &str, values: &[&str]) {
    let entry = props
        .get_mut(field)
        .unwrap_or_else(|| panic!("unknown Record field `{field}`"));
    if let Some(obj) = entry.as_object_mut() {
        obj.remove("type");
        obj.remove("format");
        obj.insert("enum".into(), json!(values));
    }
}

fn set_pattern(props: &mut Map<String, Value>, field: &str, pattern: &str) {
    let entry = props
        .get_mut(field)
        .unwrap_or_else(|| panic!("unknown Record field `{field}`"));
    if let Some(obj) = entry.as_object_mut() {
        obj.insert("pattern".into(), json!(pattern));
    }
}
