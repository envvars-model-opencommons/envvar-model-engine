//! The command-line tool, and the artifacts committed to this repository.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_argenv")
}
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}
fn run(args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .env_clear()
        .output()
        .expect("runs")
}
fn run_with_env(args: &[&str], vars: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(bin());
    cmd.args(args).env_clear();
    for (k, v) in vars {
        cmd.env(k, v);
    }
    cmd.output().expect("runs")
}
fn write_temp(name: &str, v: &serde_json::Value) -> PathBuf {
    let p = std::env::temp_dir().join(format!("argenv-test-{name}.json"));
    std::fs::write(&p, serde_json::to_vec_pretty(v).unwrap()).unwrap();
    p
}

fn valid_document() -> serde_json::Value {
    serde_json::json!({
        "contract_version": 1,
        "source": "app@test",
        "precedence": ["arg", "env", "default"],
        "inputs": [
            { "key": "hdr", "type": "bool", "stability": "stable",
              "env": { "name": "APP_HDR" },
              "arg": { "long": "hdr", "arity": 0, "negatable": true, "repeatable": false } },
            { "key": "log_level", "type": "enum", "stability": "stable",
              "allowed": ["info", "warn"], "since": "1.0", "reviewed": "2026-01-01",
              "env": { "name": "APP_LOG_LEVEL" },
              "arg": { "long": "log-level", "short": "l", "arity": 1,
                       "value_name": "LEVEL", "negatable": false, "repeatable": false } }
        ]
    })
}

#[test]
fn schema_prints_the_contract() {
    let out = run(&["schema"]);
    assert!(out.status.success());
    let s: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(s["$schema"], "https://json-schema.org/draft/2020-12/schema");
    assert!(s["properties"]["inputs"]["items"]["properties"]["key"].is_object());
    assert!(
        s["definitions"]["ArgBinding"].is_object(),
        "bindings are described"
    );
}

#[test]
fn help_and_version_succeed_and_an_unknown_command_does_not() {
    for flag in ["--help", "-h", "--version", "-V"] {
        assert!(run(&[flag]).status.success(), "{flag}");
    }
    assert_eq!(run(&["frobnicate"]).status.code(), Some(2));
}

#[test]
fn the_committed_schema_is_the_one_this_model_produces() {
    let committed =
        std::fs::read_to_string(repo().join("schema/argenv-contract.v1.schema.json")).unwrap();
    let fresh = String::from_utf8(run(&["schema"]).stdout).unwrap();
    assert_eq!(
        committed.trim(),
        fresh.trim(),
        "the committed schema is stale: regenerate it with `argenv schema -o …`"
    );
}

#[test]
fn the_published_api_copy_matches_the_committed_schema() {
    let root = repo();
    let a = std::fs::read_to_string(root.join("schema/argenv-contract.v1.schema.json")).unwrap();
    let b = std::fs::read_to_string(root.join("api/v1/contract.schema.json")).unwrap();
    assert_eq!(
        a.trim(),
        b.trim(),
        "api/v1/contract.schema.json is out of date"
    );
}

