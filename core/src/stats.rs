//! Pure statistics: no database, no clock. `today` is always passed in.
use crate::dates::{is_scheduled, week_column};
use crate::model::{GoalMode, Tracker, TrackerType};
use chrono::{Duration, NaiveDate};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// How several records on one day combine: ratings average, everything else sums.
pub fn aggregate(kind: TrackerType, vals: &[f64]) -> f64 {
    if vals.is_empty() { return 0.0; }
    match kind {
        TrackerType::Rating => vals.iter().sum::<f64>() / vals.len() as f64,
        _ => vals.iter().sum(),
    }
}

/// Heatmap intensity. 0 = explicit zero; 1–4 = share of the daily goal reached (>0, ≥33%, ≥66%, ≥100%).
pub fn level(value: f64, goal: f64) -> u8 {
    if value <= 0.0 { return 0; }
    let r = value / goal;
    if r >= 1.0 { 4 } else if r >= 0.66 { 3 } else if r >= 0.33 { 2 } else { 1 }
}

#[derive(Serialize, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub recorded_days: usize,
    pub active_days: usize,
    pub zero_days: usize,
    pub total: f64,
    pub average: Option<f64>,
    pub median: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub best_streak: u32,
    pub current_streak: u32,
    pub scheduled_days: u32,
    pub goal_days: u32,
    pub goal_hit_pct: Option<f64>,
    /// "days" for daily goals, "weeks" for weekly goals (applies to streaks and goal counts).
    pub unit: String,
}

pub type Frozen = BTreeSet<NaiveDate>;

/// Start of the week containing `d` for a grid whose first column is `week_start` (0 = Sunday).
pub fn week_start_of(d: NaiveDate, week_start: u32) -> NaiveDate {
    d - Duration::days(week_column(d, week_start) as i64)
}

/// Streaks. Daily goals: a scheduled day without a value above zero breaks the streak, unless it is frozen;
/// rest days and frozen days neither break nor extend it. The current streak is not broken by a still-empty today.
/// Weekly goals: a week succeeds with `weekly_target` qualifying days (value >= goal; a frozen day counts as one);
/// the unfinished current week never breaks the streak. Returns (best, current).
pub fn streaks(t: &Tracker, days: &BTreeMap<NaiveDate, f64>, frozen: &Frozen, today: NaiveDate, week_start: u32) -> (u32, u32) {
    match t.goal_mode {
        GoalMode::Weekly => weekly_streaks(t, days, frozen, today, week_start),
        GoalMode::Daily => daily_streaks(t, days, frozen, today),
    }
}

fn daily_streaks(t: &Tracker, days: &BTreeMap<NaiveDate, f64>, frozen: &Frozen, today: NaiveDate) -> (u32, u32) {
    let done = |d: NaiveDate| days.get(&d).map_or(false, |v| *v > 0.0);
    let breaks = |d: NaiveDate| is_scheduled(t, d) && !frozen.contains(&d);
    let Some(&first) = days.keys().next() else { return (0, 0) };

    let (mut best, mut run) = (0u32, 0u32);
    let mut d = first;
    while d <= today {
        if done(d) { run += 1; best = best.max(run); } else if breaks(d) { run = 0; }
        match d.succ_opt() { Some(n) => d = n, None => break }
    }

    let mut cur = 0u32;
    let mut d = today;
    if !done(d) && breaks(d) { d = match d.pred_opt() { Some(p) => p, None => return (best, 0) }; }
    while d >= first {
        if done(d) { cur += 1; } else if breaks(d) { break; }
        match d.pred_opt() { Some(p) => d = p, None => break }
    }
    (best, cur)
}

fn week_qualifying(t: &Tracker, days: &BTreeMap<NaiveDate, f64>, frozen: &Frozen, w: NaiveDate) -> u32 {
    (0..7).filter(|i| {
        let d = w + Duration::days(*i);
        days.get(&d).map_or(false, |v| *v > 0.0 && *v >= t.goal) || frozen.contains(&d)
    }).count() as u32
}

