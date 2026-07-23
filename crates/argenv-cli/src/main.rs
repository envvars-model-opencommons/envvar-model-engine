//! `argenv` — emit and check ArgEnv contract documents.
//!
//! ```text
//! argenv schema [-o PATH]        emit the JSON Schema contract
//! argenv check PATH...           validate contract documents
//! argenv lint PATH [-- ARGS...]  check this invocation against a contract
//! argenv usage PATH              render the help text a contract implies
//! ```
use argenv::{contract, lint, Invocation, ProcessEnv, Record, Severity};
use serde_json::Value;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("schema") => cmd_schema(&args[1..]),
        Some("check") => cmd_check(&args[1..]),
        Some("lint") => cmd_lint(&args[1..]),
        Some("usage") => cmd_usage(&args[1..]),
        Some("--version") | Some("-V") => {
            println!("argenv {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--help") | Some("-h") | None => {
            print_help();
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("argenv: unknown command `{other}`");
            print_help();
            ExitCode::from(2)
        }
    }
}

fn print_help() {
    println!(
        "argenv {}\n\
         \n\
         USAGE\n    \
             argenv schema [-o PATH]         emit the JSON Schema contract\n    \
             argenv check PATH...            validate contract documents\n    \
             argenv lint PATH [-- ARGS...]   check this invocation against a contract\n    \
             argenv usage PATH               render the help text a contract implies\n    \
             argenv --version | --help\n",
        env!("CARGO_PKG_VERSION")
    );
}

