//! What checking an invocation catches — and what it deliberately ignores.
mod common;
use argenv::*;
use common::*;

fn check(argv: &[&str], vars: &[(&str, &str)]) -> Vec<Finding> {
    let a = args(argv);
    let e = env(vars);
    lint(&model(), &Invocation { args: &a, env: &e })
}

fn errors(f: &[Finding]) -> Vec<&Finding> {
    f.iter()
        .filter(|f| f.severity() == Severity::Error)
        .collect()
}

#[test]
fn a_satisfied_invocation_reports_nothing() {
    let f = check(&["--log-level", "warn"], &[("APP_KEY", "abc")]);
    assert!(errors(&f).is_empty(), "{f:#?}");
}

#[test]
fn a_misspelled_flag_is_caught_with_a_suggestion() {
    let f = check(&["--log-levl", "warn"], &[("APP_KEY", "k")]);
    assert!(
        f.iter().any(|f| matches!(
            f, Finding::UnknownArg { did_you_mean: Some(s), .. } if s == "--log-level"
        )),
        "{f:#?}"
    );
}

#[test]
fn a_misspelled_variable_is_caught_with_a_suggestion() {
    let f = check(&[], &[("APP_LOG_LEVL", "warn"), ("APP_KEY", "k")]);
    assert!(
        f.iter().any(|f| matches!(
            f, Finding::UnknownEnv { did_you_mean, .. } if did_you_mean == "APP_LOG_LEVEL"
        )),
        "{f:#?}"
    );
}

#[test]
fn the_rest_of_the_environment_is_never_mentioned() {
    let f = check(
        &[],
        &[("PATH", "/usr/bin"), ("HOME", "/root"), ("APP_KEY", "k")],
    );
    assert!(
        !f.iter().any(|f| matches!(f, Finding::UnknownEnv { .. })),
        "a checker that shouts about PATH is one nobody runs: {f:#?}"
    );
}

#[test]
fn a_flag_that_needs_a_value_and_gets_none_is_an_error() {
    let f = check(&["--log-level"], &[("APP_KEY", "k")]);
    assert!(f.iter().any(|f| matches!(f, Finding::MissingValue { .. })));
}

#[test]
fn a_flag_that_takes_no_value_and_gets_one_is_an_error() {
    let f = check(&["--hdr=yes"], &[("APP_KEY", "k")]);
    assert!(f
        .iter()
        .any(|f| matches!(f, Finding::UnexpectedValue { .. })));
}

#[test]
fn a_value_outside_its_domain_is_an_error_that_names_the_door() {
    let f = check(&["--log-level", "loud"], &[("APP_KEY", "k")]);
    let bad = f
        .iter()
        .find(|f| matches!(f, Finding::InvalidValue { .. }))
        .expect("expected InvalidValue");
    assert_eq!(bad.severity(), Severity::Error);
    assert!(bad.to_string().contains("argument"), "{bad}");

    let from_env = check(&[], &[("APP_HDR", "perhaps"), ("APP_KEY", "k")]);
    let bad = from_env
        .iter()
        .find(|f| matches!(f, Finding::InvalidValue { .. }))
        .expect("expected InvalidValue");
    assert!(bad.to_string().contains("environment"), "{bad}");
}

#[test]
fn a_declared_default_is_never_judged() {
    let f = check(&[], &[("APP_KEY", "k")]);
    assert!(
        !f.iter().any(|f| matches!(f, Finding::InvalidValue { .. })),
        "the program's own default is its choice: {f:#?}"
    );
}

#[test]
fn an_unknown_token_names_the_offender_and_spares_the_rest() {
    let f = check(&["--hud", "fps,bogus,gpuload"], &[("APP_KEY", "k")]);
    assert!(f.iter().any(|f| matches!(
        f, Finding::UnknownToken { token, .. } if token == "bogus"
    )));
    assert!(f.iter().all(|f| !matches!(
        f, Finding::UnknownToken { token, .. } if token == "fps" || token == "gpuload"
    )));
}

#[test]
fn a_missing_required_input_says_how_it_could_have_been_supplied() {
    let f = check(&[], &[]);
    let missing = f
        .iter()
        .find(|f| matches!(f, Finding::MissingRequired { key, .. } if key == "licence_key"))
        .expect("expected MissingRequired");
    let text = missing.to_string();
    assert!(
        text.contains("--licence-key") && text.contains("APP_KEY"),
        "{text}"
    );
}

#[test]
fn a_deprecated_input_in_use_is_a_warning_that_names_its_successor() {
    let f = check(&["--profile"], &[("APP_KEY", "k")]);
    let dep = f
        .iter()
        .find(|f| matches!(f, Finding::Deprecated { .. }))
        .expect("expected Deprecated");
    assert_eq!(dep.severity(), Severity::Warning);
    assert!(dep.to_string().contains("hud"), "{dep}");
}

#[test]
fn a_legacy_name_is_recognised_rather_than_reported_as_a_mistake() {
    let f = check(&[], &[("APP_LOGFILE", "/tmp/a.log"), ("APP_KEY", "k")]);
    assert!(
        !f.iter().any(|f| matches!(f, Finding::UnknownEnv { .. })),
        "{f:#?}"
    );
}

#[test]
fn checking_works_from_a_published_document_alone() {
    // A consumer that never saw the Rust types, in any language, can do this.
    let json = serde_json::to_string(&document("app@abc", &model())).unwrap();
    let doc: serde_json::Value = serde_json::from_str(&json).unwrap();
    let declared: Vec<Record> = serde_json::from_value(doc["inputs"].clone()).unwrap();

    let a = args(&["--log-level", "loud"]);
    let e = env(&[("APP_KEY", "k")]);
    let f = lint(&declared, &Invocation { args: &a, env: &e });
    assert!(f.iter().any(|f| matches!(f, Finding::InvalidValue { .. })));
}

#[test]
fn an_environment_only_program_can_be_checked_without_arguments() {
    let e = env(&[("APP_HDR", "perhaps"), ("APP_KEY", "k")]);
    let f = lint_env(&model(), &e);
    assert!(f.iter().any(|f| matches!(f, Finding::InvalidValue { .. })));
}
