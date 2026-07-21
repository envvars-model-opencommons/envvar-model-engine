//! # opencommons-env-model — a typed contract for environment-variable interfaces
//!
//! A declared, machine-readable, cross-language contract for the env vars a program
//! exposes — "OpenAPI-style" (declared, generated, validated) but for the env-var
//! *interface* a program presents, not an HTTP protocol. Domain-neutral: nothing
//! here is Proton-specific (Proton is just the reference adopter).
//!
//! ## Read this file in two passes
//! * **§1 VOCABULARY** — the domain types (`Version`, `Type`, `Stability`, ...). You
//!   rarely edit these. Skim once, then ignore.
//! * **§2 THE DECLARATION** — [`EnvVar`]: ONE flat struct. A variable is a single
//!   flat literal that reads like a JSON object, top to bottom. This is the surface
//!   you actually author and read.
//!
//! The declaration IS the typed accessor: `EnvVar::<bool>::get()` returns a `bool`,
//! so the schema and the code that reads the var can never drift apart.
#![allow(dead_code)]

use serde::Serialize;
use serde_json::Value;
use std::fmt;

/// Bump on any breaking change to the wire [`Record`]. Additive changes (a new
/// optional field) keep the major and stay backward-compatible.
pub const CONTRACT_VERSION: u32 = 1;

// ###########################################################################
// §1  VOCABULARY  — domain types. No naked primitive ever stands for a
//     domain-meaningful value; illegal values are unconstructible.
// ###########################################################################

/// A project version (`major.minor.patch`). Constructed only via [`Version::parse`],
/// so a malformed literal is a **compile error** in a `const`. Ordered, so
/// `since <= THIS_VERSION` is a checkable invariant.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version { pub major: u16, pub minor: u16, pub patch: u16 }

impl Version {
    /// `const` parser. `Version::parse("2.x")` fails to compile, not at runtime.
    /// Note: only plain `major[.minor[.patch]]` — a pre-release suffix will panic.
    pub const fn parse(s: &str) -> Version {
        let b = s.as_bytes();
        let (mut n, mut idx, mut i) = ([0u16; 3], 0usize, 0usize);
        while i < b.len() {
            let c = b[i];
            if c == b'.' { idx += 1; if idx > 2 { panic!("too many version components"); } }
            else if c >= b'0' && c <= b'9' { n[idx] = n[idx] * 10 + (c - b'0') as u16; }
            else { panic!("invalid character in version"); }
            i += 1;
        }
        Version { major: n[0], minor: n[1], patch: n[2] }
    }
}
impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// The version of the build emitting the schema. Reference it via [`Since::This`]
/// for a var added right now, instead of hardcoding a number that would drift.
pub const THIS_VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION"));

// -- const digit helpers for date parsing --
const fn digit(b: u8) -> u16 { if b < b'0' || b > b'9' { panic!("non-digit in date"); } (b - b'0') as u16 }
const fn two(b: &[u8], i: usize) -> u16 { digit(b[i]) * 10 + digit(b[i + 1]) }
const fn four(b: &[u8], i: usize) -> u16 { two(b, i) * 100 + two(b, i + 2) }

/// A human review date (`YYYY-MM-DD`). Hand-written **by design** — it asserts
/// "a person verified this entry as of this date", which no tool can derive.
/// Const-validated; ordered, so `reviewed <= today` is checkable.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReviewDate { pub year: u16, pub month: u8, pub day: u8 }

impl ReviewDate {
    /// `const` parser; rejects malformed dates at compile time.
    pub const fn parse(s: &str) -> ReviewDate {
        let b = s.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' { panic!("date must be YYYY-MM-DD"); }
        let (year, month, day) = (four(b, 0), two(b, 5) as u8, two(b, 8) as u8);
        if month < 1 || month > 12 || day < 1 || day > 31 { panic!("date out of range"); }
        ReviewDate { year, month, day }
    }
    /// Today (UTC), from the system clock — used to reject future review dates.
    pub fn today() -> ReviewDate {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
        let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
        ReviewDate { year: y as u16, month: m as u8, day: d as u8 }
    }
}
impl fmt::Display for ReviewDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// The coarse wire *kind* a variable exposes in the contract. The rich Rust type
/// on [`EnvVar<T>`] is the real enforcement; this is the portable label.
#[derive(Clone, Copy)]
pub enum Type { String, Bool, Enum, Flags, Uint, Int, Float, Path }
impl Type {
    fn as_str(self) -> &'static str {
        match self {
            Type::String => "string", Type::Bool => "bool", Type::Enum => "enum",
            Type::Flags => "flags", Type::Uint => "uint", Type::Int => "int",
            Type::Float => "float", Type::Path => "path",
        }
    }
}