#[test]
fn the_published_example_is_a_valid_document() {
    let example = repo().join("api/v1/example.json");
    let out = run(&["check", example.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn check_accepts_a_wellformed_document() {
    let p = write_temp("ok", &valid_document());
    let out = run(&["check", p.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("2 inputs"));
}

/// A named way of corrupting a document, so each case reads as a sentence.
type Mutation = (&'static str, fn(&mut serde_json::Value));

#[test]
fn check_rejects_every_kind_of_malformed_entry() {
    let cases: &[Mutation] = &[
        ("unknown type", |d| {
            d["inputs"][0]["type"] = serde_json::json!("nonsense")
        }),
        ("unknown stability", |d| {
            d["inputs"][0]["stability"] = serde_json::json!("sorta")
        }),
        ("malformed version", |d| {
            d["inputs"][1]["since"] = serde_json::json!("1.x")
        }),
        ("malformed date", |d| {
            d["inputs"][1]["reviewed"] = serde_json::json!("2026-13-45")
        }),
        ("missing key", |d| {
            d["inputs"][0].as_object_mut().unwrap().remove("key");
        }),
        ("enum without tokens", |d| {
            d["inputs"][1]["allowed"] = serde_json::json!([])
        }),
        ("deprecated without details", |d| {
            d["inputs"][0]["stability"] = serde_json::json!("deprecated")
        }),
        ("wrong contract version", |d| {
            d["contract_version"] = serde_json::json!(99)
        }),
        ("no binding at all", |d| {
            let o = d["inputs"][0].as_object_mut().unwrap();
            o.remove("env");
            o.remove("arg");
        }),
        ("arg without arity", |d| {
            d["inputs"][0]["arg"]
                .as_object_mut()
                .unwrap()
                .remove("arity");
        }),
        ("impossible arity", |d| {
            d["inputs"][0]["arg"]["arity"] = serde_json::json!(7)
        }),
        ("flag claimed twice", |d| {
            d["inputs"][1]["arg"]["long"] = serde_json::json!("hdr")
        }),
        ("variable claimed twice", |d| {
            d["inputs"][1]["env"]["name"] = serde_json::json!("APP_HDR")
        }),
    ];
    for (label, mutate) in cases {
        let mut doc = valid_document();
        mutate(&mut doc);
        let p = write_temp("bad", &doc);
        let out = run(&["check", p.to_str().unwrap()]);
        assert!(!out.status.success(), "`{label}` should have been rejected");
    }
}

#[test]
fn check_flags_an_unrecognised_field_so_a_typo_is_visible() {
    let mut doc = valid_document();
    doc["inputs"][0]["sumary"] = serde_json::json!("misspelled");
    let p = write_temp("typo-field", &doc);
    let out = run(&["check", p.to_str().unwrap()]);
    assert!(String::from_utf8_lossy(&out.stderr).contains("unrecognised field"));
}

#[test]
fn lint_checks_arguments_and_environment_together() {
    let p = write_temp("lint", &valid_document());
    let path = p.to_str().unwrap();

    let ok = run_with_env(&["lint", path, "--", "--log-level", "warn"], &[]);
    assert!(
        ok.status.success(),
        "{}",
        String::from_utf8_lossy(&ok.stderr)
    );

    let bad_arg = run_with_env(&["lint", path, "--", "--log-level", "loud"], &[]);
    assert!(
        !bad_arg.status.success(),
        "an out-of-domain argument must fail"
    );

    let bad_env = run_with_env(&["lint", path], &[("APP_HDR", "perhaps")]);
    assert!(
        !bad_env.status.success(),
        "an out-of-domain variable must fail"
    );
}

#[test]
fn lint_warns_about_typos_in_either_surface_without_failing() {
    let p = write_temp("lint-typo", &valid_document());
    let path = p.to_str().unwrap();

    let flag = run_with_env(&["lint", path, "--", "--log-levl", "warn"], &[]);
    assert!(String::from_utf8_lossy(&flag.stderr).contains("did you mean --log-level"));
    assert!(
        flag.status.success(),
        "a suspected typo is reported, not fatal"
    );

    let var = run_with_env(&["lint", path], &[("APP_LOG_LEVL", "warn")]);
    assert!(String::from_utf8_lossy(&var.stderr).contains("did you mean APP_LOG_LEVEL"));
}

#[test]
fn lint_ignores_the_rest_of_the_environment() {
    let p = write_temp("lint-noise", &valid_document());
    let out = run_with_env(
        &["lint", p.to_str().unwrap()],
        &[("PATH", "/usr/bin"), ("HOME", "/root")],
    );
    assert!(out.status.success());
    assert!(!String::from_utf8_lossy(&out.stderr).contains("PATH"));
}

#[test]
fn usage_renders_help_from_the_contract() {
    let p = write_temp("usage", &valid_document());
    let out = run(&["usage", p.to_str().unwrap()]);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("-l, --log-level <LEVEL>"), "{text}");
    assert!(text.contains("--hdr"), "{text}");
}
