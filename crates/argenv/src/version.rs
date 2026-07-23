//! Version — a parsed, ordered project version. Never a bare string.
use std::fmt;

/// A project version (`major[.minor[.patch]]`).
///
/// Constructed only via [`Version::parse`], so a malformed literal is a **compile
/// error** when used in a `const`. Ordered, which makes `since <= THIS_VERSION`
/// a checkable invariant rather than a comment.
///
/// ```
/// # use argenv::Version;
/// const V: Version = Version::parse("2.1");
/// assert_eq!(V.to_string(), "2.1.0");
/// assert!(Version::parse("2.1") < Version::parse("2.2"));
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Version {
    /// Major component.
    pub major: u16,
    /// Minor component (0 when omitted).
    pub minor: u16,
    /// Patch component (0 when omitted).
    pub patch: u16,
}

impl Version {
    /// Parse `major[.minor[.patch]]` in a `const` context.
    ///
    /// A semantic-version pre-release or build suffix (`0.2.0-rc1`, `1.0.0+d34d`)
    /// is accepted and ignored: only the numeric core is retained, so ordering
    /// compares releases rather than release-candidate labels. This matters
    /// because [`THIS_VERSION`] parses the crate's own version, and a project
    /// that tags a pre-release must still compile.
    ///
    /// # Panics
    /// At **compile time** (in a `const`) for a non-numeric character in the
    /// numeric core, more than three components, or an empty string.
    pub const fn parse(s: &str) -> Version {
        let b = s.as_bytes();
        if b.is_empty() {
            panic!("version string is empty");
        }
        let mut n = [0u16; 3];
        let mut idx = 0usize;
        let mut i = 0usize;
        while i < b.len() {
            let c = b[i];
            if c == b'-' || c == b'+' {
                break; // pre-release or build metadata: not part of the ordering
            } else if c == b'.' {
                idx += 1;
                if idx > 2 {
                    panic!("version has more than three components");
                }
            } else if c >= b'0' && c <= b'9' {
                n[idx] = n[idx] * 10 + (c - b'0') as u16;
            } else {
                panic!("invalid character in version string");
            }
            i += 1;
        }
        Version {
            major: n[0],
            minor: n[1],
            patch: n[2],
        }
    }

    /// Fallible runtime parse (for data read from JSON rather than authored).
    ///
    /// Accepts and ignores a pre-release or build suffix, as [`Version::parse`] does.
    pub fn try_parse(s: &str) -> Option<Version> {
        let mut n = [0u16; 3];
        let mut idx = 0usize;
        if s.is_empty() {
            return None;
        }
        let core = s.split(['-', '+']).next().unwrap_or(s);
        if core.is_empty() {
            return None;
        }
        for c in core.bytes() {
            if c == b'.' {
                idx += 1;
                if idx > 2 {
                    return None;
                }
            } else if c.is_ascii_digit() {
                n[idx] = n[idx].checked_mul(10)?.checked_add((c - b'0') as u16)?;
            } else {
                return None;
            }
        }
        Some(Version {
            major: n[0],
            minor: n[1],
            patch: n[2],
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// The version of the crate currently building. Reference it via
/// [`crate::Since::This`] for a variable added *now*, instead of hardcoding a
/// number that would silently drift.
pub const THIS_VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION"));
