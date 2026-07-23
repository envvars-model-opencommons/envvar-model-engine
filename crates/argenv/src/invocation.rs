//! Resolving an invocation: what the program was actually given, and from where.
use crate::{EnvSource, Finding, Record};
use std::collections::BTreeMap;

/// Which door a resolved value arrived through.
///
/// Carried on every resolved value because *"why is this value what it is?"* is
/// the question a precedence chain otherwise makes unanswerable.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    /// An explicit command-line argument.
    Arg,
    /// An environment variable.
    Env,
    /// The declared default; nothing supplied a value.
    Default,
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Source::Arg => "argument",
            Source::Env => "environment",
            Source::Default => "default",
        })
    }
}

/// One resolved value and its provenance.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    /// The raw, unparsed text.
    pub raw: String,
    /// Where it came from.
    pub source: Source,
}

/// What a program was invoked with: an argument vector and an environment.
pub struct Invocation<'a> {
    /// The arguments, **excluding** the program name.
    pub args: &'a [String],
    /// The environment.
    pub env: &'a dyn EnvSource,
}

impl<'a> Invocation<'a> {
    /// An invocation with no arguments, reading the given environment.
    pub fn from_env(env: &'a dyn EnvSource) -> Invocation<'a> {
        Invocation { args: &[], env }
    }

    /// Resolve every declared input against this invocation.
    pub fn resolve(&self, model: &[Record]) -> Resolution {
        let mut values: BTreeMap<String, Resolved> = BTreeMap::new();
        let mut findings = Vec::new();

        let parsed = parse_args(model, self.args, &mut findings);
        for (key, raw) in parsed.values {
            values.insert(
                key,
                Resolved {
                    raw,
                    source: Source::Arg,
                },
            );
        }

        for r in model {
            if values.contains_key(&r.key) {
                continue;
            }
            if let Some(raw) = r.env_names().iter().find_map(|n| self.env.get(n)) {
                values.insert(
                    r.key.clone(),
                    Resolved {
                        raw,
                        source: Source::Env,
                    },
                );
                continue;
            }
            if let Some(d) = &r.default {
                values.insert(
                    r.key.clone(),
                    Resolved {
                        raw: render_default(d),
                        source: Source::Default,
                    },
                );
            }
        }

        Resolution {
            values,
            positionals: parsed.positionals,
            findings,
        }
    }
}

/// Everything an invocation resolved to, with provenance and any problems found.
#[derive(Clone, Debug, Default)]
pub struct Resolution {
    values: BTreeMap<String, Resolved>,
    positionals: Vec<String>,
    findings: Vec<Finding>,
}

impl Resolution {
    /// The raw text resolved for `key`, if anything supplied one.
    pub fn raw(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|r| r.raw.as_str())
    }

    /// Where the value for `key` came from.
    pub fn source(&self, key: &str) -> Option<Source> {
        self.values.get(key).map(|r| r.source)
    }

    /// Every resolved value, by key.
    pub fn values(&self) -> &BTreeMap<String, Resolved> {
        &self.values
    }

    /// Arguments that were not flags, in order, plus anything after `--`.
    pub fn positionals(&self) -> &[String] {
        &self.positionals
    }

    /// Problems found while parsing the argument vector.
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }
}

struct Parsed {
    values: BTreeMap<String, String>,
    positionals: Vec<String>,
}

