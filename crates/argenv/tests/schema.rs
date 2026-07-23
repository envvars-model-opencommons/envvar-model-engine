//! What the emitted JSON Schema promises.
#![cfg(feature = "contract")]
mod common;
use argenv::*;
use common::*;

fn schema() -> serde_json::Value {
    argenv::contract::json_schema()
}
fn input_props() -> serde_json::Value {
    schema()["properties"]["inputs"]["items"]["properties"].clone()
}

#[test]
fn every_field_of_a_record_is_present_and_documented() {
    let props = input_props();
    for field in [
        "key",
        "type",
        "default",
        "allowed",
        "separators",
        "required",
        "stability",
        "since",
        "deprecation",
        "group",
        "maps_to",
        "example",
        "observe",
        "reviewed",
        "summary",
        "modified",
        "source",
        "env",
        "arg",
    ] {
        assert!(props.get(field).is_some(), "schema is missing `{field}`");
        assert!(
            props[field]["description"].is_string(),
            "`{field}` undocumented"
        );
    }
}

#[test]
fn both_bindings_are_described_including_the_derived_arity() {
    let defs = schema()["definitions"].clone();
    for (ty, fields) in [
        ("EnvBinding", vec!["name", "aliases"]),
        (
            "ArgBinding",
            vec![
                "long",
                "short",
                "arity",
                "value_name",
                "negatable",
                "repeatable",
            ],
        ),
    ] {
        let props = &defs[ty]["properties"];
        for f in fields {
            assert!(props.get(f).is_some(), "{ty} is missing `{f}`");
            assert!(props[f]["description"].is_string(), "{ty}.{f} undocumented");
        }
    }
    assert!(
        defs["ArgBinding"]["properties"]["arity"]["description"]
            .as_str()
            .unwrap()
            .contains("re-derive"),
        "arity must explain why it is written out"
    );
}

#[test]
fn everything_a_real_record_emits_is_described() {
    let props = input_props();
    let defs = schema()["definitions"].clone();
    for r in model() {
        let emitted = serde_json::to_value(&r).unwrap();
        for key in emitted.as_object().unwrap().keys() {
            assert!(
                props.get(key).is_some(),
                "emitted field `{key}` is undescribed"
            );
        }
        if let Some(a) = &r.arg {
            for key in serde_json::to_value(a).unwrap().as_object().unwrap().keys() {
                assert!(
                    defs["ArgBinding"]["properties"].get(key).is_some(),
                    "emitted arg field `{key}` is undescribed"
                );
            }
        }
    }
}

#[test]
fn closed_sets_are_taken_from_the_rust_enums() {
    let props = input_props();
    assert_eq!(props["type"]["enum"], serde_json::json!(Type::ALL));
    assert_eq!(
        props["stability"]["enum"],
        serde_json::json!(Stability::ALL)
    );
    assert_eq!(
        schema()["properties"]["precedence"]["items"]["enum"],
        serde_json::json!(PRECEDENCE)
    );
}

#[test]
fn structured_scalars_carry_their_format() {
    let props = input_props();
    for field in ["key", "since", "reviewed", "modified"] {
        assert!(
            props[field]["pattern"].is_string(),
            "`{field}` needs a pattern"
        );
    }
    let defs = schema()["definitions"].clone();
    assert!(defs["EnvBinding"]["properties"]["name"]["pattern"].is_string());
    assert!(defs["ArgBinding"]["properties"]["long"]["pattern"].is_string());
}

#[test]
fn a_record_stays_open_to_fields_added_later() {
    assert_eq!(
        schema()["properties"]["inputs"]["items"]["additionalProperties"],
        serde_json::json!(true),
        "a format that refuses the unfamiliar cannot grow"
    );
}

#[test]
fn only_the_two_facts_that_are_always_known_are_required() {
    assert_eq!(
        schema()["properties"]["inputs"]["items"]["required"],
        serde_json::json!(["key", "type"])
    );
}

#[test]
fn the_envelope_is_identified_and_versioned() {
    let s = schema();
    assert_eq!(s["$schema"], "https://json-schema.org/draft/2020-12/schema");
    assert!(s["$id"].as_str().unwrap().contains("argenv"));
    assert_eq!(
        s["properties"]["contract_version"]["const"],
        CONTRACT_VERSION
    );
    assert_eq!(
        s["required"],
        serde_json::json!(["contract_version", "inputs"])
    );
}