/// Lifecycle stability. `Deprecated` must be paired with a [`Deprecation`]
/// (enforced by [`validate`]).
#[derive(Clone, Copy, PartialEq)]
pub enum Stability { Unknown, Stable, Experimental, Debug, Deprecated }
impl Stability {
    fn as_str(self) -> &'static str {
        match self {
            Stability::Unknown => "unknown", Stability::Stable => "stable",
            Stability::Experimental => "experimental", Stability::Debug => "debug",
            Stability::Deprecated => "deprecated",
        }
    }
}

/// When a var was introduced — never a bare string.
/// * `This` resolves to [`THIS_VERSION`] (a var added now).
/// * `At(v)` is an explicit, parsed, comparable version (checked `<= THIS_VERSION`).
/// * `Unknown` = not yet classified.
#[derive(Clone, Copy)]
pub enum Since { Unknown, This, At(Version) }
impl Since {
    fn to_json(self) -> Value {
        match self {
            Since::Unknown => Value::Null,
            Since::This => Value::String(THIS_VERSION.to_string()),
            Since::At(v) => Value::String(v.to_string()),
        }
    }
}

/// A reference to a config key in another surface (e.g. a `dxvk.conf` key an env
/// var bridges to). Format-validated (must be namespaced `a.b`). It's an external
/// reference, so only its format is checked here, not that the target exists.
#[derive(Clone, Copy)]
pub struct ConfigKeyRef(pub &'static str);
impl ConfigKeyRef {
    pub const fn new(s: &'static str) -> ConfigKeyRef {
        let b = s.as_bytes();
        let (mut i, mut dot) = (0usize, false);
        while i < b.len() { if b[i] == b'.' { dot = true; } i += 1; }
        if !dot { panic!("config key must be namespaced, e.g. \"dxgi.enableHDR\""); }
        ConfigKeyRef(s)
    }
}

/// Deprecation details, bundled so an entry can't be half-deprecated.
/// `replaced_by` is a **compile-checked** pointer to the successor's name (a fn
/// reading another var's `name`; rename/remove the successor and it stops
/// compiling). `migration` is freeform guidance for the no-clean-successor case.
#[derive(Clone, Copy)]
pub struct Deprecation {
    /// Version in which the var became deprecated.
    pub since: Version,
    /// Compile-checked reference to the successor's name, when there is a 1:1 one.
    pub replaced_by: Option<fn() -> &'static str>,
    /// Human guidance when there's no clean successor (a config key, "just remove", ...).
    pub migration: &'static str,
}
impl Deprecation {
    fn to_json(&self) -> Value {
        serde_json::json!({
            "since": self.since.to_string(),
            "replaced_by": serde_json::to_value(self.replaced_by.map(|f| f())).unwrap(),
            "migration": opt_str(self.migration),
        })
    }
}

/// How a domain type reads itself from an env string. Implementing this on the
/// value type (like [`std::str::FromStr`]) means a var declaration carries **no
/// `parse` field** — parsing lives with the type that owns the domain.
pub trait FromEnv: Sized {
    fn from_env(s: &str) -> Option<Self>;
}

// -- plain types --
impl FromEnv for bool {
    fn from_env(s: &str) -> Option<bool> {
        match s.trim() { "1" | "true" | "on" => Some(true), "0" | "false" | "off" | "" => Some(false), _ => None }
    }
}
impl FromEnv for String { fn from_env(s: &str) -> Option<String> { Some(s.to_string()) } }
impl FromEnv for std::path::PathBuf {
    fn from_env(s: &str) -> Option<std::path::PathBuf> { if s.is_empty() { None } else { Some(s.into()) } }
}

// -- shared value domains (reuse these or add your own, following the same rule) --

/// Enum domain whose [`LogLevel::TOKENS`] are the single source for both parsing
/// and a var's `allowed` list — so they can't drift apart.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel { Trace, Debug, Info, Warn, Error, None }
impl LogLevel { pub const TOKENS: &'static [&'static str] = &["trace", "debug", "info", "warn", "error", "none"]; }
impl FromEnv for LogLevel {
    fn from_env(s: &str) -> Option<LogLevel> {
        Some(match s.trim() {
            "trace" => LogLevel::Trace, "debug" => LogLevel::Debug, "info" => LogLevel::Info,
            "warn" => LogLevel::Warn, "error" => LogLevel::Error, "none" => LogLevel::None, _ => return None,
        })
    }
}

