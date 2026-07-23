//! What makes a declaration well formed. Each test names one rule.
mod common;
use argenv::*;
use common::*;

#[test]
fn a_complete_surface_satisfies_every_rule() {
    let p = problems();
    assert!(p.is_empty(), "{p:#?}");
}

#[test]
fn a_key_a_type_and_one_binding_are_all_that_is_required() {
    const BARE: Input<String> = Input {
        key: "bare",
        ty: Type::String,
        env: Some(Env::new("APP_BARE")),
        ..Input::EMPTY
    };
    assert!(
        BARE.check().is_empty(),
        "a harvested name must be a valid entry"
    );
}

#[test]
fn an_input_nothing_can_set_is_rejected() {
    const UNREACHABLE: Input<String> = Input {
        key: "unreachable",
        ty: Type::String,
        ..Input::EMPTY
    };
    assert!(UNREACHABLE
        .check()
        .iter()
        .any(|e| e.contains("nothing can ever set it")));
}

#[test]
fn a_key_is_transport_free_snake_case() {
    for bad in ["LogLevel", "log-level", "9log", "log level", "APP_LOG"] {
        let v: Input<bool> = Input {
            key: bad,
            ty: Type::Bool,
            arg: Some(Arg::long("x")),
            ..Input::EMPTY
        };
        assert!(!v.check().is_empty(), "`{bad}` should be rejected as a key");
    }
}

#[test]
fn an_environment_name_must_be_settable_from_a_shell() {
    for bad in ["APP VAR", "APP-VAR", "9APP", "APP.VAR"] {
        let v: Input<bool> = Input {
            key: "k",
            ty: Type::Bool,
            env: Some(Env::new(bad)),
            ..Input::EMPTY
        };
        assert!(
            !v.check().is_empty(),
            "`{bad}` should be rejected as an env name"
        );
    }
}

#[test]
fn a_long_flag_is_kebab_case_and_carries_no_dashes() {
    for bad in ["--log-level", "LogLevel", "log_level", "9log"] {
        let v: Input<bool> = Input {
            key: "k",
            ty: Type::Bool,
            arg: Some(Arg::long(bad)),
            ..Input::EMPTY
        };
        assert!(
            !v.check().is_empty(),
            "`{bad}` should be rejected as a long flag"
        );
    }
}

#[test]
fn a_long_flag_may_not_start_with_no_because_that_is_a_negation() {
    const CLASH: Input<bool> = Input {
        key: "k",
        ty: Type::Bool,
        arg: Some(Arg::long("no-colour")),
        ..Input::EMPTY
    };
    assert!(CLASH.check().iter().any(|e| e.contains("collides")));
}

#[test]
fn an_arg_binding_needs_at_least_one_form() {
    const NEITHER: Input<bool> = Input {
        key: "k",
        ty: Type::Bool,
        arg: Some(Arg::EMPTY),
        ..Input::EMPTY
    };
    assert!(NEITHER
        .check()
        .iter()
        .any(|e| e.contains("neither a long nor a short")));
}

#[test]
fn negation_belongs_to_booleans_and_repetition_to_lists() {
    const BAD_NEG: Input<String> = Input {
        key: "k",
        ty: Type::String,
        arg: Some(Arg {
            negatable: true,
            ..Arg::long("x")
        }),
        ..Input::EMPTY
    };
    assert!(BAD_NEG.check().iter().any(|e| e.contains("negatable")));

    const BAD_REP: Input<bool> = Input {
        key: "k",
        ty: Type::Bool,
        arg: Some(Arg {
            repeatable: true,
            ..Arg::long("x")
        }),
        ..Input::EMPTY
    };
    assert!(BAD_REP.check().iter().any(|e| e.contains("repeatable")));
}

#[test]
fn a_boolean_flag_takes_no_value_so_it_names_none() {
    const POINTLESS: Input<bool> = Input {
        key: "k",
        ty: Type::Bool,
        arg: Some(Arg {
            value_name: "VALUE",
            ..Arg::long("x")
        }),
        ..Input::EMPTY
    };
    assert!(POINTLESS
        .check()
        .iter()
        .any(|e| e.contains("takes no value")));
}

#[test]
fn arity_follows_from_the_type_and_is_never_declared() {
    assert_eq!(HDR.arity(), 0, "a boolean flag's presence is the value");
    assert_eq!(LEVEL.arity(), 1);
    assert_eq!(
        HDR.to_record().arg.unwrap().arity,
        0,
        "and is written out explicitly"
    );
    assert_eq!(LEVEL.to_record().arg.unwrap().arity, 1);
}

#[test]
fn a_closed_set_declares_its_tokens_and_a_list_declares_its_separators() {
    const NO_TOKENS: Input<LogLevel> = Input {
        key: "k",
        ty: Type::Enum,
        arg: Some(Arg::long("x")),
        ..Input::EMPTY
    };
    assert!(NO_TOKENS
        .check()
        .iter()
        .any(|e| e.contains("`allowed` is empty")));

    const NO_SEP: Input<String> = Input {
        key: "k",
        ty: Type::List,
        allowed: &["a"],
        arg: Some(Arg::long("x")),
        ..Input::EMPTY
    };
    assert!(NO_SEP.check().iter().any(|e| e.contains("cannot be split")));
}

#[test]
fn a_version_cannot_be_newer_than_the_build_and_a_review_cannot_be_in_the_future() {
    const AHEAD: Input<bool> = Input {
        key: "k",
        ty: Type::Bool,
        arg: Some(Arg::long("x")),
        since: Since::At(Version::parse("999.0")),
        ..Input::EMPTY
    };
    assert!(AHEAD.check().iter().any(|e| e.contains("newer than")));

    const FUTURE: Input<bool> = Input {
        key: "k",
        ty: Type::Bool,
        arg: Some(Arg::long("x")),
        reviewed: Some(ReviewDate::parse("2999-01-01")),
        ..Input::EMPTY
    };
    assert!(FUTURE.check().iter().any(|e| e.contains("future")));
    assert!(
        ReviewDate::today().next_day() > ReviewDate::today(),
        "one day of slack"
    );
}

#[test]
fn deprecation_and_stability_must_agree() {
    const MARKED: Input<bool> = Input {
        key: "k",
        ty: Type::Bool,
        arg: Some(Arg::long("x")),
        stability: Stability::Deprecated,
        ..Input::EMPTY
    };
    assert!(MARKED
        .check()
        .iter()
        .any(|e| e.contains("no Deprecation details")));
}

#[test]
fn a_successor_reference_resolves_to_a_live_declaration() {
    let dep = PROFILE.deprecation.expect("declared deprecated");
    assert_eq!(dep.replaced_by.expect("has a successor")(), HUD.key);
}

#[test]
fn two_inputs_may_not_claim_the_same_key_name_or_flag() {
    let clash = |a: Record, b: Record| !check_unique(&[a, b]).is_empty();
    assert!(clash(LEVEL.to_record(), LEVEL.to_record()), "same key");

    const OTHER: Input<bool> = Input {
        key: "other",
        ty: Type::Bool,
        env: Some(Env::new("APP_HDR")),
        ..Input::EMPTY
    };
    assert!(clash(HDR.to_record(), OTHER.to_record()), "same env name");

    const SAME_FLAG: Input<bool> = Input {
        key: "another",
        ty: Type::Bool,
        arg: Some(Arg::long("hdr")),
        ..Input::EMPTY
    };
    assert!(clash(HDR.to_record(), SAME_FLAG.to_record()), "same flag");
}
