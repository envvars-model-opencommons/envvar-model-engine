//! Dates — a validated review date, plus the UTC clock helpers.
use std::fmt;

/// A human review date (`YYYY-MM-DD`).
///
/// Hand-written **by design**: it asserts *"a person verified this entry as of
/// this date"*, which no tool can derive. (Contrast `modified`, which is stamped
/// automatically — a human would forget to bump it and the field would lie.)
/// Const-validated and ordered, so `reviewed <= today` is checkable.
///
/// ```
/// # use argenv::ReviewDate;
/// const R: ReviewDate = ReviewDate::parse("2026-07-21");
/// assert_eq!(R.to_string(), "2026-07-21");
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct ReviewDate {
    /// Four-digit year.
    pub year: u16,
    /// Month, `1..=12`.
    pub month: u8,
    /// Day, `1..=31`.
    pub day: u8,
}

const fn digit(b: u8) -> u16 {
    if b < b'0' || b > b'9' {
        panic!("non-digit character in date");
    }
    (b - b'0') as u16
}
const fn two(b: &[u8], i: usize) -> u16 {
    digit(b[i]) * 10 + digit(b[i + 1])
}
const fn four(b: &[u8], i: usize) -> u16 {
    two(b, i) * 100 + two(b, i + 2)
}

impl ReviewDate {
    /// Parse `YYYY-MM-DD` in a `const` context.
    ///
    /// # Panics
    /// At **compile time** for a wrong length, missing dashes, non-digits, or a
    /// month/day outside range.
    pub const fn parse(s: &str) -> ReviewDate {
        let b = s.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            panic!("review date must be formatted YYYY-MM-DD");
        }
        let year = four(b, 0);
        let month = two(b, 5) as u8;
        let day = two(b, 8) as u8;
        if month < 1 || month > 12 {
            panic!("review date month out of range");
        }
        if day < 1 || day > 31 {
            panic!("review date day out of range");
        }
        ReviewDate { year, month, day }
    }

    /// Fallible runtime parse.
    pub fn try_parse(s: &str) -> Option<ReviewDate> {
        let b = s.as_bytes();
        if b.len() != 10
            || b[4] != b'-'
            || b[7] != b'-'
            || !b
                .iter()
                .enumerate()
                .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
        {
            return None;
        }
        let (y, m, d) = (four(b, 0), two(b, 5) as u8, two(b, 8) as u8);
        if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
            return None;
        }
        Some(ReviewDate {
            year: y,
            month: m,
            day: d,
        })
    }

    /// The following day. Used to allow one day of slack when comparing a
    /// hand-written local date against a UTC clock.
    pub fn next_day(self) -> ReviewDate {
        let days = days_from_civil(self.year as i64, self.month as u32, self.day as u32) + 1;
        let (y, m, dd) = civil_from_days(days);
        ReviewDate {
            year: y as u16,
            month: m as u8,
            day: dd as u8,
        }
    }

    /// Today in UTC, from the system clock. Used to reject future review dates.
    pub fn today() -> ReviewDate {
        let (y, m, d) = civil_from_days(unix_secs().div_euclid(86_400));
        ReviewDate {
            year: y as u16,
            month: m as u8,
            day: d as u8,
        }
    }
}

impl fmt::Display for ReviewDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

pub(crate) fn unix_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64
}

/// An RFC-3339 UTC timestamp for the current moment (used for `generated`).
pub(crate) fn now_iso() -> String {
    let secs = unix_secs();
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    let tod = secs.rem_euclid(86_400);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m,
        d,
        tod / 3600,
        (tod % 3600) / 60,
        tod % 60
    )
}

/// (year, month, day) -> days-since-epoch. Howard Hinnant's days-from-civil.
pub(crate) fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// days-since-epoch -> (year, month, day). Howard Hinnant's civil-from-days.
pub(crate) fn civil_from_days(z: i64) -> (i64, u32, u32) {
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
