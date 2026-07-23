//! Vocabulary — the closed sets and structured scalars a declaration is built
//! from. No naked primitive ever stands for a domain-meaningful value.
use crate::Version;
use serde_json::Value;
use std::fmt;

/// The coarse wire *kind* a variable exposes in the contract.
///
/// This is the portable label other languages see; the Rust type parameter on
/// [`crate::Input`] is the real enforcement.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Type {
    /// Opaque text with no further structure.
    String,
    /// A two-state toggle (`1/0`, `true/false`, `on/off`).
    Bool,
    /// Exactly one token from `allowed`.
    Enum,
    /// A separated list of tokens from `allowed`.
    List,
    /// A non-negative integer.
    Uint,
    /// A signed integer.
    Int,
    /// A floating-point number.
    Float,
    /// A filesystem path.
    Path,
}

impl Type {
    /// The wire spelling used in the JSON contract.
    pub const fn as_str(self) -> &'static str {
        match self {
            Type::String => "string",
            Type::Bool => "bool",
            Type::Enum => "enum",
            Type::List => "list",
            Type::Uint => "uint",
            Type::Int => "int",
            Type::Float => "float",
            Type::Path => "path",
        }
    }
    /// Every wire spelling, in declaration order — the schema's `enum` list.
    pub const ALL: &'static [&'static str] = &[
        "string", "bool", "enum", "list", "uint", "int", "float", "path",
    ];
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How much a consumer may rely on a variable.
///
/// `Deprecated` must be paired with a [`Deprecation`]; [`crate::Input::check`]
/// enforces that, so an entry can never be half-deprecated.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stability {
    /// Not yet classified. The honest default for a freshly harvested name.
    Unknown,
    /// Safe to depend on; changes follow the project's compatibility policy.
    Stable,
    /// May change or vanish without notice.
    Experimental,
    /// Diagnostic knob. Present in shipped builds, but not a supported interface.
    Debug,
    /// Scheduled for removal; see [`Deprecation`] for the migration path.
    Deprecated,
}

impl Stability {
    /// The wire spelling used in the JSON contract.
    pub const fn as_str(self) -> &'static str {
        match self {
            Stability::Unknown => "unknown",
            Stability::Stable => "stable",
            Stability::Experimental => "experimental",
            Stability::Debug => "debug",
            Stability::Deprecated => "deprecated",
        }
    }
    /// Every wire spelling — the schema's `enum` list.
    pub const ALL: &'static [&'static str] =
        &["unknown", "stable", "experimental", "debug", "deprecated"];
}

impl fmt::Display for Stability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// When a variable was introduced — never a bare string.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Since {
    /// Not yet classified.
    Unknown,
    /// Introduced in the version currently being built ([`crate::THIS_VERSION`]).
    /// Use this for a variable you are adding right now: it can never go stale.
    This,
    /// Introduced in an explicit, parsed, comparable version.
    At(Version),
}

impl Since {
    /// Resolve to a concrete version, if known.
    pub fn resolve(self) -> Option<Version> {
        match self {
            Since::Unknown => None,
            Since::This => Some(crate::THIS_VERSION),
            Since::At(v) => Some(v),
        }
    }
    pub(crate) fn to_json(self) -> Option<String> {
        self.resolve().map(|v| v.to_string())
    }
}

/// A reference to a key in another configuration surface that this variable
/// bridges to (for example a `.conf` key the variable overrides).
///
/// Format-validated: must be namespaced (`section.key`). The target lives in a
/// different system, so only the *shape* can be checked here — this is the one
/// deliberately-weak reference in the model, and it is typed rather than a bare
/// string so it cannot be confused with prose.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ConfigKeyRef(pub &'static str);

impl ConfigKeyRef {
    /// Validate and wrap a namespaced config key.
    ///
    /// # Panics
    /// At **compile time** if the key contains no `.` separator.
    pub const fn new(s: &'static str) -> ConfigKeyRef {
        let b = s.as_bytes();
        let mut i = 0usize;
        let mut dot = false;
        while i < b.len() {
            if b[i] == b'.' {
                dot = true;
            }
            i += 1;
        }
        if !dot {
            panic!("config key must be namespaced, e.g. \"section.key\"");
        }
        ConfigKeyRef(s)
    }
}

impl fmt::Display for ConfigKeyRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// Why a variable is going away and what to do instead.
///
/// Bundled into one struct so an entry cannot be half-deprecated.
#[derive(Clone, Copy, Debug)]
pub struct Deprecation {
    /// The version in which the variable became deprecated.
    pub since: Version,
    /// A **compile-checked** reference to the successor, when there is a clean
    /// 1:1 replacement. Store a function that reads the successor's `key`
    /// (`Some(|| Model::NEW_INPUT.key)`): rename or delete the successor and
    /// this stops compiling, so the pointer can never dangle.
    pub replaced_by: Option<fn() -> &'static str>,
    /// Human guidance when there is no clean successor — a config key to use, a
    /// "simply remove it", a split into two variables. Advisory prose: a machine
    /// must not try to apply it automatically.
    pub migration: &'static str,
}

impl Deprecation {
    pub(crate) fn to_json(self) -> Value {
        serde_json::json!({
            "since": self.since.to_string(),
            "replaced_by": self.replaced_by.map(|f| f()),
            "migration": if self.migration.is_empty() { None } else { Some(self.migration) },
        })
    }
}