/// Parse an argument vector against a model.
///
/// Recognised forms — chosen to match long-standing convention rather than to
/// invent one:
///
/// * `--flag`, `--no-flag` (when negatable)
/// * `--opt value`, `--opt=value`
/// * `-f`, `-o value`, `-ovalue`
/// * `-abc`, bundling several no-value short flags
/// * `--`, after which everything is positional
///
/// A repeated flag accumulates when the input is a list, and otherwise the last
/// occurrence wins. Anything unrecognised becomes a [`Finding`] rather than
/// being dropped in silence.
fn parse_args(model: &[Record], args: &[String], findings: &mut Vec<Finding>) -> Parsed {
    let mut values: BTreeMap<String, String> = BTreeMap::new();
    let mut positionals = Vec::new();
    let mut i = 0usize;

    while i < args.len() {
        let a = &args[i];

        if a == "--" {
            positionals.extend(args[i + 1..].iter().cloned());
            break;
        }

        if let Some(body) = a.strip_prefix("--") {
            let (name, inline) = match body.split_once('=') {
                Some((n, v)) => (n, Some(v.to_string())),
                None => (body, None),
            };

            // Negation: --no-colour sets the boolean off.
            if let Some(base) = name.strip_prefix("no-") {
                if let Some(r) = find_long(model, base)
                    .filter(|r| r.arg.as_ref().map(|a| a.negatable).unwrap_or(false))
                {
                    values.insert(r.key.clone(), "false".to_string());
                    i += 1;
                    continue;
                }
            }

            match find_long(model, name) {
                Some(r) => {
                    let arity = r.arg.as_ref().map(|a| a.arity).unwrap_or(1);
                    if arity == 0 {
                        if inline.is_some() {
                            findings.push(Finding::UnexpectedValue {
                                arg: format!("--{name}"),
                            });
                        }
                        values.insert(r.key.clone(), "true".to_string());
                        i += 1;
                    } else {
                        match inline.or_else(|| args.get(i + 1).cloned()) {
                            Some(v) => {
                                accumulate(&mut values, r, v);
                                i += if a.contains('=') { 1 } else { 2 };
                            }
                            None => {
                                findings.push(Finding::MissingValue {
                                    arg: format!("--{name}"),
                                });
                                i += 1;
                            }
                        }
                    }
                }
                None => {
                    findings.push(unknown_arg(model, &format!("--{name}")));
                    i += 1;
                }
            }
            continue;
        }

        if a.len() > 1 && a.starts_with('-') {
            let body = &a[1..];
            let first = body.chars().next().expect("non-empty");
            match find_short(model, first) {
                Some(r) => {
                    let arity = r.arg.as_ref().map(|a| a.arity).unwrap_or(1);
                    let rest: String = body.chars().skip(1).collect();
                    if arity == 0 {
                        values.insert(r.key.clone(), "true".to_string());
                        if rest.is_empty() {
                            i += 1;
                        } else {
                            // Bundled shorts: -abc. Re-enter with the remainder.
                            let mut rebuilt = vec![format!("-{rest}")];
                            rebuilt.extend(args[i + 1..].iter().cloned());
                            let sub = parse_args(model, &rebuilt, findings);
                            values.extend(sub.values);
                            positionals.extend(sub.positionals);
                            break;
                        }
                    } else {
                        let value = if !rest.is_empty() {
                            Some(rest.trim_start_matches('=').to_string())
                        } else {
                            args.get(i + 1).cloned()
                        };
                        match value {
                            Some(v) => {
                                accumulate(&mut values, r, v);
                                i += if rest.is_empty() { 2 } else { 1 };
                            }
                            None => {
                                findings.push(Finding::MissingValue {
                                    arg: format!("-{first}"),
                                });
                                i += 1;
                            }
                        }
                    }
                }
                None => {
                    findings.push(unknown_arg(model, &format!("-{first}")));
                    i += 1;
                }
            }
            continue;
        }

        positionals.push(a.clone());
        i += 1;
    }

    Parsed {
        values,
        positionals,
    }
}

/// A repeated flag accumulates for a list input, and otherwise the last wins.
fn accumulate(values: &mut BTreeMap<String, String>, r: &Record, v: String) {
    let repeatable = r.arg.as_ref().map(|a| a.repeatable).unwrap_or(false);
    match values.get(&r.key) {
        Some(prev) if repeatable => {
            let sep = r
                .separators
                .first()
                .map(String::as_str)
                .unwrap_or(",")
                .to_string();
            let joined = format!("{prev}{sep}{v}");
            values.insert(r.key.clone(), joined);
        }
        _ => {
            values.insert(r.key.clone(), v);
        }
    }
}

fn find_long<'a>(model: &'a [Record], name: &str) -> Option<&'a Record> {
    model
        .iter()
        .find(|r| r.arg.as_ref().and_then(|a| a.long.as_deref()) == Some(name))
}

fn find_short(model: &[Record], c: char) -> Option<&Record> {
    model.iter().find(|r| {
        r.arg
            .as_ref()
            .and_then(|a| a.short.as_deref())
            .and_then(|s| s.chars().next())
            == Some(c)
    })
}

fn unknown_arg(model: &[Record], label: &str) -> Finding {
    let known: Vec<String> = model.iter().flat_map(Record::arg_labels).collect();
    Finding::UnknownArg {
        arg: label.to_string(),
        did_you_mean: crate::lint::nearest(label, &known),
    }
}

/// Render a declared default as the text an equivalent invocation would carry.
fn render_default(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}
