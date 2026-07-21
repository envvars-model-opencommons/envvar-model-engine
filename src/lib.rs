//! # proton-env-model — the standard env-var contract for the Proton stack
//!
//! This crate is the **single canonical definition** of what an environment
//! variable declaration looks like across Proton, DXVK, vkd3d-proton, Wine, and
//! consumers like gamesteam. It owns the *shape*; projects author only the
//! *data* (their own list of vars). The shape is never hand-copied per project.
//!
//! Two layers, deliberately separated:
//!   * [`Meta`] + [`Var`] — the **rich authoring types** (Rust-only): const-checked
//!     versions, exhaustive enums, range-validated newtypes, compile-checked
//!     references. This is where the guarantees live.
//!   * [`Record`] — the **portable contract type** (`Serialize`, and `JsonSchema`
//!     under `--features contract`). Every language binding implements *this*.
//!     [`Var::to_record`] renders rich → portable.
//!
//! The cross-language "swagger" is `schema_for!(Record)` (see [`contract_schema`]).
//! Non-Rust projects validate their emitted var JSON against it in CI; they never
//! reproduce the shape by hand.
#![allow(dead_code)]

use serde::Serialize;
use serde_json::Value;
use std::fmt;

/// Bump on any breaking change to [`Record`]. Additive changes (new optional
/// field) keep the major and stay backward-compatible.
pub const CONTRACT_VERSION: u32 = 1;

// ===========================================================================
// Domain primitives — NO naked primitive stands for a domain-meaningful value.
// Each of these makes illegal states unconstructible (at compile time in Rust).
// ===========================================================================

/// A project version (`major.minor.patch`). Constructed only via [`Version::parse`],
/// which rejects malformed input *at compile time* when used in a `const`.
/// Ordered, so `since <= THIS_VERSION` is a checkable invariant.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl Version {
    /// `const` parser: `Version::parse("2.1")` is validated when the const is built.
    /// A bad literal (`"2.x"`, `"two"`) is a compile error, not a runtime surprise.
    pub const fn parse(s: &str) -> Version {
        let b = s.as_bytes();
        let mut nums = [0u16; 3];
        let mut idx = 0usize;
        let mut i = 0usize;
        while i < b.len() {
            let c = b[i];
            if c == b'.' {
                idx += 1;
                if idx > 2 {
                    panic!("version has too many components (expected major.minor.patch)");
                }
            } else if c >= b'0' && c <= b'9' {
                nums[idx] = nums[idx] * 10 + (c - b'0') as u16;
            } else {
                panic!("invalid character in version string");
            }
            i += 1;
        }
        Version { major: nums[0], minor: nums[1], patch: nums[2] }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// The version of the build emitting the schema. A var added *now* references
/// this (`Since::This`) instead of hardcoding a number that would drift.
pub const THIS_VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION"));

// -- small const digit helpers for date parsing --
const fn digit(b: u8) -> u16 {
    if b < b'0' || b > b'9' {
        panic!("non-digit where a date digit was expected");
    }
    (b - b'0') as u16
}
const fn two(b: &[u8], i: usize) -> u16 { digit(b[i]) * 10 + digit(b[i + 1]) }
const fn four(b: &[u8], i: usize) -> u16 { two(b, i) * 100 + two(b, i + 2) }

/// A human review date (`YYYY-MM-DD`). Hand-written **by design** — it encodes a
/// human assertion ("someone verified this entry as of this date") that no tool
/// can derive. Const-validated; ordered, so `reviewed <= today` is checkable.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReviewDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl ReviewDate {
    pub const fn parse(s: &str) -> ReviewDate {
        let b = s.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            panic!("review date must be YYYY-MM-DD");
        }
        let year = four(b, 0);
        let month = two(b, 5) as u8;
        let day = two(b, 8) as u8;
        if month < 1 || month > 12 || day < 1 || day > 31 {
            panic!("review date out of range");
        }
        ReviewDate { year, month, day }
    }

    /// Today, in UTC — derived from the system clock via a proleptic-Gregorian
    /// conversion (no external calendar crate). Used to reject future review dates.
    pub fn today() -> ReviewDate {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0) as i64;
        let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
        ReviewDate { year: y as u16, month: m as u8, day: d as u8 }
    }
}

