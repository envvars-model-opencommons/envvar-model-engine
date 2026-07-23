//! Checking a real invocation against the declared contract.
//!
//! This is the failure mode the project exists to fix. Without a declaration, a
//! misspelled flag is an error the program never sees, a misspelled variable is
//! ignored in silence, and an out-of-domain value is accepted and then
//! misbehaves somewhere far from its cause.
use crate::{EnvSource, Invocation, Record, Resolution, Source};

/// How seriously to take a [`Finding`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Severity {
    /// Worth surfacing; safe to continue.
    Warning,
    /// The invocation does not satisfy the contract.
    Error,
}

/// Something wrong with an invocation, relative to the declared contract.
#[derive(Clone, Debug, PartialEq)]
pub enum Finding {
    /// A flag was given that no input declares.
    UnknownArg {
        /// The flag as written, dashes included.
        arg: String,
        /// The declared flag it most likely meant.
        did_you_mean: Option<String>,
    },
    /// A variable is set that no input declares, but which closely resembles one
    /// that exists.
    ///
    /// Only near-misses are reported: an environment holds hundreds of unrelated
    /// variables, and a checker that mentions `PATH` is a checker nobody runs.
    UnknownEnv {
        /// The variable that is set.
        name: String,
        /// The declared name it most likely meant.
        did_you_mean: String,
    },
    /// A flag that takes a value was given none.
    MissingValue {
        /// The flag as written.
        arg: String,
    },
    /// A flag that takes no value was given one.
    UnexpectedValue {
        /// The flag as written.
        arg: String,
    },
    /// A declared input received a value outside its domain.
    InvalidValue {
        /// The input's key.
        key: String,
        /// Where the offending value came from.
        source: Source,
        /// The offending raw value.
        value: String,
        /// Why it was rejected.
        reason: String,
    },
    /// A token in a list value is not among the declared tokens.
    UnknownToken {
        /// The input's key.
        key: String,
        /// The offending token.
        token: String,
        /// The tokens that are accepted.
        allowed: Vec<String>,
    },
    /// A required input was not supplied and has no default.
    MissingRequired {
        /// The input's key.
        key: String,
        /// How it could have been supplied.
        via: Vec<String>,
    },
    /// A deprecated input is in use.
    Deprecated {
        /// The input's key.
        key: String,
        /// Guidance carried by the declaration.
        guidance: String,
    },
}

impl Finding {
    /// How seriously to take this finding.
    pub fn severity(&self) -> Severity {
        match self {
            Finding::InvalidValue { .. }
            | Finding::UnknownToken { .. }
            | Finding::MissingRequired { .. }
            | Finding::MissingValue { .. }
            | Finding::UnexpectedValue { .. } => Severity::Error,
            Finding::UnknownArg { .. }
            | Finding::UnknownEnv { .. }
            | Finding::Deprecated { .. } => Severity::Warning,
        }
    }
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Finding::UnknownArg { arg, did_you_mean } => match did_you_mean {
                Some(s) => write!(f, "{arg} is not a declared flag; did you mean {s}?"),
                None => write!(f, "{arg} is not a declared flag"),
            },
            Finding::UnknownEnv { name, did_you_mean } => {
                write!(f, "{name} is not declared; did you mean {did_you_mean}?")
            }
            Finding::MissingValue { arg } => write!(f, "{arg} needs a value"),
            Finding::UnexpectedValue { arg } => write!(f, "{arg} takes no value"),
            Finding::InvalidValue {
                key,
                source,
                value,
                reason,
            } => write!(
                f,
                "{key} (from {source}) is `{value}`, which is invalid: {reason}"
            ),
            Finding::UnknownToken {
                key,
                token,
                allowed,
            } => write!(
                f,
                "{key} contains unknown token `{token}`; accepted: {}",
                allowed.join(", ")
            ),
            Finding::MissingRequired { key, via } => {
                if via.is_empty() {
                    write!(f, "{key} is required but was not supplied")
                } else {
                    write!(f, "{key} is required; supply it with {}", via.join(" or "))
                }
            }
            Finding::Deprecated { key, guidance } => {
                if guidance.is_empty() {
                    write!(f, "{key} is deprecated")
                } else {
                    write!(f, "{key} is deprecated: {guidance}")
                }
            }
        }
    }
}

