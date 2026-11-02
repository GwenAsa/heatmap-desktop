//! Quick capture: turns a typed line like `coding 90m yesterday // refactor` into an entry.
//! Pure parsing (no clock, no database): "today" and the tracker list are passed in.
use crate::dates::{fmt_date, parse_date};
use crate::error::{invalid, Result};
use crate::model::{Tracker, TrackerType};
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use serde::Serialize;

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Capture { pub tracker_id: String, pub tracker_name: String, pub date: String, pub value: f64, pub note: String }

fn split_num(tok: &str) -> Option<(f64, String)> {
    let end = tok.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(tok.len());
    if end == 0 { return None; }
    Some((tok[..end].parse().ok()?, tok[end..].to_lowercase()))
}

fn weekday_of(tok: &str) -> Option<Weekday> {
    let t = tok.to_lowercase();
    if t.len() < 3 { return None; }
    [("monday", Weekday::Mon), ("tuesday", Weekday::Tue), ("wednesday", Weekday::Wed), ("thursday", Weekday::Thu),
     ("friday", Weekday::Fri), ("saturday", Weekday::Sat), ("sunday", Weekday::Sun)]
        .iter().find(|(n, _)| n.starts_with(&t)).map(|(_, w)| *w)
}

fn resolve<'a>(tokens: &[&str], trackers: &'a [Tracker]) -> Result<(&'a Tracker, usize)> {
    let live: Vec<&Tracker> = trackers.iter().filter(|t| !t.archived).collect();
    for n in (1..=tokens.len()).rev() {
        let name = tokens[..n].join(" ").to_lowercase();
        if let Some(t) = live.iter().find(|t| t.name.to_lowercase() == name) { return Ok((t, n)); }
    }
    // Otherwise accept an unambiguous prefix of the first word ("cod" -> Coding). Never guess from the middle of a name.
    let first = tokens[0].to_lowercase();
    let hits: Vec<&&Tracker> = live.iter().filter(|t| t.name.to_lowercase().starts_with(&first)).collect();
    match hits.len() {
        1 => return Ok((hits[0], 1)),
        0 => {}
        _ => return invalid(format!("“{}” is ambiguous: {}.", tokens[0], hits.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", "))),
    }
    invalid(format!("No tracker matches “{}”.", tokens[0]))
}

