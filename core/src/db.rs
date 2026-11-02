use crate::dates::{fmt_date, parse_date};
use crate::error::{invalid, Error, Result};
use crate::model::*;
use crate::stats::{aggregate, compute_stats, level, Stats};
use chrono::NaiveDate;
use rusqlite::{params, Connection, Row};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use uuid::Uuid;

/// Ordered, append-only migrations. `PRAGMA user_version` records how many have been applied.
const MIGRATIONS: &[&str] = &[include_str!("../migrations/0001_init.sql"), include_str!("../migrations/0002_goals_sources.sql")];

pub const TRACKER_COLS: &str = "id,name,kind,unit,goal,color,active_days,start_date,end_date,archived,sort,goal_mode,weekly_target,freezes_per_month";

pub struct Db {
    pub(crate) conn: Connection,
}

pub(crate) fn tracker_row(r: &Row) -> rusqlite::Result<Tracker> {
    let kind: String = r.get(2)?;
    let kind = TrackerType::parse(&kind).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(Tracker {
        id: r.get(0)?, name: r.get(1)?, kind, unit: r.get(3)?, goal: r.get(4)?, color: r.get(5)?,
        active_days: r.get::<_, i64>(6)? as u8, start_date: r.get(7)?, end_date: r.get(8)?,
        archived: r.get::<_, i64>(9)? != 0, sort: r.get(10)?,
        goal_mode: GoalMode::parse(&r.get::<_, String>(11)?), weekly_target: r.get::<_, i64>(12)? as u8, freezes_per_month: r.get::<_, i64>(13)? as u8,
    })
}

pub(crate) fn insert_tracker(c: &Connection, t: &Tracker) -> Result<()> {
    c.execute(
        "INSERT INTO trackers (id,name,kind,unit,goal,color,active_days,start_date,end_date,archived,sort,goal_mode,weekly_target,freezes_per_month)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        params![t.id, t.name, t.kind.as_str(), t.unit, t.goal, t.color, t.active_days as i64,
                t.start_date, t.end_date, t.archived as i64, t.sort, t.goal_mode.as_str(), t.weekly_target as i64, t.freezes_per_month as i64],
    )?;
    Ok(())
}

/// The single place where an entry is validated and inserted (used by logging and both importers).
pub(crate) fn insert_entry(c: &Connection, t: &Tracker, id: Option<String>, date: &str, value: f64, note: &str, source: &str) -> Result<Entry> {
    parse_date(date)?;
    validate_value(t.kind, value)?;
    let note = note.trim();
    if note.chars().count() > 500 { return invalid("Note is longer than 500 characters."); }
    if t.kind == TrackerType::Boolean && value > 0.0 {
        let dup: i64 = c.query_row(
            "SELECT COUNT(*) FROM entries WHERE tracker_id=?1 AND date=?2 AND value>0",
            params![t.id, date], |r| r.get(0))?;
        if dup > 0 { return invalid("Already marked done for this day."); }
    }
    let id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
    c.execute("INSERT INTO entries (id,tracker_id,date,value,note,source) VALUES (?1,?2,?3,?4,?5,?6)",
              params![id, t.id, date, value, note, source])?;
    Ok(Entry { id, tracker_id: t.id.clone(), date: date.into(), value, note: note.into(), source: source.into() })
}

fn group(kind: TrackerType, rows: Vec<(String, f64)>) -> Result<BTreeMap<NaiveDate, (f64, usize)>> {
    let mut m: BTreeMap<NaiveDate, Vec<f64>> = BTreeMap::new();
    for (d, v) in rows { m.entry(parse_date(&d)?).or_default().push(v); }
    Ok(m.into_iter().map(|(d, v)| (d, (aggregate(kind, &v), v.len()))).collect())
}

