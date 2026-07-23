//! How a value is read: from which door, in what order, and with what provenance.
mod common;
use argenv::*;
use common::*;

fn resolve(argv: &[&str], vars: &[(&str, &str)]) -> Resolution {
    let a = args(argv);
    let e = env(vars);
    Invocation { args: &a, env: &e }.resolve(&model())
}

#[test]
fn a_value_arrives_typed_from_either_door() {
    let from_arg = resolve(&["--log-level", "warn"], &[]);
    assert_eq!(LEVEL.get_from(&from_arg), Some(LogLevel::Warn));

    let from_env = resolve(&[], &[("APP_LOG_LEVEL", "error")]);
    assert_eq!(LEVEL.get_from(&from_env), Some(LogLevel::Error));
}

#[test]
fn an_argument_beats_an_environment_variable_which_beats_the_default() {
    let both = resolve(&["--log-level", "warn"], &[("APP_LOG_LEVEL", "error")]);
    assert_eq!(LEVEL.get_from(&both), Some(LogLevel::Warn));
    assert_eq!(both.source("log_level"), Some(Source::Arg));

    let env_only = resolve(&[], &[("APP_LOG_LEVEL", "error")]);
    assert_eq!(env_only.source("log_level"), Some(Source::Env));

    let neither = resolve(&[], &[]);
    assert_eq!(neither.source("log_level"), Some(Source::Default));
    assert_eq!(PRECEDENCE, &["arg", "env", "default"]);
}

#[test]
fn every_value_says_where_it_came_from() {
    let r = resolve(&["--hdr"], &[("APP_LOG_LEVEL", "warn")]);
    assert_eq!(r.source("hdr"), Some(Source::Arg));
    assert_eq!(r.source("log_level"), Some(Source::Env));
    assert_eq!(
        r.source("hud"),
        None,
        "nothing supplied it and it has no default"
    );
}

#[test]
fn a_boolean_flag_is_true_by_its_presence_and_false_by_its_negation() {
    assert_eq!(HDR.get_from(&resolve(&["--hdr"], &[])), Some(true));
    assert_eq!(HDR.get_from(&resolve(&["--no-hdr"], &[])), Some(false));
    assert_eq!(HDR.get_from(&resolve(&["-H"], &[])), Some(true));
}

#[test]
fn a_value_may_be_attached_or_separate() {
    for argv in [
        vec!["--log-level", "warn"],
        vec!["--log-level=warn"],
        vec!["-l", "warn"],
        vec!["-lwarn"],
    ] {
        assert_eq!(
            LEVEL.get_from(&resolve(&argv, &[])),
            Some(LogLevel::Warn),
            "{argv:?} should parse"
        );
    }
}

#[test]
fn short_boolean_flags_bundle() {
    let r = resolve(&["-vH"], &[]);
    assert_eq!(VERBOSE.get_from(&r), Some(true));
    assert_eq!(HDR.get_from(&r), Some(true));
}

#[test]
fn a_repeatable_flag_accumulates() {
    let r = resolve(&["--hud", "fps", "--hud", "gpuload"], &[]);
    assert_eq!(r.raw("hud"), Some("fps,gpuload"));
}

#[test]
fn a_non_repeatable_flag_keeps_the_last_occurrence() {
    let r = resolve(&["--log-level", "warn", "--log-level", "error"], &[]);
    assert_eq!(LEVEL.get_from(&r), Some(LogLevel::Error));
}

#[test]
fn everything_after_a_double_dash_is_positional() {
    let r = resolve(&["--hdr", "--", "--log-level", "warn"], &[]);
    assert_eq!(HDR.get_from(&r), Some(true));
    assert_eq!(
        r.positionals(),
        &["--log-level".to_string(), "warn".to_string()]
    );
    assert_eq!(
        r.source("log_level"),
        Some(Source::Default),
        "not parsed as a flag"
    );
}

#[test]
fn absent_and_invalid_are_both_none() {
    assert_eq!(
        LEVEL.get_from(&resolve(&[], &[])),
        Some(LogLevel::Info),
        "the default"
    );
    assert_eq!(
        HDR.get_from(&resolve(&[], &[("APP_HDR", "perhaps")])),
        None,
        "invalid must never masquerade as a value"
    );
}

#[test]
fn a_legacy_environment_name_is_still_honoured() {
    let r = resolve(&[], &[("APP_LOGFILE", "/tmp/app.log")]);
    assert_eq!(r.raw("log_path"), Some("/tmp/app.log"));
}

#[test]
fn an_environment_alone_resolves_for_programs_that_take_no_arguments() {
    let e = env(&[("APP_LOG_LEVEL", "warn")]);
    let r = Invocation::from_env(&e).resolve(&model());
    assert_eq!(LEVEL.get_from(&r), Some(LogLevel::Warn));
    assert_eq!(LEVEL.get_in(&e), Some(LogLevel::Warn), "or read directly");
}

/// A source that is neither this process nor a map: a snapshot in the
/// NUL-separated form the kernel exposes at `/proc/<pid>/environ`.
struct ProcEnviron(&'static [u8]);
impl EnvSource for ProcEnviron {
    fn get(&self, name: &str) -> Option<String> {
        self.pairs().find(|(k, _)| k == name).map(|(_, v)| v)
    }
    fn names(&self) -> Vec<String> {
        self.pairs().map(|(k, _)| k).collect()
    }
}
impl ProcEnviron {
    fn pairs(&self) -> impl Iterator<Item = (String, String)> + '_ {
        self.0
            .split(|b| *b == 0)
            .filter(|s| !s.is_empty())
            .filter_map(|s| std::str::from_utf8(s).ok())
            .filter_map(|s| {
                s.split_once('=')
                    .map(|(k, v)| (k.to_string(), v.to_string()))
            })
    }
}

#[test]
fn a_foreign_snapshot_is_a_source_like_any_other() {
    let snapshot = ProcEnviron(b"PATH=/usr/bin\0APP_LOG_LEVEL=warn\0");
    assert_eq!(LEVEL.get_in(&snapshot), Some(LogLevel::Warn));
}
