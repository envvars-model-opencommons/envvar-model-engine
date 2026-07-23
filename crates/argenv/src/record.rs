//! [`Record`] — the portable projection, and the document envelope.
use crate::date::now_iso;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Bump on any **breaking** change to [`Record`]. Additive changes (a new
/// optional field) keep the major version and stay backward-compatible.
pub const CONTRACT_VERSION: u32 = 1;

/// The order in which bindings win when more than one supplies a value.
///
/// Uniform across a document, so it lives in the envelope rather than being
/// repeated on every record. An explicit argument beats an ambient environment
/// variable, which beats a compiled-in default — the order users expect, stated
/// rather than assumed.
pub const PRECEDENCE: &[&str] = &["arg", "env", "default"];

/// How a value may arrive from the environment.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "contract", derive(schemars::JsonSchema))]
pub struct EnvBinding {
    /// The variable name.
    pub name: String,
    /// Other names still honoured.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
}

/// How a value may arrive from the argument vector.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "contract", derive(schemars::JsonSchema))]
pub struct ArgBinding {
    /// The long form, without dashes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long: Option<String>,
    /// The short form, without its dash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    /// How many values the flag consumes: `0` for a boolean whose presence is
    /// the value, `1` otherwise.
    ///
    /// Derived from the type by the emitter and written out explicitly, so no
    /// consumer in any language has to re-derive it — and so a generated parser
    /// can never disagree with the one that published the contract.
    pub arity: u8,
    /// The placeholder shown in help for the value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_name: Option<String>,
    /// Whether `--no-<long>` is accepted.
    #[serde(default)]
    pub negatable: bool,
    /// Whether the flag may be repeated, accumulating values.
    #[serde(default)]
    pub repeatable: bool,
}

/// One input, flattened to plain JSON-friendly types.
///
/// This is the shape every language binding implements and the JSON Schema
/// describes. Rich types collapse to strings here on purpose: the contract must
/// be expressible in C, Python, and JSON, none of which have const-validated
/// newtypes.
///
/// Unknown fields are **ignored** rather than rejected on read: the format grows
/// by adding optional fields, so a consumer pinned to this version must keep
/// reading documents produced by a later one. Authoring mistakes are caught by
/// the command-line checker, which reports unrecognised fields explicitly.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "contract", derive(schemars::JsonSchema))]
pub struct Record {
    /// Transport-free identity, `snake_case`. The join key across every source.
    pub key: String,
    /// Wire kind: one of [`crate::Type::ALL`].
    #[serde(rename = "type")]
    pub ty: String,
    /// The declared default, in its natural JSON form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    /// Accepted tokens for the `enum` and `list` kinds.
    #[serde(default)]
    pub allowed: Vec<String>,
    /// Characters that separate items in a `list` value.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub separators: Vec<String>,
    /// Whether the program requires this input.
    #[serde(default)]
    pub required: bool,
    /// One of [`crate::Stability::ALL`].
    #[serde(default = "unknown_stability")]
    pub stability: String,
    /// Introducing version, `major.minor.patch`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
    /// `{ since, replaced_by, migration }`, present only when deprecated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecation: Option<Value>,
    /// Grouping tag for documentation and help output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// A key in another configuration surface this input bridges to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maps_to: Option<String>,
    /// A copy-pasteable example.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
    /// A resolved-configuration field this input visibly changes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observe: Option<String>,
    /// Date a human last verified the entry, `YYYY-MM-DD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed: Option<String>,
    /// Prose description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Date the declaration was last edited. Stamped by tooling from version
    /// control, never hand-written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
    /// Which project and revision this record came from; populated when records
    /// from several documents are merged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// The environment binding, if this input accepts one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<EnvBinding>,
    /// The argument-vector binding, if this input accepts one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arg: Option<ArgBinding>,
}

fn unknown_stability() -> String {
    "unknown".to_string()
}

impl Record {
    /// Every environment name this input answers to.
    pub fn env_names(&self) -> Vec<&str> {
        match &self.env {
            None => Vec::new(),
            Some(e) => std::iter::once(e.name.as_str())
                .chain(e.aliases.iter().map(String::as_str))
                .collect(),
        }
    }

    /// Every argument spelling this input answers to, dashes included.
    pub fn arg_labels(&self) -> Vec<String> {
        let mut v = Vec::new();
        if let Some(a) = &self.arg {
            if let Some(s) = &a.short {
                v.push(format!("-{s}"));
            }
            if let Some(l) = &a.long {
                v.push(format!("--{l}"));
                if a.negatable {
                    v.push(format!("--no-{l}"));
                }
            }
        }
        v
    }

    /// The signature as it would appear in help output, e.g.
    /// `-l, --log-level <LEVEL>`. Empty when the input has no argument form.
    pub fn usage(&self) -> String {
        let Some(a) = &self.arg else {
            return String::new();
        };
        let mut parts = Vec::new();
        if let Some(s) = &a.short {
            parts.push(format!("-{s}"));
        }
        if let Some(l) = &a.long {
            parts.push(format!("--{l}"));
        }
        let mut out = parts.join(", ");
        if a.arity > 0 {
            let placeholder = a.value_name.clone().unwrap_or_else(|| "VALUE".to_string());
            out.push_str(&format!(" <{placeholder}>"));
        }
        out
    }

    /// Whether `raw` is a legal value, judged from the declaration's metadata
    /// alone.
    ///
    /// This is the cross-language check: it uses only what the published
    /// contract carries, so any consumer can perform it without the originating
    /// program's types.
    pub fn accepts(&self, raw: &str) -> Result<(), String> {
        let v = raw.trim();
        match self.ty.as_str() {
            "bool" => match v {
                "1" | "0" | "true" | "false" | "True" | "False" | "on" | "off" | "yes" | "no"
                | "" => Ok(()),
                _ => Err("expected a boolean".into()),
            },
            "uint" => v
                .parse::<u64>()
                .map(|_| ())
                .map_err(|_| "expected a non-negative integer".to_string()),
            "int" => v
                .parse::<i64>()
                .map(|_| ())
                .map_err(|_| "expected an integer".to_string()),
            "float" => v
                .parse::<f64>()
                .map(|_| ())
                .map_err(|_| "expected a number".to_string()),
            "enum" => {
                if self.allowed.iter().any(|a| a == v) {
                    Ok(())
                } else {
                    Err(format!("accepted: {}", self.allowed.join(", ")))
                }
            }
            // Token membership for lists is reported per token, so the caller can
            // name the offender; the value as a whole is always well formed.
            "list" | "path" | "string" => Ok(()),
            other => Err(format!("unknown declared type `{other}`")),
        }
    }

    /// Split a list value into its tokens using the declared separators. Empty
    /// when this input is not a list.
    pub fn tokens<'a>(&self, raw: &'a str) -> Vec<&'a str> {
        if self.ty != "list" || self.separators.is_empty() {
            return Vec::new();
        }
        let seps: Vec<char> = self
            .separators
            .iter()
            .filter_map(|s| s.chars().next())
            .collect();
        raw.split(|c| seps.contains(&c))
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .collect()
    }
}

