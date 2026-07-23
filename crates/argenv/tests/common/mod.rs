//! Shared fixtures: one small but complete invocation surface, so every test
//! file reads against the same example.
#![allow(dead_code)]

use argenv::*;
use std::collections::BTreeMap;

/// An environment to read from, without touching process-global state.
pub fn env(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Arguments, as a program would receive them (without the program name).
pub fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// Both doors, one setting.
pub const LEVEL: Input<LogLevel> = Input {
    key: "log_level",
    ty: Type::Enum,
    default: Some(LogLevel::Info),
    allowed: LogLevel::TOKENS,
    env: Some(Env::new("APP_LOG_LEVEL")),
    arg: Some(Arg {
        value_name: "LEVEL",
        ..Arg::pair("log-level", 'l')
    }),
    stability: Stability::Stable,
    since: Since::This,
    reviewed: Some(ReviewDate::parse("2020-01-01")),
    summary: "Log verbosity",
    ..Input::EMPTY
};

/// A boolean whose presence is the value, and which can be negated.
pub const HDR: Input<bool> = Input {
    key: "hdr",
    ty: Type::Bool,
    default: Some(false),
    env: Some(Env::new("APP_HDR")),
    arg: Some(Arg {
        negatable: true,
        ..Arg::pair("hdr", 'H')
    }),
    stability: Stability::Stable,
    since: Since::This,
    maps_to: Some(ConfigKeyRef::new("display.enableHDR")),
    observe: Some("hdr_enabled"),
    reviewed: Some(ReviewDate::parse("2020-01-01")),
    summary: "Expose HDR output",
    ..Input::EMPTY
};

/// A repeatable list.
pub const HUD: Input<String> = Input {
    key: "hud",
    ty: Type::List,
    allowed: &["fps", "gpuload", "memory"],
    separators: &[',', ';'],
    env: Some(Env::new("APP_HUD")),
    arg: Some(Arg {
        repeatable: true,
        value_name: "ELEMENT",
        ..Arg::long("hud")
    }),
    stability: Stability::Stable,
    since: Since::This,
    reviewed: Some(ReviewDate::parse("2020-01-01")),
    summary: "Overlay elements",
    ..Input::EMPTY
};

/// Environment only, with a legacy name still honoured.
pub const LOG_PATH: Input<std::path::PathBuf> = Input {
    key: "log_path",
    ty: Type::Path,
    env: Some(Env {
        name: "APP_LOG_PATH",
        aliases: &["APP_LOGFILE"],
    }),
    group: "logging",
    ..Input::EMPTY
};

/// A short-only boolean, for bundling.
pub const VERBOSE: Input<bool> = Input {
    key: "verbose",
    ty: Type::Bool,
    arg: Some(Arg::short('v')),
    stability: Stability::Stable,
    since: Since::This,
    reviewed: Some(ReviewDate::parse("2020-01-01")),
    summary: "Explain what is happening",
    ..Input::EMPTY
};

/// Required, with no default.
pub const KEY: Input<String> = Input {
    key: "licence_key",
    ty: Type::String,
    required: true,
    env: Some(Env::new("APP_KEY")),
    arg: Some(Arg {
        value_name: "KEY",
        ..Arg::long("licence-key")
    }),
    stability: Stability::Stable,
    since: Since::This,
    reviewed: Some(ReviewDate::parse("2020-01-01")),
    summary: "Licence key",
    ..Input::EMPTY
};

/// On its way out, pointing at its successor.
pub const PROFILE: Input<bool> = Input {
    key: "profile",
    ty: Type::Bool,
    env: Some(Env::new("APP_PROFILE")),
    arg: Some(Arg::long("profile")),
    stability: Stability::Deprecated,
    since: Since::This,
    deprecation: Some(Deprecation {
        since: THIS_VERSION,
        replaced_by: Some(|| HUD.key),
        migration: "use --hud gpuload instead",
    }),
    reviewed: Some(ReviewDate::parse("2020-01-01")),
    summary: "Deprecated profiling overlay",
    ..Input::EMPTY
};

/// The whole declared surface, as a program would publish it.
pub fn model() -> Vec<Record> {
    vec![
        LEVEL.to_record(),
        HDR.to_record(),
        HUD.to_record(),
        LOG_PATH.to_record(),
        VERBOSE.to_record(),
        KEY.to_record(),
        PROFILE.to_record(),
    ]
}

/// Every rule violation across the model.
pub fn problems() -> Vec<String> {
    let mut v = Vec::new();
    v.extend(LEVEL.check());
    v.extend(HDR.check());
    v.extend(HUD.check());
    v.extend(LOG_PATH.check());
    v.extend(VERBOSE.check());
    v.extend(KEY.check());
    v.extend(PROFILE.check());
    v.extend(check_unique(&model()));
    v
}
