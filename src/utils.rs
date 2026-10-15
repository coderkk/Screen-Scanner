//! Miscellaneous utility functions.

use std::time::{SystemTime, UNIX_EPOCH};

/// Returns a human‑readable UTC timestamp string in `YYYY-MM-DD HH:MM:SS` format.
///
/// This is a lightweight, dependency‑free alternative to pulling in the `chrono` crate.
/// The date arithmetic handles leap years correctly.
pub fn chrono_now_str() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let secs  = now % 60;
    let mins  = (now / 60) % 60;
    let hours = (now / 3600) % 24;
    let days  = now / 86400;

    // Walk from 1970‑01‑01 to find the current year and remaining day‑of‑year
    let mut year: i64 = 1970;
    let mut d = days as i64;
    loop {
        let leap = is_leap(year);
        let days_in_year: i64 = if leap { 366 } else { 365 };
        if d >= days_in_year {
            d -= days_in_year;
            year += 1;
        } else {
            break;
        }
    }

    let leap = is_leap(year);
    let month_days: [i64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];

    let mut month: u32 = 1;
    for &md in &month_days {
        if d >= md {
            d -= md;
            month += 1;
        } else {
            break;
        }
    }
    let day = d + 1;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hours, mins, secs
    )
}

/// Returns `true` if `year` is a leap year according to the Gregorian calendar.
fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

// ────────────────────────────────────────────────────────────
// Unit tests
// ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chrono_now_str_has_expected_format() {
        let s = chrono_now_str();
        // e.g. "2026-10-15 14:07:00"
        assert_eq!(s.len(), 19, "Timestamp string should be 19 chars: {}", s);
        assert_eq!(&s[4..5], "-");
        assert_eq!(&s[7..8], "-");
        assert_eq!(&s[10..11], " ");
        assert_eq!(&s[13..14], ":");
        assert_eq!(&s[16..17], ":");
    }

    #[test]
    fn leap_year_detection() {
        assert!(is_leap(2000), "2000 is a leap year");
        assert!(!is_leap(1900), "1900 is NOT a leap year");
        assert!(is_leap(2024), "2024 is a leap year");
        assert!(!is_leap(2023), "2023 is NOT a leap year");
    }
}
