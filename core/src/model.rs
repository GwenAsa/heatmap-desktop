use crate::dates::parse_date;
use crate::error::{invalid, Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TrackerType { Boolean, Count, Duration, Numeric, Rating }

impl TrackerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Boolean => "boolean", Self::Count => "count", Self::Duration => "duration",
            Self::Numeric => "numeric", Self::Rating => "rating",
        }
    }
    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "boolean" => Self::Boolean, "count" => Self::Count, "duration" => Self::Duration,
            "numeric" => Self::Numeric, "rating" => Self::Rating,
            _ => return Err(Error::Validation(format!("Unknown tracker type “{s}”"))),
        })
    }
}

fn all_days() -> u8 { 127 }
fn seven() -> u8 { 7 }
fn manual() -> String { "manual".into() }

/// Daily: every scheduled day should be done. Weekly: reach `weekly_target` qualifying days per week.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum GoalMode { #[default] Daily, Weekly }
impl GoalMode {
    pub fn as_str(&self) -> &'static str { match self { Self::Daily => "daily", Self::Weekly => "weekly" } }
    pub fn parse(s: &str) -> Self { if s == "weekly" { Self::Weekly } else { Self::Daily } }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tracker {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: TrackerType,
    #[serde(default)] pub unit: String,
    pub goal: f64,
    pub color: String,
    #[serde(default = "all_days")] pub active_days: u8,
    #[serde(default)] pub start_date: Option<String>,
    #[serde(default)] pub end_date: Option<String>,
    #[serde(default)] pub archived: bool,
    #[serde(default)] pub sort: i64,
    #[serde(default)] pub goal_mode: GoalMode,
    #[serde(default = "seven")] pub weekly_target: u8,
    #[serde(default)] pub freezes_per_month: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TrackerInput {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: TrackerType,
    #[serde(default)] pub unit: String,
    pub goal: f64,
    pub color: String,
    #[serde(default = "all_days")] pub active_days: u8,
    #[serde(default)] pub start_date: Option<String>,
    #[serde(default)] pub end_date: Option<String>,
    #[serde(default)] pub goal_mode: GoalMode,
    #[serde(default = "seven")] pub weekly_target: u8,
    #[serde(default)] pub freezes_per_month: u8,
}

impl TrackerInput {
    pub fn validate(&self) -> Result<()> {
        let n = self.name.trim().chars().count();
        if n == 0 || n > 40 { return invalid("Name must be 1–40 characters."); }
        if self.unit.trim().chars().count() > 12 { return invalid("Unit must be at most 12 characters."); }
        if !(self.goal.is_finite() && self.goal > 0.0 && self.goal < 1e9) {
            return invalid("Goal must be a number greater than zero.");
        }
        let c = self.color.as_bytes();
        if c.len() != 7 || c[0] != b'#' || !c[1..].iter().all(u8::is_ascii_hexdigit) {
            return invalid("Color must look like #3ddc84.");
        }
        if !(1..=127).contains(&self.active_days) { return invalid("Select at least one scheduled weekday."); }
        if !(1..=7).contains(&self.weekly_target) { return invalid("Weekly target must be between 1 and 7 days."); }
        if self.freezes_per_month > 10 { return invalid("Freeze days per month: at most 10."); }
        let s = self.start_date.as_deref().map(parse_date).transpose()?;
        let e = self.end_date.as_deref().map(parse_date).transpose()?;
        if let (Some(s), Some(e)) = (s, e) {
            if e < s { return invalid("End date is before the start date."); }
        }
        Ok(())
    }
}

impl Tracker {
    pub fn input(&self) -> TrackerInput {
        TrackerInput {
            name: self.name.clone(), kind: self.kind, unit: self.unit.clone(), goal: self.goal,
            color: self.color.clone(), active_days: self.active_days,
            start_date: self.start_date.clone(), end_date: self.end_date.clone(),
            goal_mode: self.goal_mode, weekly_target: self.weekly_target, freezes_per_month: self.freezes_per_month,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub tracker_id: String,
    pub date: String,
    pub value: f64,
    #[serde(default)] pub note: String,
    #[serde(default = "manual")] pub source: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Source { pub id: String, pub tracker_id: String, pub kind: String, pub path: String, #[serde(default)] pub last_synced: Option<String> }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Freeze { pub tracker_id: String, pub date: String }

/// Per-day aggregate returned to the heatmap. `level` is 0 for an explicit zero, 1–4 otherwise.
/// Days without records are simply absent (missing data is not the same as zero).
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DayAgg { pub date: String, pub value: f64, pub count: usize, pub level: u8 }

pub fn validate_value(kind: TrackerType, v: f64) -> Result<()> {
    if !v.is_finite() || v < 0.0 || v > 1e9 { return invalid("Value must be a non-negative number."); }
    match kind {
        TrackerType::Boolean if v != 0.0 && v != 1.0 => invalid("A done/not-done value must be 0 or 1."),
        TrackerType::Rating if v != 0.0 && !(1.0..=5.0).contains(&v) => invalid("Rating must be between 1 and 5."),
        _ => Ok(()),
    }
}