/// Wrap a project's records in the standard envelope.
///
/// `source` identifies the emitting project and revision (e.g. `"myapp@a1b2c3d"`)
/// and should be stamped by the build, not typed by hand.
pub fn document(source: &str, records: &[Record]) -> Value {
    serde_json::json!({
        "contract_version": CONTRACT_VERSION,
        "source": source,
        "generated": now_iso(),
        "precedence": PRECEDENCE,
        "inputs": records,
    })
}

/// Problems that only exist across a whole model: two inputs claiming the same
/// identity, the same environment name, or the same flag.
///
/// A collision is silent at runtime — one binding simply shadows the other — so
/// it has to be caught here.
pub fn check_unique(records: &[Record]) -> Vec<String> {
    use std::collections::BTreeMap;
    let mut problems = Vec::new();

    let mut claim = |space: &str, what: String, by: &str, seen: &mut BTreeMap<String, String>| {
        if let Some(first) = seen.get(&what) {
            problems.push(format!(
                "{space} `{what}` is claimed by both `{first}` and `{by}`"
            ));
        } else {
            seen.insert(what, by.to_string());
        }
    };

    let mut keys = BTreeMap::new();
    let mut envs = BTreeMap::new();
    let mut args = BTreeMap::new();
    for r in records {
        claim("key", r.key.clone(), &r.key, &mut keys);
        for n in r.env_names() {
            claim("env name", n.to_string(), &r.key, &mut envs);
        }
        for l in r.arg_labels() {
            claim("flag", l, &r.key, &mut args);
        }
    }
    problems
}