/// A number with a meaningful sentinel: `0` means "uncapped", not "zero frames".
/// Modeled as an enum so the sentinel is explicit and arithmetic is impossible.
#[derive(Clone, Copy)]
pub enum FrameCap { Uncapped, Fps(u32) }
impl FromEnv for FrameCap {
    fn from_env(s: &str) -> Option<FrameCap> {
        let n: u32 = s.trim().parse().ok()?;
        Some(if n == 0 { FrameCap::Uncapped } else { FrameCap::Fps(n) })
    }
}
impl Serialize for FrameCap {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self { FrameCap::Uncapped => s.serialize_u32(0), FrameCap::Fps(n) => s.serialize_u32(*n) }
    }
}

/// A constrained float in `0.1..=1.0`. Out-of-range and `NaN` are unconstructible.
#[derive(Clone, Copy, Serialize)]
#[serde(transparent)]
pub struct RenderScale(f32);
impl RenderScale {
    pub const fn new(v: f32) -> RenderScale {
        assert!(v >= 0.1 && v <= 1.0, "render scale must be within 0.1..=1.0"); // NaN fails both -> panics
        RenderScale(v)
    }
    pub fn get(self) -> f32 { self.0 }
}
impl FromEnv for RenderScale {
    fn from_env(s: &str) -> Option<RenderScale> {
        let v: f32 = s.trim().parse().ok()?;
        (v >= 0.1 && v <= 1.0).then_some(RenderScale(v))
    }
}

/// A three-state toggle — a `bool` would be a lie here.
#[derive(Clone, Copy, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Tristate { #[default] Auto, On, Off }
impl Tristate { pub const TOKENS: &'static [&'static str] = &["auto", "on", "off"]; }
impl FromEnv for Tristate {
    fn from_env(s: &str) -> Option<Tristate> {
        Some(match s.trim() {
            "auto" | "" => Tristate::Auto, "on" | "1" | "true" => Tristate::On,
            "off" | "0" | "false" => Tristate::Off, _ => return None,
        })
    }
}

// ###########################################################################
// §2  THE DECLARATION  — one flat struct. A variable is a single flat literal
//     that reads top-to-bottom like a JSON object. Author with `..EnvVar::EMPTY`
//     so only the fields you know appear. Every field is hover-documented.
// ###########################################################################

pub struct EnvVar<T: 'static> {
    /// REQUIRED. The exact environment variable name, e.g. `"DXVK_HDR"`.
    pub name: &'static str,
    /// REQUIRED. Coarse wire kind. The Rust type `T` is the real enforcement.
    pub ty: Type,
    /// The default value when the var is unset — the real typed `T`, not a string.
    pub default: Option<T>,
    /// Accepted tokens for `Enum`/`Flags`. Wire to the type's `TOKENS`
    /// (e.g. `LogLevel::TOKENS`) so they never drift from parsing.
    pub allowed: &'static [&'static str],
    /// Whether the program requires this to be set.
    pub required: bool,
    /// Lifecycle stability.
    pub stability: Stability,
    /// The version this var was introduced in.
    pub since: Since,
    /// Present iff `stability == Deprecated`; how to migrate off it.
    pub deprecation: Option<Deprecation>,
    /// A grouping tag for docs/UX, e.g. `"logging"`, `"hdr"`. Free-form on purpose.
    pub group: &'static str,
    /// A config key in another surface this var bridges to, if any.
    pub maps_to: Option<ConfigKeyRef>,
    /// A copy-pasteable example, e.g. `"DXVK_HDR=1"`.
    pub example: &'static str,
    /// A resolved-config field this flips (for the validation harness), e.g.
    /// gamescope's `"bExposeHDRSupport"`. An external identifier -> opaque string.
    pub observe: Option<&'static str>,
    /// HAND-WRITTEN human verification date. `None` = nobody has vouched for it yet.
    pub reviewed: Option<ReviewDate>,
    /// Freeform prose (legitimately a string). What the variable does.
    pub summary: &'static str,
}