impl fmt::Display for ReviewDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// days-since-epoch -> (year, month, day). Howard Hinnant's civil algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as i64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Coarse wire "kind" of a variable (what the *contract* exposes). The rich Rust
/// type on [`Var<T>`] is the real enforcement; this is the portable label.
#[derive(Clone, Copy)]
pub enum Type { String, Bool, Enum, Flags, Uint, Int, Float, Path }

impl Type {
    fn as_str(self) -> &'static str {
        match self {
            Type::String => "string",
            Type::Bool => "bool",
            Type::Enum => "enum",
            Type::Flags => "flags",
            Type::Uint => "uint",
            Type::Int => "int",
            Type::Float => "float",
            Type::Path => "path",
        }
    }
}

/// Lifecycle stability. `Deprecated` must be paired with [`Deprecation`] (enforced
/// by [`validate`]).
#[derive(Clone, Copy, PartialEq)]
pub enum Stability { Unknown, Stable, Experimental, Debug, Deprecated }

impl Stability {
    fn as_str(self) -> &'static str {
        match self {
            Stability::Unknown => "unknown",
            Stability::Stable => "stable",
            Stability::Experimental => "experimental",
            Stability::Debug => "debug",
            Stability::Deprecated => "deprecated",
        }
    }
}

/// When a var was introduced. Never a bare string:
///   * `This`       -> resolves to [`THIS_VERSION`] (use for a var added now).
///   * `At(Version)` -> an explicit, parsed, comparable version (checked <= current).
///   * `Unknown`    -> not yet classified (emitter may later derive from git history).
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

/// A reference to a config key in another surface (e.g. a `dxvk.conf` key that an
/// env var bridges to). Format-validated (must be namespaced `a.b`). It's an
/// *external* reference, so it can only be format-checked here, not resolved.
#[derive(Clone, Copy)]
pub struct ConfigKeyRef(pub &'static str);

impl ConfigKeyRef {
    pub const fn new(s: &'static str) -> ConfigKeyRef {
        let b = s.as_bytes();
        let mut i = 0usize;
        let mut has_dot = false;
        while i < b.len() {
            if b[i] == b'.' {
                has_dot = true;
            }
            i += 1;
        }
        if !has_dot {
            panic!("config key must be namespaced, e.g. \"dxgi.enableHDR\"");
        }
        ConfigKeyRef(s)
    }
}

/// Deprecation details, bundled so an entry can't be half-deprecated.
/// `replaced_by` is a **compile-checked** pointer to the successor's name (a fn
/// that reads another `Var`'s name — rename/remove the successor and this stops
/// compiling). `migration` is freeform human guidance for the no-clean-successor
/// case (a config key, "just remove it", etc.).
#[derive(Clone, Copy)]
pub struct Deprecation {
    pub since: Version,
    pub replaced_by: Option<fn() -> &'static str>,
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

// -- Example rich value domains (these ship with the standard as shared vocabulary;
//    projects reuse them or add their own following the same "no naked primitive" rule).

/// Enum domain whose accepted tokens are the *single source* for both parsing and
/// the schema's `allowed` list — so they cannot drift apart.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel { Trace, Debug, Info, Warn, Error, None }

impl LogLevel {
    pub const TOKENS: &'static [&'static str] = &["trace", "debug", "info", "warn", "error", "none"];
    pub fn parse(s: &str) -> Option<LogLevel> {
        Some(match s.trim() {
            "trace" => LogLevel::Trace,
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" => LogLevel::Warn,
            "error" => LogLevel::Error,
            "none" => LogLevel::None,
            _ => return None,
        })
    }
}

/// A number with a meaningful sentinel: `0` is "uncapped", not "zero frames".
/// Modeled as an enum so the sentinel is explicit and `Fps(n) + 1` is impossible.
#[derive(Clone, Copy)]
pub enum FrameCap { Uncapped, Fps(u32) }

impl FrameCap {
    pub fn parse(s: &str) -> Option<FrameCap> {
        let n: u32 = s.trim().parse().ok()?;
        Some(if n == 0 { FrameCap::Uncapped } else { FrameCap::Fps(n) })
    }
}
impl Serialize for FrameCap {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            FrameCap::Uncapped => s.serialize_u32(0),
            FrameCap::Fps(n) => s.serialize_u32(*n),
        }
    }
}

/// A constrained float: `0.1..=1.0`. Out-of-range (and `NaN`) can't be constructed.
#[derive(Clone, Copy, Serialize)]
#[serde(transparent)]
pub struct RenderScale(f32);

