//! Descriptive insights computed from stored daily values. These describe patterns in your own data;
//! a correlation is never evidence that one thing causes another.
use crate::dates::fmt_date;
use chrono::{Datelike, Duration, NaiveDate};
use serde::Serialize;
use std::collections::BTreeMap;

pub fn pearson(xs: &[f64], ys: &[f64]) -> Option<f64> {
    let n = xs.len();
    if n < 2 || n != ys.len() { return None; }
    let (mx, my) = (xs.iter().sum::<f64>() / n as f64, ys.iter().sum::<f64>() / n as f64);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (x, y) in xs.iter().zip(ys) { sxy += (x - mx) * (y - my); sxx += (x - mx).powi(2); syy += (y - my).powi(2); }
    if sxx == 0.0 || syy == 0.0 { None } else { Some(sxy / (sxx * syy).sqrt()) }
}

fn strength(r: f64) -> &'static str {
    match r.abs() { x if x < 0.2 => "negligible", x if x < 0.4 => "weak", x if x < 0.6 => "moderate", x if x < 0.8 => "strong", _ => "very strong" }
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Correlation { pub n: usize, pub r: f64, pub lag: i64, pub strength: &'static str, pub caution: &'static str }

/// Compares A on day d with B on day d+lag, using only days where both were recorded. Needs ≥ 14 pairs.
pub fn correlate(a: &BTreeMap<NaiveDate, f64>, b: &BTreeMap<NaiveDate, f64>, lag: i64, from: NaiveDate, to: NaiveDate) -> Option<Correlation> {
    let (mut xs, mut ys) = (vec![], vec![]);
    let mut d = from;
    while d <= to {
        if let (Some(x), Some(y)) = (a.get(&d), b.get(&(d + Duration::days(lag)))) { xs.push(*x); ys.push(*y); }
        d += Duration::days(1);
    }
    if xs.len() < 14 { return None; }
    let r = pearson(&xs, &ys)?;
    Some(Correlation { n: xs.len(), r, lag, strength: strength(r), caution: "A correlation describes how two series move together in your records. It does not show that one causes the other." })
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Outlier { pub date: String, pub value: f64, pub direction: &'static str }

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Insights {
    pub recorded_days: usize,
    /// Average recorded value per weekday, index 0 = Sunday.
    pub weekday_avg: Vec<Option<f64>>,
    pub weekday_days: Vec<usize>,
    pub best_weekday: Option<usize>,
    pub last30_avg: Option<f64>,
    pub prev30_avg: Option<f64>,
    pub change_pct: Option<f64>,
    pub busiest: Option<Outlier>,
    pub outliers: Vec<Outlier>,
}

fn avg(v: &[f64]) -> Option<f64> { if v.is_empty() { None } else { Some(v.iter().sum::<f64>() / v.len() as f64) } }

pub fn compute_insights(days: &BTreeMap<NaiveDate, f64>, today: NaiveDate) -> Insights {
    let mut sums = [0.0f64; 7];
    let mut counts = [0usize; 7];
    for (d, v) in days {
        let i = d.weekday().num_days_from_sunday() as usize;
        sums[i] += v;
        counts[i] += 1;
    }
    let weekday_avg: Vec<Option<f64>> = (0..7).map(|i| if counts[i] > 0 { Some(sums[i] / counts[i] as f64) } else { None }).collect();
    let best_weekday = (0..7).filter(|i| counts[*i] >= 2).max_by(|a, b| weekday_avg[*a].partial_cmp(&weekday_avg[*b]).unwrap());
    let in_range = |from: i64, to: i64| -> Vec<f64> {
        days.range((today - Duration::days(from))..=(today - Duration::days(to))).map(|(_, v)| *v).collect()
    };
    let (last, prev) = (avg(&in_range(29, 0)), avg(&in_range(59, 30)));
    let change_pct = match (last, prev) { (Some(l), Some(p)) if p > 0.0 => Some((l - p) / p * 100.0), _ => None };
    let busiest = days.iter().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).filter(|(_, v)| **v > 0.0)
        .map(|(d, v)| Outlier { date: fmt_date(*d), value: *v, direction: "high" });

    // Unusual days in the last 90 days: more than 2 standard deviations from that window's mean.
    let recent: Vec<(&NaiveDate, &f64)> = days.range((today - Duration::days(89))..=today).collect();
    let mut outliers = vec![];
    if recent.len() >= 10 {
        let vals: Vec<f64> = recent.iter().map(|(_, v)| **v).collect();
        let m = vals.iter().sum::<f64>() / vals.len() as f64;
        let sd = (vals.iter().map(|v| (v - m).powi(2)).sum::<f64>() / vals.len() as f64).sqrt();
        if sd > 0.0 {
            for (d, v) in &recent {
                if (**v - m).abs() > 2.0 * sd { outliers.push(Outlier { date: fmt_date(**d), value: **v, direction: if **v > m { "high" } else { "low" } }); }
            }
            outliers.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap());
            outliers.truncate(3);
        }
    }
    Insights { recorded_days: days.len(), weekday_avg, weekday_days: counts.to_vec(), best_weekday, last30_avg: last, prev30_avg: prev, change_pct, busiest, outliers }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::parse_date;
    fn d(s: &str) -> NaiveDate { parse_date(s).unwrap() }
    fn series(start: &str, vals: &[f64]) -> BTreeMap<NaiveDate, f64> {
        vals.iter().enumerate().map(|(i, v)| (d(start) + Duration::days(i as i64), *v)).collect()
    }

    #[test]
    fn pearson_known_values() {
        assert!((pearson(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0]).unwrap() - 1.0).abs() < 1e-12);
        assert!((pearson(&[1.0, 2.0, 3.0], &[3.0, 2.0, 1.0]).unwrap() + 1.0).abs() < 1e-12);
        assert_eq!(pearson(&[1.0, 1.0, 1.0], &[1.0, 2.0, 3.0]), None, "no variance → undefined");
        assert_eq!(pearson(&[1.0], &[1.0]), None);
    }

    #[test]
    fn correlation_needs_enough_overlap_and_supports_lag() {
        let a = series("2025-01-01", &(0..30).map(|i| (i % 5) as f64).collect::<Vec<_>>());
        // b repeats a's pattern one day later.
        let b = series("2025-01-02", &(0..30).map(|i| (i % 5) as f64).collect::<Vec<_>>());
        let same_day = correlate(&a, &b, 0, d("2025-01-01"), d("2025-01-30")).unwrap();
        let lagged = correlate(&a, &b, 1, d("2025-01-01"), d("2025-01-30")).unwrap();
        assert!(lagged.r > 0.99 && same_day.r < lagged.r);
        assert_eq!(lagged.strength, "very strong");
        assert!(lagged.caution.contains("does not show"));
        let short = series("2025-01-01", &[1.0, 2.0, 3.0]);
        assert!(correlate(&short, &short, 0, d("2025-01-01"), d("2025-01-03")).is_none());
    }

    #[test]
    fn weekday_profile_and_trend() {
        // 2025-03-02 is a Sunday. Mondays are 10, everything else 1, over 8 weeks.
        let vals: Vec<f64> = (0..56).map(|i| if i % 7 == 1 { 10.0 } else { 1.0 }).collect();
        let m = series("2025-03-02", &vals);
        let ins = compute_insights(&m, d("2025-04-26"));
        assert_eq!(ins.best_weekday, Some(1));
        assert_eq!(ins.weekday_avg[1], Some(10.0));
        assert_eq!(ins.weekday_days[1], 8);
        assert_eq!(ins.busiest.as_ref().unwrap().value, 10.0);
        assert!(ins.change_pct.is_some());
    }

    #[test]
    fn change_between_consecutive_30_day_windows() {
        let mut m = series("2025-03-01", &vec![10.0; 30]);   // 03-01..03-30
        m.extend(series("2025-03-31", &vec![15.0; 30]));      // 03-31..04-29
        let ins = compute_insights(&m, d("2025-04-29"));
        assert_eq!((ins.last30_avg, ins.prev30_avg, ins.change_pct), (Some(15.0), Some(10.0), Some(50.0)));
    }

    #[test]
    fn outliers_are_flagged_only_with_enough_data() {
        let mut vals = vec![5.0; 20];
        vals[10] = 60.0;
        let m = series("2025-03-01", &vals);
        let ins = compute_insights(&m, d("2025-03-20"));
        assert_eq!(ins.outliers.len(), 1);
        assert_eq!((ins.outliers[0].value, ins.outliers[0].direction), (60.0, "high"));
        assert!(compute_insights(&series("2025-03-01", &[1.0, 50.0]), d("2025-03-02")).outliers.is_empty());
        assert_eq!(compute_insights(&BTreeMap::new(), d("2025-03-02")).best_weekday, None);
    }
}