/// Check an invocation against a declared contract.
///
/// Works from [`Record`]s, so it applies equally to a model declared in this
/// crate and to a contract document published by a program in another language.
pub fn lint(model: &[Record], invocation: &Invocation) -> Vec<Finding> {
    let resolution = invocation.resolve(model);
    let mut findings = resolution.findings().to_vec();
    findings.extend(check_resolution(model, &resolution));
    findings.extend(check_environment(model, invocation.env));
    findings
}

/// Check only what an environment supplies, for programs that take no arguments.
pub fn lint_env(model: &[Record], env: &dyn EnvSource) -> Vec<Finding> {
    lint(model, &Invocation::from_env(env))
}

fn check_resolution(model: &[Record], resolution: &Resolution) -> Vec<Finding> {
    let mut findings = Vec::new();
    for r in model {
        let Some(value) = resolution.values().get(&r.key) else {
            if r.required {
                let mut via = r.arg_labels();
                via.extend(r.env_names().iter().map(|n| n.to_string()));
                findings.push(Finding::MissingRequired {
                    key: r.key.clone(),
                    via,
                });
            }
            continue;
        };

        // A declared default is the program's own choice; only supplied values
        // are judged.
        if value.source == Source::Default {
            continue;
        }

        if let Err(reason) = r.accepts(&value.raw) {
            findings.push(Finding::InvalidValue {
                key: r.key.clone(),
                source: value.source,
                value: value.raw.clone(),
                reason,
            });
        }
        if !r.allowed.is_empty() {
            for token in r.tokens(&value.raw) {
                if !r.allowed.iter().any(|a| a == token) {
                    findings.push(Finding::UnknownToken {
                        key: r.key.clone(),
                        token: token.to_string(),
                        allowed: r.allowed.clone(),
                    });
                }
            }
        }
        if let Some(dep) = &r.deprecation {
            let guidance = dep
                .get("replaced_by")
                .and_then(|v| v.as_str())
                .map(|s| format!("use {s} instead"))
                .or_else(|| {
                    dep.get("migration")
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                })
                .unwrap_or_default();
            findings.push(Finding::Deprecated {
                key: r.key.clone(),
                guidance,
            });
        }
    }
    findings
}

fn check_environment(model: &[Record], env: &dyn EnvSource) -> Vec<Finding> {
    let declared: Vec<String> = model
        .iter()
        .flat_map(|r| r.env_names().into_iter().map(str::to_string))
        .collect();
    let prefixes: std::collections::BTreeSet<&str> = declared
        .iter()
        .filter_map(|n| n.split_once('_').map(|(p, _)| p))
        .collect();

    let mut findings = Vec::new();
    for name in env.names() {
        if declared.contains(&name) {
            continue;
        }
        // Only names plausibly aimed at this program are considered.
        let shares_prefix = name
            .split_once('_')
            .map(|(p, _)| prefixes.contains(p))
            .unwrap_or(false);
        if !shares_prefix {
            continue;
        }
        let known: Vec<&str> = declared.iter().map(String::as_str).collect();
        if let Some(best) = nearest(&name, &known) {
            findings.push(Finding::UnknownEnv {
                name,
                did_you_mean: best,
            });
        }
    }
    findings
}

/// The closest known spelling within a small edit distance, if any.
pub(crate) fn nearest<S: AsRef<str>>(name: &str, known: &[S]) -> Option<String> {
    // Roughly one typo per eight characters, at least one, at most three.
    let budget = (name.len() / 8).clamp(1, 3);
    known
        .iter()
        .map(|d| (edit_distance(name, d.as_ref()), d.as_ref()))
        .filter(|(dist, _)| *dist <= budget)
        .min_by_key(|(dist, _)| *dist)
        .map(|(_, d)| d.to_string())
}

/// Levenshtein distance, two-row variant.
fn edit_distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.is_empty() {
        return b.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}