impl Db {
    pub fn open(path: &Path) -> Result<Db> {
        if let Some(p) = path.parent() { std::fs::create_dir_all(p)?; }
        let conn = Connection::open(path)?;
        conn.query_row("PRAGMA journal_mode=WAL", [], |r| r.get::<_, String>(0))?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        Self::init(conn)
    }

    pub fn open_memory() -> Result<Db> { Self::init(Connection::open_in_memory()?) }

    fn init(conn: Connection) -> Result<Db> {
        conn.pragma_update(None, "foreign_keys", true)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let mut db = Db { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn schema_version(&self) -> Result<i64> {
        Ok(self.conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }

    fn migrate(&mut self) -> Result<()> {
        let v = self.schema_version()?;
        if v as usize > MIGRATIONS.len() {
            return Err(Error::Internal(format!(
                "This database (schema {v}) was created by a newer version of the app (supports {}).", MIGRATIONS.len())));
        }
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(v as usize) {
            let tx = self.conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", (i + 1) as i64)?;
            tx.commit()?;
        }
        Ok(())
    }

    // ---- trackers -------------------------------------------------------------------------
    pub fn list_trackers(&self) -> Result<Vec<Tracker>> {
        let mut st = self.conn.prepare(&format!("SELECT {TRACKER_COLS} FROM trackers ORDER BY sort, rowid"))?;
        let rows = st.query_map([], tracker_row)?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn get_tracker(&self, id: &str) -> Result<Tracker> {
        self.conn.query_row(&format!("SELECT {TRACKER_COLS} FROM trackers WHERE id=?1"), [id], tracker_row)
            .map_err(|e| match e { rusqlite::Error::QueryReturnedNoRows => Error::NotFound("Tracker".into()), e => e.into() })
    }

    pub fn create_tracker(&mut self, inp: TrackerInput) -> Result<Tracker> {
        inp.validate()?;
        let tx = self.conn.transaction()?;
        let sort: i64 = tx.query_row("SELECT COALESCE(MAX(sort),-1)+1 FROM trackers", [], |r| r.get(0))?;
        let t = Tracker {
            id: Uuid::new_v4().to_string(), name: inp.name.trim().into(), kind: inp.kind, unit: inp.unit.trim().into(),
            goal: inp.goal, color: inp.color.to_lowercase(), active_days: inp.active_days,
            start_date: inp.start_date, end_date: inp.end_date, archived: false, sort,
            goal_mode: inp.goal_mode, weekly_target: inp.weekly_target, freezes_per_month: inp.freezes_per_month,
        };
        insert_tracker(&tx, &t)?;
        tx.commit()?;
        Ok(t)
    }

    /// Edits never rewrite recorded values; changing a goal only changes how past days are shaded.
    pub fn update_tracker(&mut self, id: &str, inp: TrackerInput) -> Result<Tracker> {
        let cur = self.get_tracker(id)?;
        inp.validate()?;
        if cur.kind != inp.kind {
            let n: i64 = self.conn.query_row("SELECT COUNT(*) FROM entries WHERE tracker_id=?1", [id], |r| r.get(0))?;
            if n > 0 { return invalid("The type can’t be changed once a tracker has entries."); }
        }
        self.conn.execute(
            "UPDATE trackers SET name=?2,kind=?3,unit=?4,goal=?5,color=?6,active_days=?7,start_date=?8,end_date=?9,
             goal_mode=?10,weekly_target=?11,freezes_per_month=?12,
             updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",
            params![id, inp.name.trim(), inp.kind.as_str(), inp.unit.trim(), inp.goal, inp.color.to_lowercase(),
                    inp.active_days as i64, inp.start_date, inp.end_date, inp.goal_mode.as_str(), inp.weekly_target as i64, inp.freezes_per_month as i64])?;
        self.get_tracker(id)
    }

    pub fn set_archived(&mut self, id: &str, archived: bool) -> Result<()> {
        let n = self.conn.execute("UPDATE trackers SET archived=?2 WHERE id=?1", params![id, archived as i64])?;
        if n == 0 { return Err(Error::NotFound("Tracker".into())); }
        Ok(())
    }

    pub fn delete_tracker(&mut self, id: &str) -> Result<()> {
        let n = self.conn.execute("DELETE FROM trackers WHERE id=?1", [id])?;
        if n == 0 { return Err(Error::NotFound("Tracker".into())); }
        Ok(())
    }

    // ---- entries --------------------------------------------------------------------------
    pub fn add_entry(&mut self, tracker_id: &str, date: &str, value: f64, note: &str) -> Result<Entry> {
        let t = self.get_tracker(tracker_id)?;
        let tx = self.conn.transaction()?;
        let e = insert_entry(&tx, &t, None, date, value, note, "manual")?;
        tx.commit()?;
        Ok(e)
    }

    pub fn get_entry(&self, id: &str) -> Result<Entry> {
        self.conn.query_row("SELECT id,tracker_id,date,value,note,source FROM entries WHERE id=?1", [id], |r| {
            Ok(Entry { id: r.get(0)?, tracker_id: r.get(1)?, date: r.get(2)?, value: r.get(3)?, note: r.get(4)?, source: r.get(5)? })
        }).map_err(|e| match e { rusqlite::Error::QueryReturnedNoRows => Error::NotFound("Entry".into()), e => e.into() })
    }

    pub fn update_entry(&mut self, id: &str, value: f64, note: &str) -> Result<Entry> {
        let cur = self.get_entry(id)?;
        let t = self.get_tracker(&cur.tracker_id)?;
        validate_value(t.kind, value)?;
        let note = note.trim();
        if note.chars().count() > 500 { return invalid("Note is longer than 500 characters."); }
        let tx = self.conn.transaction()?;
        if t.kind == TrackerType::Boolean && value > 0.0 {
            let dup: i64 = tx.query_row(
                "SELECT COUNT(*) FROM entries WHERE tracker_id=?1 AND date=?2 AND value>0 AND id<>?3",
                params![t.id, cur.date, id], |r| r.get(0))?;
            if dup > 0 { return invalid("Already marked done for this day."); }
        }
        tx.execute("UPDATE entries SET value=?2, note=?3 WHERE id=?1", params![id, value, note])?;
        tx.commit()?;
        self.get_entry(id)
    }

    pub fn delete_entry(&mut self, id: &str) -> Result<()> {
        let n = self.conn.execute("DELETE FROM entries WHERE id=?1", [id])?;
        if n == 0 { return Err(Error::NotFound("Entry".into())); }
        Ok(())
    }

    pub fn list_entries(&self, tracker_id: &str, date: &str) -> Result<Vec<Entry>> {
        parse_date(date)?;
        let mut st = self.conn.prepare(
            "SELECT id,tracker_id,date,value,note,source FROM entries WHERE tracker_id=?1 AND date=?2 ORDER BY created_at, rowid")?;
        let rows = st.query_map(params![tracker_id, date], |r| {
            Ok(Entry { id: r.get(0)?, tracker_id: r.get(1)?, date: r.get(2)?, value: r.get(3)?, note: r.get(4)?, source: r.get(5)? })
        })?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    fn rows(&self, tracker_id: &str, from: &str, to: &str) -> Result<Vec<(String, f64)>> {
        let mut st = self.conn.prepare(
            "SELECT date,value FROM entries WHERE tracker_id=?1 AND date BETWEEN ?2 AND ?3 ORDER BY date")?;
        let rows = st.query_map(params![tracker_id, from, to], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// One aggregate per day that has at least one record in `[from, to]`.
    pub fn day_aggregates(&self, tracker_id: &str, from: &str, to: &str) -> Result<Vec<DayAgg>> {
        parse_date(from)?;
        parse_date(to)?;
        let t = self.get_tracker(tracker_id)?;
        let g = group(t.kind, self.rows(tracker_id, from, to)?)?;
        Ok(g.into_iter().map(|(d, (v, n))| DayAgg { date: fmt_date(d), value: v, count: n, level: level(v, t.goal) }).collect())
    }

    pub fn daily_values(&self, tracker_id: &str) -> Result<BTreeMap<NaiveDate, f64>> {
        let t = self.get_tracker(tracker_id)?;
        Ok(group(t.kind, self.rows(tracker_id, "0000-01-01", "9999-12-31")?)?.into_iter().map(|(d, (v, _))| (d, v)).collect())
    }

    pub fn frozen_set(&self, tracker_id: &str) -> Result<BTreeSet<NaiveDate>> {
        let mut st = self.conn.prepare("SELECT date FROM freezes WHERE tracker_id=?1")?;
        let rows = st.query_map([tracker_id], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        rows.iter().map(|d| parse_date(d)).collect()
    }

    pub fn stats(&self, tracker_id: &str, today: &str, week_start: u32) -> Result<Stats> {
        let today = parse_date(today)?;
        let t = self.get_tracker(tracker_id)?;
        Ok(compute_stats(&t, &self.daily_values(tracker_id)?, &self.frozen_set(tracker_id)?, today, week_start))
    }

    // ---- freeze days ----------------------------------------------------------------------
    pub fn list_freezes(&self, tracker_id: &str, from: &str, to: &str) -> Result<Vec<String>> {
        let mut st = self.conn.prepare("SELECT date FROM freezes WHERE tracker_id=?1 AND date BETWEEN ?2 AND ?3 ORDER BY date")?;
        let rows = st.query_map(params![tracker_id, from, to], |r| r.get(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Freeze or unfreeze one day. A month may hold at most `freezes_per_month` frozen days.
    pub fn set_freeze(&mut self, tracker_id: &str, date: &str, on: bool) -> Result<()> {
        parse_date(date)?;
        let t = self.get_tracker(tracker_id)?;
        let tx = self.conn.transaction()?;
        if on {
            let month = format!("{}%", &date[..8]);
            let used: i64 = tx.query_row("SELECT COUNT(*) FROM freezes WHERE tracker_id=?1 AND date LIKE ?2 AND date<>?3",
                params![tracker_id, month, date], |r| r.get(0))?;
            if used >= t.freezes_per_month as i64 {
                return invalid(if t.freezes_per_month == 0 { "This tracker has no freeze days. Enable them in Edit.".to_string() }
                               else { format!("All {} freeze days for this month are used.", t.freezes_per_month) });
            }
            tx.execute("INSERT OR IGNORE INTO freezes(tracker_id,date) VALUES(?1,?2)", params![tracker_id, date])?;
        } else {
            tx.execute("DELETE FROM freezes WHERE tracker_id=?1 AND date=?2", params![tracker_id, date])?;
        }
        tx.commit()?;
        Ok(())
    }

    // ---- automatic sources ----------------------------------------------------------------
    pub fn list_sources(&self, tracker_id: &str) -> Result<Vec<Source>> {
        let mut st = self.conn.prepare("SELECT id,tracker_id,kind,path,last_synced FROM sources WHERE tracker_id=?1 ORDER BY rowid")?;
        let rows = st.query_map([tracker_id], |r| Ok(Source { id: r.get(0)?, tracker_id: r.get(1)?, kind: r.get(2)?, path: r.get(3)?, last_synced: r.get(4)? }))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn remove_source(&mut self, source_id: &str, delete_entries: bool) -> Result<()> {
        let tx = self.conn.transaction()?;
        if delete_entries { tx.execute("DELETE FROM entries WHERE source=?1", [format!("git:{source_id}")])?; }
        let n = tx.execute("DELETE FROM sources WHERE id=?1", [source_id])?;
        if n == 0 { return Err(Error::NotFound("Source".into())); }
        tx.commit()?;
        Ok(())
    }
}
