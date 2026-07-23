//! The whole life of a contract, in one runnable story.
//!
//! Read this file to understand the project: a program declares its invocation
//! surface, publishes it, and a completely separate consumer — with no access to
//! the Rust types — uses that publication to catch a broken launch.
mod common;
use argenv::*;
use common::*;

#[test]
fn a_contract_is_declared_published_and_used_to_catch_a_broken_launch() {
    // ---- 1. A program declares its invocation surface. --------------------
    // The declarations are `const`: a misspelled key or a wrongly typed read
    // would have failed to compile, before this test ever ran.
    let surface = model();
    assert!(problems().is_empty(), "the surface is well formed");

    // ---- 2. It is published as one document. ------------------------------
    let published = serde_json::to_string_pretty(&document("app@a1b2c3d", &surface)).unwrap();
    assert!(published.contains("\"contract_version\": 1"));

    // ---- 3. A separate consumer reads it back. ----------------------------
    // No dependency on the program, and it could be written in any language:
    // everything it needs is in the document, including how many values each
    // flag takes.
    let doc: serde_json::Value = serde_json::from_str(&published).unwrap();
    let declared: Vec<Record> = serde_json::from_value(doc["inputs"].clone()).unwrap();
    assert_eq!(declared.len(), surface.len());

    // ---- 4. It assembles an invocation for a child process. ---------------
    // Six mistakes, every one silent in a world without a contract.
    let argv = args(&[
        "--log-levl",
        "warn", // a misspelled flag: never takes effect
        "--hud",
        "fps,bogus", // one bad token in a list
        "--profile", // deprecated
    ]);
    let environment = env(&[
        ("PATH", "/usr/bin"),     // unrelated: must be ignored
        ("APP_HDR", "perhaps"),   // not a boolean
        ("APP_LOG_LEVL", "info"), // a misspelled variable
    ]);
    // APP_KEY, which is required, is missing entirely.
    let invocation = Invocation {
        args: &argv,
        env: &environment,
    };

    // ---- 5. The contract catches all of it, before anything is launched. --
    let findings = lint(&declared, &invocation);
    let has = |p: fn(&Finding) -> bool| findings.iter().any(p);

    assert!(
        has(|f| matches!(f, Finding::UnknownArg { arg, .. } if arg == "--log-levl")),
        "the misspelled flag: {findings:#?}"
    );
    assert!(
        has(|f| matches!(f, Finding::UnknownEnv { name, .. } if name == "APP_LOG_LEVL")),
        "the misspelled variable"
    );
    assert!(
        has(|f| matches!(f, Finding::InvalidValue { key, .. } if key == "hdr")),
        "the value outside its domain"
    );
    assert!(
        has(|f| matches!(f, Finding::UnknownToken { token, .. } if token == "bogus")),
        "the unknown token"
    );
    assert!(
        has(|f| matches!(f, Finding::MissingRequired { key, .. } if key == "licence_key")),
        "the missing required input"
    );
    assert!(
        has(|f| matches!(f, Finding::Deprecated { key, .. } if key == "profile")),
        "the deprecated input"
    );
    assert!(
        !has(|f| matches!(f, Finding::UnknownEnv { name, .. } if name == "PATH")),
        "unrelated variables must be left alone"
    );

    // ---- 6. Severity separates "stop" from "worth knowing". ---------------
    let errors = findings
        .iter()
        .filter(|f| f.severity() == Severity::Error)
        .count();
    let warnings = findings
        .iter()
        .filter(|f| f.severity() == Severity::Warning)
        .count();
    assert_eq!(errors, 3, "invalid value, unknown token, missing required");
    assert_eq!(warnings, 3, "two typos and the deprecated input");

    // ---- 7. A corrected invocation resolves cleanly, and says where from. --
    let argv = args(&["--log-level", "warn"]);
    let environment = env(&[("APP_KEY", "abc"), ("APP_HDR", "1")]);
    let invocation = Invocation {
        args: &argv,
        env: &environment,
    };
    assert!(lint(&declared, &invocation)
        .iter()
        .all(|f| f.severity() != Severity::Error));

    let resolved = invocation.resolve(&declared);
    assert_eq!(LEVEL.get_from(&resolved), Some(LogLevel::Warn));
    assert_eq!(resolved.source("log_level"), Some(Source::Arg));
    assert_eq!(HDR.get_from(&resolved), Some(true));
    assert_eq!(resolved.source("hdr"), Some(Source::Env));

    // ---- 8. The same document renders the program's help. -----------------
    let usage: Vec<String> = declared
        .iter()
        .map(Record::usage)
        .filter(|u| !u.is_empty())
        .collect();
    assert!(usage.contains(&"-l, --log-level <LEVEL>".to_string()));
}
