//! Calendar helpers. All dates are timezone-free local calendar dates (`NaiveDate`).
//! The UI supplies "today" explicitly, so results never depend on the host time zone or DST.
use crate::error::{invalid, Result};
use crate::model::Tracker;
use chrono::{Datelike, NaiveDate};

pub fn parse_date(s: &str) -> Result<NaiveDate> {
    if s.len() != 10 {
        return invalid(format!("Invalid date “{s}”. Use YYYY-MM-DD."));
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .or_else(|_| invalid(format!("Invalid date “{s}”.")))
}

pub fn fmt_date(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

pub fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

pub fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => if is_leap(y) { 29 } else { 28 },
    }
}

/// Column (0..6) of a date in a week grid whose first column is `week_start` (0 = Sunday).
pub fn week_column(d: NaiveDate, week_start: u32) -> u32 {
    (d.weekday().num_days_from_sunday() + 7 - week_start % 7) % 7
}

/// Is the weekday of `d` set in the tracker's schedule bitmask (bit 0 = Sunday)?
pub fn is_active_weekday(mask: u8, d: NaiveDate) -> bool {
    (mask >> d.weekday().num_days_from_sunday()) & 1 == 1
}

pub fn in_period(t: &Tracker, d: NaiveDate) -> bool {
    let after = t.start_date.as_deref().and_then(|s| parse_date(s).ok()).map_or(true, |s| d >= s);
    let before = t.end_date.as_deref().and_then(|s| parse_date(s).ok()).map_or(true, |e| d <= e);
    after && before
}

/// A day on which the tracker is expected to be done (inside its period and on a scheduled weekday).
pub fn is_scheduled(t: &Tracker, d: NaiveDate) -> bool {
    in_period(t, d) && is_active_weekday(t.active_days, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn d(s: &str) -> NaiveDate { parse_date(s).unwrap() }

    #[test]
    fn leap_years() {
        assert!(is_leap(2024) && is_leap(2000));
        assert!(!is_leap(1900) && !is_leap(2100) && !is_leap(2023));
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2023, 2), 28);
        assert!(parse_date("2024-02-29").is_ok());
        assert!(parse_date("2023-02-29").is_err());
        assert!(parse_date("1900-02-29").is_err());
    }

    #[test]
    fn month_lengths() {
        let total: u32 = (1..=12).map(|m| days_in_month(2025, m)).sum();
        assert_eq!(total, 365);
        let total: u32 = (1..=12).map(|m| days_in_month(2024, m)).sum();
        assert_eq!(total, 366);
    }

    #[test]
    fn rejects_malformed_dates() {
        for bad in ["2024-2-3", "24-02-03", "2024/02/03", "", "2024-13-01", "2024-00-10", "2024-04-31"] {
            assert!(parse_date(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn weekday_alignment() {
        // 2025-10-01 is a Wednesday.
        assert_eq!(week_column(d("2025-10-01"), 0), 3);
        assert_eq!(week_column(d("2025-10-01"), 1), 2);
        // 2024-12-29 is a Sunday: first column when weeks start Sunday, last when Monday.
        assert_eq!(week_column(d("2024-12-29"), 0), 0);
        assert_eq!(week_column(d("2024-12-29"), 1), 6);
    }

    #[test]
    fn year_and_month_boundaries_and_dst_are_plain_calendar_math() {
        assert_eq!(d("2024-12-31").succ_opt().unwrap(), d("2025-01-01"));
        assert_eq!(d("2024-02-28").succ_opt().unwrap(), d("2024-02-29"));
        assert_eq!(d("2024-02-29").succ_opt().unwrap(), d("2024-03-01"));
        // US and EU spring-forward dates are ordinary 1-day steps for naive dates.
        assert_eq!(d("2024-03-10").succ_opt().unwrap(), d("2024-03-11"));
        assert_eq!(d("2024-03-31").succ_opt().unwrap(), d("2024-04-01"));
    }

    #[test]
    fn schedule_mask_uses_sunday_as_bit_zero() {
        let weekdays_only = 0b0111110; // Mon-Fri
        assert!(is_active_weekday(weekdays_only, d("2025-10-01"))); // Wed
        assert!(!is_active_weekday(weekdays_only, d("2025-10-04"))); // Sat
        assert!(!is_active_weekday(weekdays_only, d("2025-10-05"))); // Sun
    }
}