impl RenderScale {
    pub const fn new(v: f32) -> RenderScale {
        // NaN fails both comparisons -> panics, so NaN is rejected too.
        assert!(v >= 0.1 && v <= 1.0, "render scale must be within 0.1..=1.0");
        RenderScale(v)
    }
    pub fn get(self) -> f32 { self.0 }
    pub fn parse(s: &str) -> Option<RenderScale> {
        let v: f32 = s.trim().parse().ok()?;
        (v >= 0.1 && v <= 1.0).then_some(RenderScale(v))
    }
}

/// A three-state toggle — a `bool` would be a lie here.
#[derive(Clone, Copy, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Tristate { #[default] Auto, On, Off }

impl Tristate {
    pub const TOKENS: &'static [&'static str] = &["auto", "on", "off"];
    pub fn parse(s: &str) -> Option<Tristate> {
        Some(match s.trim() {
            "auto" | "" => Tristate::Auto,
            "on" | "1" | "true" => Tristate::On,
            "off" | "0" | "false" => Tristate::Off,
            _ => return None,
        })
    }
}

// ===========================================================================
// Meta — the rich AUTHORING type. Hand-written as terse literals; every
// domain-meaningful field is a typed primitive above. `Meta` is `Copy` so it
// composes freely in `const` var declarations.
// ===========================================================================

#[derive(Clone, Copy)]
pub struct Meta {
    /// REQUIRED. The exact environment variable name, e.g. "DXVK_HDR".
    pub name: &'static str,
    /// REQUIRED. Coarse wire kind (rich enforcement is the `Var<T>` type).
    pub ty: Type,
    /// Domain tokens for `Enum`/`Flags`. Wire them to the type's `TOKENS` const
    /// (e.g. `LogLevel::TOKENS`) so they never drift from the parser.
    pub allowed: &'static [&'static str],
    pub required: bool,
    pub stability: Stability,
    pub since: Since,
    pub deprecation: Option<Deprecation>,
    pub group: &'static str,
    pub maps_to: Option<ConfigKeyRef>,
    pub example: &'static str,
    /// A resolved-config field this flips (for the validation harness), e.g.
    /// gamescope's "bExposeHDRSupport". External identifier -> opaque string.
    pub observe: Option<&'static str>,
    /// HAND-WRITTEN human verification date. Empty = nobody has vouched for it.
    pub reviewed: Option<ReviewDate>,
    /// Freeform prose (legitimately a string). Community fills this in.
    pub summary: &'static str,
}

/// The empty baseline. Author a var by overriding only what's known:
/// `Meta { name: "...", ty: ..., ..Meta::EMPTY }`.
impl Meta {
    pub const EMPTY: Meta = Meta {
        name: "",
        ty: Type::String,
        allowed: &[],
        required: false,
        stability: Stability::Unknown,
        since: Since::Unknown,
        deprecation: None,
        group: "",
        maps_to: None,
        example: "",
        observe: None,
        reviewed: None,
        summary: "",
    };
}

// ===========================================================================
// Var<T> — a typed handle: schema row + typed accessor + compile-checked symbol,
// fused. `EnvModel::DXVK_HDR` is the name-checked symbol; `.get()` returns the
// declared type with absence made explicit.
// ===========================================================================

pub struct Var<T: 'static> {
    pub meta: Meta,
    /// The typed default (NOT a string). Rendered into the contract at emit time.
    pub default: Option<T>,
    /// Parses the raw env string into the domain type; `None` = absent/invalid.
    pub parse: fn(&str) -> Option<T>,
}

impl<T: 'static> Var<T> {
    /// Read + parse. Absent OR invalid -> `None` (never a silent empty string).
    pub fn get(&self) -> Option<T> {
        std::env::var(self.meta.name).ok().and_then(|s| (self.parse)(&s))
    }
    /// Read with a caller-supplied fallback for the absent/invalid case.
    pub fn get_or(&self, fallback: T) -> T {
        self.get().unwrap_or(fallback)
    }
    /// Was it explicitly set (regardless of validity)?
    pub fn is_set(&self) -> bool {
        std::env::var(self.meta.name).is_ok()
    }
}

impl<T: 'static + Clone> Var<T> {
    /// Read, or fall back to the declared default.
    pub fn get_or_default(&self) -> Option<T> {
        self.get().or_else(|| self.default.clone())
    }
}

