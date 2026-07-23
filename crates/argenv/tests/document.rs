//! The portable projection: what leaves the program, and what can come back in.
mod common;
use argenv::*;
use common::*;

#[test]
fn a_declaration_projects_without_losing_what_it_declared() {
    let r = LEVEL.to_record();
    assert_eq!(r.key, "log_level");
    assert_eq!(r.ty, "enum");
    assert_eq!(r.default, Some(serde_json::json!("info")));
    assert_eq!(r.env.as_ref().unwrap().name, "APP_LOG_LEVEL");
    let arg = r.arg.as_ref().unwrap();
    assert_eq!(arg.long.as_deref(), Some("log-level"));
    assert_eq!(arg.short.as_deref(), Some("l"));
    assert_eq!(arg.arity, 1);
    assert_eq!(arg.value_name.as_deref(), Some("LEVEL"));
}

#[test]
fn a_binding_the_input_does_not_have_is_absent_not_empty() {
    let r = LOG_PATH.to_record();
    assert!(r.arg.is_none(), "an env-only input declares no arg binding");
    let json = serde_json::to_value(&r).unwrap();
    assert!(json.get("arg").is_none());
}

#[test]
fn unknown_facts_are_omitted_rather_than_nulled() {
    let json = serde_json::to_value(LOG_PATH.to_record()).unwrap();
    assert!(json.get("summary").is_none());
    assert!(json.get("reviewed").is_none());
    assert_eq!(
        json["stability"], "unknown",
        "an unclassified entry says so"
    );
}

#[test]
fn a_record_round_trips_through_json() {
    for r in model() {
        let text = serde_json::to_string(&r).unwrap();
        assert_eq!(serde_json::from_str::<Record>(&text).unwrap(), r);
    }
}

#[test]
fn a_field_added_by_a_later_version_is_ignored_not_rejected() {
    let mut v = serde_json::to_value(HDR.to_record()).unwrap();
    v["a_field_from_v1_1"] = serde_json::json!("hello");
    serde_json::from_value::<Record>(v)
        .expect("a consumer pinned to this version must keep reading newer documents");
}

#[test]
fn the_envelope_carries_version_provenance_and_precedence() {
    let doc = document("app@a1b2c3d", &model());
    assert_eq!(doc["contract_version"], CONTRACT_VERSION);
    assert_eq!(doc["source"], "app@a1b2c3d");
    assert!(doc["generated"].as_str().unwrap().ends_with('Z'));
    assert_eq!(doc["precedence"], serde_json::json!(PRECEDENCE));
    assert_eq!(doc["inputs"].as_array().unwrap().len(), model().len());
}

#[test]
fn a_record_judges_a_value_from_its_metadata_alone() {
    assert!(LEVEL.to_record().accepts("warn").is_ok());
    assert!(LEVEL.to_record().accepts("loud").is_err());
    assert!(HDR.to_record().accepts("1").is_ok());
    assert!(HDR.to_record().accepts("perhaps").is_err());
}

#[test]
fn a_list_splits_on_every_declared_separator() {
    let hud = HUD.to_record();
    assert_eq!(
        hud.tokens("fps;gpuload,memory"),
        vec!["fps", "gpuload", "memory"]
    );
    assert!(HDR.to_record().tokens("a,b").is_empty(), "only lists split");
}

#[test]
fn a_record_renders_its_own_help_line() {
    assert_eq!(LEVEL.to_record().usage(), "-l, --log-level <LEVEL>");
    assert_eq!(HDR.to_record().usage(), "-H, --hdr");
    assert_eq!(VERBOSE.to_record().usage(), "-v");
    assert_eq!(
        LOG_PATH.to_record().usage(),
        "",
        "no argument form, no usage line"
    );
}

#[test]
fn a_record_lists_every_spelling_it_answers_to() {
    assert_eq!(
        HDR.to_record().arg_labels(),
        vec!["-H", "--hdr", "--no-hdr"]
    );
    assert_eq!(
        LOG_PATH.to_record().env_names(),
        vec!["APP_LOG_PATH", "APP_LOGFILE"]
    );
}

#[test]
fn versions_ignore_prerelease_labels_so_a_tagged_candidate_still_builds() {
    assert_eq!(Version::parse("0.2.0-rc1"), Version::parse("0.2.0"));
    assert_eq!(
        Version::try_parse("1.0.0+build7"),
        Some(Version::parse("1.0.0"))
    );
    assert_eq!(Version::try_parse("1.x"), None);
    assert_eq!(ReviewDate::try_parse("2026-13-01"), None);
}
