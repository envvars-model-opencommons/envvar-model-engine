//! [`FromRaw`] — how a type reads itself from a raw invocation string.
//!
//! The same trait serves both bindings: a value that arrives as `--log-level warn`
//! and one that arrives as `APP_LOG_LEVEL=warn` are the same value in the same
//! domain, and are parsed by the same code.
//!
//! Implementing this on the *value* type (much like [`std::str::FromStr`]) is
//! what lets a declaration carry no `parse` field: parsing lives with the type
//! that owns the domain, so the declaration stays a flat, readable literal.
use std::path::PathBuf;

/// Parse a value from the raw string an invocation carried.
///
/// Return `None` for "absent or invalid" — [`crate::Input::get`] turns that into
/// an explicit `Option`, so a malformed value can never silently become a
/// default-looking empty string.
pub trait FromRaw: Sized {
    /// Parse, or `None` if the text does not denote a valid value.
    fn from_raw(s: &str) -> Option<Self>;
}

impl FromRaw for bool {
    /// `1`/`true`/`on`/`yes` and `0`/`false`/`off`/`no`/empty; anything else is invalid.
    fn from_raw(s: &str) -> Option<bool> {
        match s.trim() {
            "1" | "true" | "True" | "on" | "yes" => Some(true),
            "0" | "false" | "False" | "off" | "no" | "" => Some(false),
            _ => None,
        }
    }
}

impl FromRaw for String {
    fn from_raw(s: &str) -> Option<String> {
        Some(s.to_string())
    }
}

impl FromRaw for PathBuf {
    /// An empty value is treated as absent rather than as the current directory.
    fn from_raw(s: &str) -> Option<PathBuf> {
        if s.trim().is_empty() {
            None
        } else {
            Some(PathBuf::from(s))
        }
    }
}

macro_rules! impl_from_raw_number {
    ($($t:ty),*) => { $(
        impl FromRaw for $t {
            fn from_raw(s: &str) -> Option<$t> { s.trim().parse::<$t>().ok() }
        }
    )* };
}
impl_from_raw_number!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64);

/// A three-state toggle. A `bool` would be a lie when "unset" is a real, distinct
/// choice — this makes the third state explicit instead of smuggling it in as
/// `Option<bool>`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Tristate {
    /// Decide at runtime (the usual default).
    #[default]
    Auto,
    /// Forced on.
    On,
    /// Forced off.
    Off,
}

impl Tristate {
    /// The accepted tokens — wire this to a declaration's `allowed` so the
    /// schema and the parser can never disagree.
    pub const TOKENS: &'static [&'static str] = &["auto", "on", "off"];
}

impl FromRaw for Tristate {
    fn from_raw(s: &str) -> Option<Tristate> {
        Some(match s.trim() {
            "auto" | "" => Tristate::Auto,
            "on" | "1" | "true" | "yes" => Tristate::On,
            "off" | "0" | "false" | "no" => Tristate::Off,
            _ => return None,
        })
    }
}

/// A conventional log-verbosity domain, provided because nearly every program
/// exposes one. Projects with different level names should declare their own
/// enum the same way.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    /// Maximum detail.
    Trace,
    /// Developer diagnostics.
    Debug,
    /// Normal operational messages.
    Info,
    /// Recoverable problems.
    Warn,
    /// Failures only.
    Error,
    /// No logging at all.
    None,
}

impl LogLevel {
    /// The accepted tokens — the single source for both parsing and `allowed`.
    pub const TOKENS: &'static [&'static str] =
        &["trace", "debug", "info", "warn", "error", "none"];
}

impl FromRaw for LogLevel {
    fn from_raw(s: &str) -> Option<LogLevel> {
        Some(match s.trim() {
            "trace" => LogLevel::Trace,
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" | "warning" => LogLevel::Warn,
            "error" | "err" => LogLevel::Error,
            "none" | "off" => LogLevel::None,
            _ => return None,
        })
    }
}