impl<T: 'static + Serialize> Var<T> {
    /// Render this rich declaration down to the portable contract [`Record`].
    pub fn to_record(&self) -> Record {
        let m = &self.meta;
        Record {
            name: m.name.to_string(),
            ty: m.ty.as_str().to_string(),
            default: self
                .default
                .as_ref()
                .map(|d| serde_json::to_value(d).unwrap_or(Value::Null)),
            allowed: m.allowed.iter().map(|s| s.to_string()).collect(),
            required: m.required,
            stability: m.stability.as_str().to_string(),
            since: value_to_opt_string(m.since.to_json()),
            deprecation: m.deprecation.as_ref().map(|d| d.to_json()),
            group: opt_string(m.group),
            maps_to: m.maps_to.map(|c| c.0.to_string()),
            example: opt_string(m.example),
            observe: m.observe.map(|s| s.to_string()),
            reviewed: m.reviewed.map(|d| d.to_string()),
            summary: opt_string(m.summary),
            // AUTO provenance, stamped by tooling (git blame on the declaration).
            // Null here; the emitter binary fills it. Never hand-written.
            modified: None,
        }
    }
}

// ===========================================================================
// Record — the PORTABLE contract type. This is what the JSON schema describes
// and what every language binding must produce. Simple types only, so it maps
// cleanly to C/Python and to JSON Schema.
// ===========================================================================

#[derive(Serialize)]
#[cfg_attr(feature = "contract", derive(schemars::JsonSchema))]
pub struct Record {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    pub allowed: Vec<String>,
    pub required: bool,
    pub stability: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecation: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maps_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observe: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
}

// ===========================================================================
// Validation — invariants that hold across ANY project's var set. Run as a test
// (or a build step) so a bad declaration fails CI, not a user's launch.
// ===========================================================================

/// Check the invariants over a set of authored metas. Returns human-readable
/// problems; empty = valid.
pub fn validate(metas: &[Meta]) -> Vec<String> {
    let mut errs = Vec::new();
    let today = ReviewDate::today();
    for m in metas {
        if m.name.is_empty() {
            errs.push("a variable has an empty name".to_string());
        }
        if let Since::At(v) = m.since {
            if v > THIS_VERSION {
                errs.push(format!(
                    "{}: since {} is newer than the current build {}",
                    m.name, v, THIS_VERSION
                ));
            }
        }
        if let Some(r) = m.reviewed {
            if r > today {
                errs.push(format!("{}: reviewed date {} is in the future", m.name, r));
            }
        }
        match (m.stability, m.deprecation.is_some()) {
            (Stability::Deprecated, false) => {
                errs.push(format!("{}: marked Deprecated but has no Deprecation info", m.name));
            }
            (s, true) if s != Stability::Deprecated => {
                errs.push(format!("{}: has Deprecation info but stability is not Deprecated", m.name));
            }
            _ => {}
        }
        if let Some(dep) = &m.deprecation {
            if dep.since > THIS_VERSION {
                errs.push(format!(
                    "{}: deprecated-since {} is newer than the current build {}",
                    m.name, dep.since, THIS_VERSION
                ));
            }
        }
    }
    errs
}

// ===========================================================================
// Contract emission — the cross-language "swagger". Generated from `Record`, so
// it is never hand-maintained. Other bindings validate their output against it.
// ===========================================================================

/// The JSON-Schema contract for a single var record (requires `--features contract`).
#[cfg(feature = "contract")]
pub fn contract_schema() -> Value {
    serde_json::to_value(schemars::schema_for!(Record)).unwrap()
}

/// Wrap a project's rendered records in the standard envelope (version + provenance).
pub fn document(source: &str, records: Vec<Record>) -> Value {
    serde_json::json!({
        "contract_version": CONTRACT_VERSION,
        "source": source,               // e.g. "dxvk@0ff9cd3" — stamped by the emitter
        "generated": now_iso(),         // AUTO
        "vars": serde_json::to_value(records).unwrap(),
    })
}

// ===========================================================================
// small helpers
// ===========================================================================

fn opt_string(s: &str) -> Option<String> {
    if s.is_empty() { None } else { Some(s.to_string()) }
}
fn opt_str(s: &str) -> Value {
    if s.is_empty() { Value::Null } else { Value::String(s.to_string()) }
}
fn value_to_opt_string(v: Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s),
        _ => None,
    }
}
fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    let tod = secs.rem_euclid(86_400);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, tod / 3600, (tod % 3600) / 60, tod % 60
    )
}