fn weekly_streaks(t: &Tracker, days: &BTreeMap<NaiveDate, f64>, frozen: &Frozen, today: NaiveDate, ws: u32) -> (u32, u32) {
    let Some(&first) = days.keys().next() else { return (0, 0) };
    let this_week = week_start_of(today, ws);
    let (mut best, mut run) = (0u32, 0u32);
    let mut w = week_start_of(first, ws);
    while w <= this_week {
        let ok = week_qualifying(t, days, frozen, w) >= t.weekly_target as u32;
        if ok { run += 1; best = best.max(run); } else if w < this_week { run = 0; }
        w += Duration::days(7);
    }
    (best, run)
}

pub fn compute_stats(t: &Tracker, days: &BTreeMap<NaiveDate, f64>, frozen: &Frozen, today: NaiveDate, week_start: u32) -> Stats {
    let vals: Vec<f64> = days.values().copied().collect();
    let mut s = Stats { recorded_days: vals.len(), unit: if t.goal_mode == GoalMode::Weekly { "weeks" } else { "days" }.into(), ..Default::default() };
    s.active_days = vals.iter().filter(|v| **v > 0.0).count();
    s.zero_days = vals.iter().filter(|v| **v <= 0.0).count();
    s.total = vals.iter().sum();
    if !vals.is_empty() {
        let mut sorted = vals.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = sorted.len();
        s.average = Some(s.total / n as f64);
        s.median = Some((sorted[(n - 1) / 2] + sorted[n / 2]) / 2.0);
        s.min = Some(sorted[0]);
        s.max = Some(sorted[n - 1]);
    }
    let (best, cur) = streaks(t, days, frozen, today, week_start);
    s.best_streak = best;
    s.current_streak = cur;
    if let Some(&first) = days.keys().next() {
        match t.goal_mode {
            GoalMode::Daily => {
                // Goal completion: scheduled days (not frozen) from the first record through today.
                let mut d = first;
                while d <= today {
                    if is_scheduled(t, d) && !frozen.contains(&d) {
                        s.scheduled_days += 1;
                        if days.get(&d).map_or(false, |v| *v >= t.goal) { s.goal_days += 1; }
                    }
                    match d.succ_opt() { Some(n) => d = n, None => break }
                }
            }
            GoalMode::Weekly => {
                // Weeks finished (plus the current week once it has succeeded) and how many reached the target.
                let this_week = week_start_of(today, week_start);
                let mut w = week_start_of(first, week_start);
                while w <= this_week {
                    let ok = week_qualifying(t, days, frozen, w) >= t.weekly_target as u32;
                    if w < this_week || ok { s.scheduled_days += 1; if ok { s.goal_days += 1; } }
                    w += Duration::days(7);
                }
            }
        }
        if s.scheduled_days > 0 { s.goal_hit_pct = Some(s.goal_days as f64 * 100.0 / s.scheduled_days as f64); }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::parse_date;

    fn d(s: &str) -> NaiveDate { parse_date(s).unwrap() }
    fn tracker(mask: u8) -> Tracker {
        Tracker { id: "t".into(), name: "T".into(), kind: TrackerType::Count, unit: "".into(), goal: 2.0,
                  color: "#3ddc84".into(), active_days: mask, start_date: None, end_date: None, archived: false, sort: 0,
                  goal_mode: GoalMode::Daily, weekly_target: 7, freezes_per_month: 0 }
    }
    fn days(list: &[(&str, f64)]) -> BTreeMap<NaiveDate, f64> { list.iter().map(|(s, v)| (d(s), *v)).collect() }
    fn st(t: &Tracker, m: &BTreeMap<NaiveDate, f64>, today: &str) -> (u32, u32) { streaks(t, m, &Frozen::new(), d(today), 0) }
    fn frozen(list: &[&str]) -> Frozen { list.iter().map(|s| d(s)).collect() }

    #[test]
    fn aggregation_rules() {
        assert_eq!(aggregate(TrackerType::Duration, &[30.0, 45.0, 15.0]), 90.0);
        assert_eq!(aggregate(TrackerType::Count, &[1.0, 1.0]), 2.0);
        assert_eq!(aggregate(TrackerType::Rating, &[2.0, 4.0, 5.0]), 11.0 / 3.0);
        assert_eq!(aggregate(TrackerType::Numeric, &[0.5, 0.25]), 0.75);
    }

    #[test]
    fn intensity_levels_follow_goal() {
        assert_eq!(level(0.0, 60.0), 0);
        assert_eq!(level(1.0, 60.0), 1);
        assert_eq!(level(20.0, 60.0), 2);
        assert_eq!(level(40.0, 60.0), 3);
        assert_eq!(level(60.0, 60.0), 4);
        assert_eq!(level(600.0, 60.0), 4);
    }

    #[test]
    fn streak_breaks_on_missed_scheduled_day() {
        let t = tracker(127);
        let m = days(&[("2025-03-01", 1.0), ("2025-03-02", 1.0), ("2025-03-04", 1.0), ("2025-03-05", 1.0), ("2025-03-06", 1.0)]);
        assert_eq!(st(&t, &m, "2025-03-06"), (3, 3));
        assert_eq!(st(&t, &m, "2025-03-08").1, 0);
    }

    #[test]
    fn empty_today_does_not_break_current_streak() {
        let t = tracker(127);
        let m = days(&[("2025-03-04", 1.0), ("2025-03-05", 1.0)]);
        assert_eq!(st(&t, &m, "2025-03-06"), (2, 2));
    }

    #[test]
    fn rest_days_do_not_break_streaks() {
        let t = tracker(0b0111110);
        let m = days(&[("2025-03-06", 1.0), ("2025-03-07", 1.0), ("2025-03-10", 1.0), ("2025-03-11", 1.0)]);
        assert_eq!(st(&t, &m, "2025-03-11"), (4, 4));
        assert_eq!(st(&tracker(127), &m, "2025-03-11"), (2, 2));
    }

    #[test]
    fn rest_day_with_data_counts_but_missing_rest_day_does_not_fail() {
        let t = tracker(0b0111110);
        let m = days(&[("2025-03-07", 1.0), ("2025-03-08", 1.0), ("2025-03-10", 1.0)]);
        assert_eq!(st(&t, &m, "2025-03-10"), (3, 3));
    }

    #[test]
    fn frozen_day_keeps_the_streak_alive_without_extending_it() {
        let t = tracker(127);
        let m = days(&[("2025-03-01", 1.0), ("2025-03-02", 1.0), ("2025-03-04", 1.0), ("2025-03-05", 1.0)]);
        assert_eq!(st(&t, &m, "2025-03-05"), (2, 2), "without a freeze the gap on the 3rd breaks it");
        assert_eq!(streaks(&t, &m, &frozen(&["2025-03-03"]), d("2025-03-05"), 0), (4, 4));
    }

    #[test]
    fn explicit_zero_breaks_streak_but_counts_as_recorded() {
        let t = tracker(127);
        let m = days(&[("2025-03-01", 1.0), ("2025-03-02", 0.0), ("2025-03-03", 1.0)]);
        let s = compute_stats(&t, &m, &Frozen::new(), d("2025-03-03"), 0);
        assert_eq!((s.best_streak, s.current_streak), (1, 1));
        assert_eq!((s.recorded_days, s.active_days, s.zero_days), (3, 2, 1));
    }

    #[test]
    fn streaks_cross_year_leap_day_and_dst_dates() {
        let t = tracker(127);
        assert_eq!(st(&t, &days(&[("2024-12-30", 1.0), ("2024-12-31", 1.0), ("2025-01-01", 1.0)]), "2025-01-01"), (3, 3));
        assert_eq!(st(&t, &days(&[("2024-02-28", 1.0), ("2024-02-29", 1.0), ("2024-03-01", 1.0)]), "2024-03-01"), (3, 3));
        let m = days(&[("2024-03-09", 1.0), ("2024-03-10", 1.0), ("2024-03-11", 1.0), ("2024-03-30", 1.0), ("2024-03-31", 1.0), ("2024-04-01", 1.0)]);
        assert_eq!(st(&t, &m, "2024-03-11").0, 3);
    }

    #[test]
    fn schedule_period_limits_goal_denominator() {
        let mut t = tracker(0b0111110);
        t.start_date = Some("2025-03-03".into());
        let m = days(&[("2025-03-03", 2.0), ("2025-03-04", 1.0), ("2025-03-05", 3.0)]);
        let s = compute_stats(&t, &m, &Frozen::new(), d("2025-03-09"), 0);
        assert_eq!((s.scheduled_days, s.goal_days), (5, 2));
        assert_eq!(s.goal_hit_pct, Some(40.0));
        assert_eq!(s.unit, "days");
    }

    #[test]
    fn basic_statistics() {
        let t = tracker(127);
        let m = days(&[("2025-03-01", 1.0), ("2025-03-02", 4.0), ("2025-03-03", 10.0), ("2025-03-04", 5.0)]);
        let s = compute_stats(&t, &m, &Frozen::new(), d("2025-03-04"), 0);
        assert_eq!((s.total, s.average, s.median, s.min, s.max), (20.0, Some(5.0), Some(4.5), Some(1.0), Some(10.0)));
        let empty = compute_stats(&t, &BTreeMap::new(), &Frozen::new(), d("2025-03-04"), 0);
        assert_eq!((empty.average, empty.best_streak, empty.goal_hit_pct), (None, 0, None));
    }

    fn weekly(target: u8) -> Tracker { let mut t = tracker(127); t.goal = 1.0; t.goal_mode = GoalMode::Weekly; t.weekly_target = target; t }

    #[test]
    fn weekly_goal_counts_weeks_not_days() {
        // Weeks start Monday (week_start = 1). W1 Mar 3–9: 3 days ok. W2 Mar 10–16: 2 days fail. W3 Mar 17–23: 3 days ok.
        let m = days(&[("2025-03-03", 1.0), ("2025-03-04", 1.0), ("2025-03-06", 1.0), ("2025-03-10", 1.0), ("2025-03-12", 1.0),
                       ("2025-03-17", 1.0), ("2025-03-18", 1.0), ("2025-03-19", 1.0)]);
        assert_eq!(streaks(&weekly(3), &m, &Frozen::new(), d("2025-03-20"), 1), (1, 1));
        let s = compute_stats(&weekly(3), &m, &Frozen::new(), d("2025-03-20"), 1);
        assert_eq!((s.unit.as_str(), s.scheduled_days, s.goal_days), ("weeks", 3, 2));
    }

    #[test]
    fn unfinished_current_week_does_not_break_a_weekly_streak() {
        let m = days(&[("2025-03-03", 1.0), ("2025-03-04", 1.0), ("2025-03-05", 1.0), ("2025-03-10", 1.0), ("2025-03-11", 1.0), ("2025-03-12", 1.0), ("2025-03-17", 1.0)]);
        assert_eq!(streaks(&weekly(3), &m, &Frozen::new(), d("2025-03-19"), 1), (2, 2));
        // Once that week is over without reaching the target, the streak is gone but the best remains.
        assert_eq!(streaks(&weekly(3), &m, &Frozen::new(), d("2025-03-24"), 1), (2, 0));
    }

    #[test]
    fn weekly_goal_requires_the_daily_goal_and_a_freeze_counts_as_a_day() {
        let mut t = weekly(2);
        t.goal = 5.0;
        let m = days(&[("2025-03-03", 5.0), ("2025-03-04", 4.0)]); // second day misses the daily goal
        assert_eq!(streaks(&t, &m, &Frozen::new(), d("2025-03-10"), 1).0, 0);
        assert_eq!(streaks(&t, &m, &frozen(&["2025-03-04"]), d("2025-03-10"), 1).0, 1);
    }

    #[test]
    fn week_start_alignment() {
        assert_eq!(week_start_of(d("2025-03-09"), 1), d("2025-03-03")); // Sunday belongs to the Monday-based week before it
        assert_eq!(week_start_of(d("2025-03-09"), 0), d("2025-03-09"));
    }
}