pub fn parse_capture(input: &str, today: NaiveDate, trackers: &[Tracker]) -> Result<Capture> {
    let (main, extra) = input.split_once("//").map_or((input, ""), |(a, b)| (a, b));
    let tokens: Vec<&str> = main.split_whitespace().collect();
    if tokens.is_empty() { return invalid("Type a tracker name, for example: coding 90m yesterday"); }
    let (t, used) = resolve(&tokens, trackers)?;
    let (mut date, mut value, mut words) = (today, None::<f64>, Vec::<&str>::new());
    let rest = &tokens[used..];
    let mut i = 0;
    while i < rest.len() {
        let tok = rest[i];
        let low = tok.to_lowercase();
        // "N days ago"
        if let (Some((n, suf)), Some(w), Some(a)) = (split_num(tok), rest.get(i + 1), rest.get(i + 2)) {
            if suf.is_empty() && ["day", "days"].contains(&w.to_lowercase().as_str()) && a.eq_ignore_ascii_case("ago") {
                date = today - Duration::days(n as i64);
                i += 3;
                continue;
            }
        }
        if let (Some((n, suf)), Some(unit)) = (split_num(tok), rest.get(i + 1)) {
            let u = unit.to_lowercase();
            let hours = ["h", "hr", "hrs", "hour", "hours"].contains(&u.as_str());
            let mins = ["m", "min", "mins", "minute", "minutes"].contains(&u.as_str());
            if suf.is_empty() && (hours || mins) {
                value = Some(if hours && t.kind == TrackerType::Duration { n * 60.0 } else { n });
                i += 2;
                continue;
            }
        }
        match low.as_str() {
            "today" => date = today,
            "yesterday" | "yday" => date = today - Duration::days(1),
            "done" | "yes" | "✓" => value = Some(1.0),
            "skip" | "skipped" | "no" => value = Some(0.0),
            _ => {
                if tok.len() == 10 && tok.as_bytes()[4] == b'-' {
                    date = parse_date(tok)?;
                } else if let Some(wd) = weekday_of(tok) {
                    let back = (today.weekday().num_days_from_monday() + 7 - wd.num_days_from_monday()) % 7;
                    date = today - Duration::days(back as i64);
                } else if let Some(rem) = low.strip_prefix('-').and_then(|r| r.strip_suffix('d')).and_then(|r| r.parse::<i64>().ok()) {
                    date = today - Duration::days(rem);
                } else if let Some((n, suf)) = split_num(tok) {
                    value = Some(match suf.as_str() {
                        "h" | "hr" | "hrs" | "hour" | "hours" if t.kind == TrackerType::Duration => n * 60.0,
                        _ => n,
                    });
                } else { words.push(tok); }
            }
        }
        i += 1;
    }
    let value = match (value, t.kind) {
        (Some(v), _) => v,
        (None, TrackerType::Boolean | TrackerType::Count) => 1.0,
        (None, _) => return invalid(format!("Add a value for “{}”, for example: {} 30", t.name, t.name.to_lowercase())),
    };
    let mut note = words.join(" ");
    if !extra.trim().is_empty() { if !note.is_empty() { note.push(' '); } note.push_str(extra.trim()); }
    Ok(Capture { tracker_id: t.id.clone(), tracker_name: t.name.clone(), date: fmt_date(date), value, note })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::GoalMode;

    fn tr(id: &str, name: &str, kind: TrackerType) -> Tracker {
        Tracker { id: id.into(), name: name.into(), kind, unit: "".into(), goal: 1.0, color: "#3ddc84".into(), active_days: 127,
                  start_date: None, end_date: None, archived: false, sort: 0, goal_mode: GoalMode::Daily, weekly_target: 7, freezes_per_month: 0 }
    }
    fn set() -> Vec<Tracker> {
        vec![tr("c", "Coding", TrackerType::Duration), tr("g", "Gym", TrackerType::Boolean), tr("r", "Reading", TrackerType::Count),
             tr("d", "Deep Work", TrackerType::Duration), tr("w", "Weight", TrackerType::Numeric)]
    }
    fn today() -> NaiveDate { parse_date("2025-10-22").unwrap() } // a Wednesday
    fn p(s: &str) -> Result<Capture> { parse_capture(s, today(), &set()) }

    #[test]
    fn duration_units_and_dates() {
        let c = p("coding 90m yesterday").unwrap();
        assert_eq!((c.tracker_id.as_str(), c.value, c.date.as_str()), ("c", 90.0, "2025-10-21"));
        assert_eq!(p("coding 1.5h").unwrap().value, 90.0);
        assert_eq!(p("Coding 2 hours today").unwrap().value, 120.0);
        assert_eq!(p("coding 45").unwrap().value, 45.0);
    }
    #[test]
    fn relative_and_absolute_dates() {
        assert_eq!(p("reading 2 days ago").unwrap().date, "2025-10-20");
        assert_eq!(p("reading -3d").unwrap().date, "2025-10-19");
        assert_eq!(p("coding 30 mon").unwrap().date, "2025-10-20");
        assert_eq!(p("coding 30 wednesday").unwrap().date, "2025-10-22");
        assert_eq!(p("coding 30 2025-02-28").unwrap().date, "2025-02-28");
        assert!(p("coding 30 2025-02-30").is_err());
    }
    #[test]
    fn boolean_and_count_defaults() {
        assert_eq!(p("gym").unwrap().value, 1.0);
        assert_eq!(p("gym done yesterday").unwrap().value, 1.0);
        assert_eq!(p("gym skip").unwrap().value, 0.0);
        let r = p("reading 2 days ago").unwrap();
        assert_eq!(r.value, 1.0, "\"2 days ago\" is a date, not a value");
        assert_eq!(p("reading 25").unwrap().value, 25.0);
    }
    #[test]
    fn tracker_resolution() {
        assert_eq!(p("deep work 60").unwrap().tracker_id, "d"); // multi-word exact name
        assert_eq!(p("cod 30").unwrap().tracker_id, "c");       // unique prefix
        assert!(p("ork 30").is_err(), "no matching from the middle of a name");
        assert_eq!(p("coding 90 min").unwrap().value, 90.0);
        assert!(p("zzz 30").is_err());
        let both = vec![tr("a", "Cooking", TrackerType::Count), tr("b", "Coding", TrackerType::Count)];
        let e = parse_capture("co 1", today(), &both).unwrap_err().to_string();
        assert!(e.contains("ambiguous") && e.contains("Cooking") && e.contains("Coding"), "{e}");
        let mut arch = set();
        arch[0].archived = true;
        assert!(parse_capture("coding 30", today(), &arch).is_err(), "archived trackers are not matched");
    }
    #[test]
    fn notes_and_missing_values() {
        let c = p("coding 45 mon // refactor parser").unwrap();
        assert_eq!((c.note.as_str(), c.date.as_str()), ("refactor parser", "2025-10-20"));
        assert_eq!(p("coding 45 fixed bug").unwrap().note, "fixed bug");
        assert!(p("coding").unwrap_err().to_string().contains("Add a value"));
        assert!(p("weight").is_err());
        assert!(p("   ").is_err());
    }
}