impl<T: 'static> EnvVar<T> {
    /// The empty baseline. Author a var by overriding only what's known:
    /// `EnvVar { name: "...", ty: ..., ..EnvVar::EMPTY }`.
    pub const EMPTY: EnvVar<T> = EnvVar {
        name: "", ty: Type::String, default: None, allowed: &[], required: false,
        stability: Stability::Unknown, since: Since::Unknown, deprecation: None,
        group: "", maps_to: None, example: "", observe: None, reviewed: None, summary: "",
    };

    /// Was the var explicitly set (regardless of validity)?
    pub fn is_set(&self) -> bool { std::env::var(self.name).is_ok() }

    /// Check this declaration's invariants; returns human-readable problems.
    pub fn check(&self) -> Vec<String> {
        let mut e = Vec::new();
        let today = ReviewDate::today();
        if self.name.is_empty() { e.push("a variable has an empty name".into()); }
        if let Since::At(v) = self.since {
            if v > THIS_VERSION { e.push(format!("{}: since {} > current {}", self.name, v, THIS_VERSION)); }
        }
        if let Some(r) = self.reviewed {
            if r > today { e.push(format!("{}: reviewed date {} is in the future", self.name, r)); }
        }
        match (self.stability, self.deprecation.is_some()) {
            (Stability::Deprecated, false) => e.push(format!("{}: Deprecated but no Deprecation info", self.name)),
            (s, true) if s != Stability::Deprecated => e.push(format!("{}: has Deprecation info but not Deprecated", self.name)),
            _ => {}
        }
        if let Some(d) = &self.deprecation {
            if d.since > THIS_VERSION { e.push(format!("{}: deprecated-since {} > current {}", self.name, d.since, THIS_VERSION)); }
        }
        e
    }
}

impl<T: 'static + FromEnv> EnvVar<T> {
    /// Read + parse. Absent OR invalid -> `None` (never a silent empty string).
    pub fn get(&self) -> Option<T> { std::env::var(self.name).ok().and_then(|s| T::from_env(&s)) }
    /// Read, with a caller-supplied fallback for the absent/invalid case.
    pub fn get_or(&self, fallback: T) -> T { self.get().unwrap_or(fallback) }
}
impl<T: 'static + FromEnv + Clone> EnvVar<T> {
    /// Read, or fall back to the declared [`EnvVar::default`].
    pub fn get_or_default(&self) -> Option<T> { self.get().or_else(|| self.default.clone()) }
}

impl<T: 'static + Serialize> EnvVar<T> {
    /// Render this rich declaration down to the portable contract [`Record`].
    pub fn to_record(&self) -> Record {
        Record {
            name: self.name.to_string(),
            ty: self.ty.as_str().to_string(),
            default: self.default.as_ref().map(|d| serde_json::to_value(d).unwrap_or(Value::Null)),
            allowed: self.allowed.iter().map(|s| s.to_string()).collect(),
            required: self.required,
            stability: self.stability.as_str().to_string(),
            since: match self.since.to_json() { Value::String(s) => Some(s), _ => None },
            deprecation: self.deprecation.as_ref().map(|d| d.to_json()),
            group: opt_string(self.group),
            maps_to: self.maps_to.map(|c| c.0.to_string()),
            example: opt_string(self.example),
            observe: self.observe.map(|s| s.to_string()),
            reviewed: self.reviewed.map(|d| d.to_string()),
            summary: opt_string(self.summary),
            modified: None, // AUTO (git blame on the declaration), stamped by the emitter
        }
    }
}

// ###########################################################################
// §3  THE WIRE CONTRACT  — `Record` is the portable projection every language
//     binding implements; the JSON-Schema of it is the cross-language "swagger".
// ###########################################################################

#[derive(Serialize)]
#[cfg_attr(feature = "contract", derive(schemars::JsonSchema))]
pub struct Record {
    pub name: String,
    #[serde(rename = "type")] pub ty: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub default: Option<Value>,
    pub allowed: Vec<String>,
    pub required: bool,
    pub stability: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub since: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub deprecation: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")] pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub maps_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub example: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub observe: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub reviewed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub modified: Option<String>,
}

/// The JSON-Schema contract for one record (requires `--features contract`).
#[cfg(feature = "contract")]
pub fn contract_schema() -> Value { serde_json::to_value(schemars::schema_for!(Record)).unwrap() }

/// Wrap a project's rendered records in the standard envelope (version + provenance).
pub fn document(source: &str, records: Vec<Record>) -> Value {
    serde_json::json!({
        "contract_version": CONTRACT_VERSION,
        "source": source,            // e.g. "dxvk@0ff9cd3" — stamped by the emitter
        "generated": now_iso(),      // AUTO
        "vars": serde_json::to_value(records).unwrap(),
    })
}

// ###########################################################################
// helpers
// ###########################################################################

fn opt_string(s: &str) -> Option<String> { if s.is_empty() { None } else { Some(s.to_string()) } }
fn opt_str(s: &str) -> Value { if s.is_empty() { Value::Null } else { Value::String(s.to_string()) } }
fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    let tod = secs.rem_euclid(86_400);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, tod / 3600, (tod % 3600) / 60, tod % 60)
}
/// days-since-epoch -> (year, month, day). Howard Hinnant's civil algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