fn cmd_schema(args: &[String]) -> ExitCode {
    let text = serde_json::to_string_pretty(&contract::json_schema()).expect("schema serialises");
    match args {
        [] => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        [flag, path] if flag == "-o" || flag == "--output" => {
            if let Some(dir) = std::path::Path::new(path).parent() {
                if !dir.as_os_str().is_empty() {
                    let _ = std::fs::create_dir_all(dir);
                }
            }
            match std::fs::write(path, format!("{text}\n")) {
                Ok(()) => {
                    eprintln!("wrote {path}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("argenv: cannot write {path}: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("argenv: usage: schema [-o PATH]");
            ExitCode::from(2)
        }
    }
}

/// Read a contract document and return its records, stamped with provenance.
fn read_document(path: &str) -> Result<Vec<Record>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: cannot read: {e}"))?;
    let doc: Value =
        serde_json::from_str(&text).map_err(|e| format!("{path}: invalid JSON: {e}"))?;
    let source = doc
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or(path)
        .to_string();
    let inputs = doc
        .get("inputs")
        .cloned()
        .ok_or_else(|| format!("{path}: missing `inputs`"))?;
    let records: Vec<Record> =
        serde_json::from_value(inputs).map_err(|e| format!("{path}: cannot read inputs: {e}"))?;
    Ok(records
        .into_iter()
        .map(|mut r| {
            r.source.get_or_insert_with(|| source.clone());
            r
        })
        .collect())
}

fn cmd_check(paths: &[String]) -> ExitCode {
    if paths.is_empty() {
        eprintln!("argenv: usage: check PATH...");
        return ExitCode::from(2);
    }
    let mut failed = false;
    for path in paths {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{path}: cannot read: {e}");
                failed = true;
                continue;
            }
        };
        let doc: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{path}: invalid JSON: {e}");
                failed = true;
                continue;
            }
        };
        let problems = validate_document(&doc);
        if problems.is_empty() {
            let n = doc
                .get("inputs")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            println!("{path}: ok ({n} inputs)");
        } else {
            for p in &problems {
                eprintln!("{path}: {p}");
            }
            failed = true;
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Check the current invocation against a published contract.
fn cmd_lint(args: &[String]) -> ExitCode {
    let (paths, argv) = match args.iter().position(|a| a == "--") {
        Some(i) => (&args[..i], args[i + 1..].to_vec()),
        None => (args, Vec::new()),
    };
    let Some(path) = paths.first() else {
        eprintln!("argenv: usage: lint PATH [-- ARGS...]");
        return ExitCode::from(2);
    };
    let records = match read_document(path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let env = ProcessEnv;
    let findings = lint(
        &records,
        &Invocation {
            args: &argv,
            env: &env,
        },
    );
    if findings.is_empty() {
        println!("invocation satisfies {} declared inputs", records.len());
        return ExitCode::SUCCESS;
    }
    let mut errors = 0usize;
    for f in &findings {
        match f.severity() {
            Severity::Error => {
                errors += 1;
                eprintln!("error: {f}");
            }
            Severity::Warning => eprintln!("warning: {f}"),
        }
    }
    if errors > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Render the help text a contract implies, so documentation has one source.
fn cmd_usage(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("argenv: usage: usage PATH");
        return ExitCode::from(2);
    };
    let records = match read_document(path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let mut groups: std::collections::BTreeMap<String, Vec<&Record>> = Default::default();
    for r in &records {
        groups
            .entry(r.group.clone().unwrap_or_else(|| "options".into()))
            .or_default()
            .push(r);
    }
    for (group, records) in groups {
        println!("\n{}", group.to_uppercase());
        for r in records {
            let usage = r.usage();
            let summary = r.summary.clone().unwrap_or_default();
            if usage.is_empty() {
                let names = r.env_names().join(", ");
                println!("    {names:<30} {summary}");
            } else {
                println!("    {usage:<30} {summary}");
            }
        }
    }
    ExitCode::SUCCESS
}

/// Fields this version of the contract knows. Anything else is reported so an
/// authoring typo is visible, without making the document invalid: later
/// versions legitimately add fields.
const KNOWN_FIELDS: &[&str] = &[
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
];

fn validate_document(doc: &Value) -> Vec<String> {
    let mut errs = Vec::new();

    match doc.get("contract_version").and_then(Value::as_u64) {
        Some(v) if v as u32 == argenv::CONTRACT_VERSION => {}
        Some(v) => errs.push(format!(
            "contract_version {v} is not supported (this tool speaks {})",
            argenv::CONTRACT_VERSION
        )),
        None => errs.push("missing contract_version".into()),
    }

    let Some(inputs) = doc.get("inputs").and_then(Value::as_array) else {
        errs.push("missing inputs array".into());
        return errs;
    };

    let mut keys = std::collections::BTreeSet::new();
    let mut flags = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();

    for (i, v) in inputs.iter().enumerate() {
        let key = v.get("key").and_then(Value::as_str).unwrap_or("");
        let at = if key.is_empty() {
            format!("inputs[{i}]")
        } else {
            key.to_string()
        };

        if key.is_empty() {
            errs.push(format!("{at}: missing or empty key"));
        } else if !keys.insert(key.to_string()) {
            errs.push(format!("{at}: duplicate key"));
        }

        if let Some(obj) = v.as_object() {
            for k in obj.keys() {
                if !KNOWN_FIELDS.contains(&k.as_str()) {
                    errs.push(format!(
                        "{at}: unrecognised field `{k}` (a typo, or a field from a later contract version)"
                    ));
                }
            }
        }

        let ty = v.get("type").and_then(Value::as_str).unwrap_or("");
        match ty {
            "" => errs.push(format!("{at}: missing type")),
            t if argenv::Type::ALL.contains(&t) => {}
            t => errs.push(format!("{at}: unknown type `{t}`")),
        }

        match v.get("stability").and_then(Value::as_str) {
            None => {}
            Some(s) if argenv::Stability::ALL.contains(&s) => {
                if s == "deprecated" && v.get("deprecation").is_none() {
                    errs.push(format!("{at}: deprecated but no deprecation details"));
                }
                if s != "deprecated" && v.get("deprecation").is_some() {
                    errs.push(format!(
                        "{at}: has deprecation details but stability is `{s}`"
                    ));
                }
            }
            Some(s) => errs.push(format!("{at}: unknown stability `{s}`")),
        }

        if let Some(s) = v.get("since").and_then(Value::as_str) {
            if argenv::Version::try_parse(s).is_none() {
                errs.push(format!("{at}: malformed since `{s}`"));
            }
        }
        for field in ["reviewed", "modified"] {
            if let Some(s) = v.get(field).and_then(Value::as_str) {
                if argenv::ReviewDate::try_parse(s).is_none() {
                    errs.push(format!("{at}: malformed {field} date `{s}`"));
                }
            }
        }

        let allowed = v
            .get("allowed")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        if matches!(ty, "enum" | "list") && allowed == 0 {
            errs.push(format!("{at}: type is `{ty}` but allowed is empty"));
        }
        if ty == "list"
            && v.get("separators")
                .and_then(Value::as_array)
                .map_or(0, Vec::len)
                == 0
        {
            errs.push(format!(
                "{at}: type is `list` but no separators are declared"
            ));
        }

        let has_env = v.get("env").is_some();
        let has_arg = v.get("arg").is_some();
        if !has_env && !has_arg {
            errs.push(format!("{at}: declares no binding, so nothing can set it"));
        }
        if let Some(e) = v
            .get("env")
            .and_then(|e| e.get("name"))
            .and_then(Value::as_str)
        {
            if !names.insert(e.to_string()) {
                errs.push(format!("{at}: environment name `{e}` is claimed twice"));
            }
        }
        if let Some(a) = v.get("arg") {
            if let Some(l) = a.get("long").and_then(Value::as_str) {
                if !flags.insert(format!("--{l}")) {
                    errs.push(format!("{at}: flag `--{l}` is claimed twice"));
                }
            }
            if let Some(s) = a.get("short").and_then(Value::as_str) {
                if !flags.insert(format!("-{s}")) {
                    errs.push(format!("{at}: flag `-{s}` is claimed twice"));
                }
            }
            match a.get("arity").and_then(Value::as_u64) {
                Some(0) | Some(1) => {}
                Some(n) => errs.push(format!("{at}: arity {n} is not 0 or 1")),
                None => errs.push(format!("{at}: arg binding is missing arity")),
            }
        }
    }
    errs
}
