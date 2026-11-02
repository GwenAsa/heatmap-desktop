//! Settings, JSON/CSV import and export, and database backup.
use crate::db::{insert_entry, insert_tracker, Db};
use crate::error::{invalid, Error, Result};
use crate::model::*;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use uuid::Uuid;

pub const SETTING_KEYS: &[&str] = &["theme", "weekStart", "zoom", "reduceMotion", "view", "selectedTracker", "studio", "widgetsOpen", "widgetCfg", "widgetPrefs"];

fn one() -> u32 { 1 }

#[derive(Serialize, Deserialize, Debug)]
pub struct Backup {
    #[serde(default = "one")] pub version: u32,
    pub trackers: Vec<Tracker>,
    pub entries: Vec<Entry>,
    #[serde(default)] pub settings: BTreeMap<String, String>,
    #[serde(default)] pub freezes: Vec<Freeze>,
    #[serde(default)] pub sources: Vec<Source>,
}

#[derive(Serialize, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport { pub trackers: usize, pub entries: usize, pub skipped: usize }

impl Db {
    pub fn get_settings(&self) -> Result<BTreeMap<String, String>> {
        let mut st = self.conn.prepare("SELECT key,value FROM settings")?;
        let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<BTreeMap<_, _>>>()?;
        Ok(rows)
    }

    pub fn set_setting(&mut self, key: &str, value: &str) -> Result<()> {
        if !SETTING_KEYS.contains(&key) { return invalid(format!("Unknown setting “{key}”.")); }
        if value.len() > 2000 { return invalid("Setting value too long."); }
        self.conn.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                          params![key, value])?;
        Ok(())
    }

    pub fn backup_to(&self, path: &Path) -> Result<()> {
        if path.exists() { return invalid("A file already exists at the chosen backup location."); }
        self.conn.execute("VACUUM INTO ?1", [path.to_string_lossy().as_ref()])?;
        Ok(())
    }

    fn all_entries(&self) -> Result<Vec<Entry>> {
        let mut st = self.conn.prepare("SELECT id,tracker_id,date,value,note,source FROM entries ORDER BY tracker_id,date,created_at,rowid")?;
        let rows = st.query_map([], |r| Ok(Entry { id: r.get(0)?, tracker_id: r.get(1)?, date: r.get(2)?, value: r.get(3)?, note: r.get(4)?, source: r.get(5)? }))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn export_json(&self) -> Result<String> {
        let trackers = self.list_trackers()?;
        let mut freezes = vec![];
        let mut sources = vec![];
        for t in &trackers {
            for date in self.list_freezes(&t.id, "0000-01-01", "9999-12-31")? { freezes.push(Freeze { tracker_id: t.id.clone(), date }); }
            sources.extend(self.list_sources(&t.id)?);
        }
        let b = Backup { version: 1, trackers, entries: self.all_entries()?, settings: self.get_settings()?, freezes, sources };
        Ok(serde_json::to_string_pretty(&b)?)
    }

    /// Replace the entire dataset from a JSON backup (also accepts the legacy browser-prototype export).
    /// Everything is validated and applied in one transaction: on any error nothing changes.
    pub fn import_json(&mut self, text: &str) -> Result<ImportReport> {
        let v: Value = serde_json::from_str(text)?;
        let b: Backup = if v.get("entries").map_or(false, Value::is_object) { legacy(&v)? } else { serde_json::from_value(v)? };
        if b.version != 1 { return invalid(format!("Unsupported backup version {}.", b.version)); }
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM entries", [])?;
        tx.execute("DELETE FROM trackers", [])?;
        let mut by_id: HashMap<String, Tracker> = HashMap::new();
        for (i, t) in b.trackers.iter().enumerate() {
            t.input().validate().map_err(|e| Error::Validation(format!("Tracker {}: {e}", i + 1)))?;
            if t.id.is_empty() || by_id.contains_key(&t.id) { return invalid(format!("Tracker {}: missing or duplicate id.", i + 1)); }
            let mut t = t.clone();
            t.sort = i as i64;
            t.color = t.color.to_lowercase();
            insert_tracker(&tx, &t)?;
            by_id.insert(t.id.clone(), t);
        }
        for (i, e) in b.entries.iter().enumerate() {
            let t = by_id.get(&e.tracker_id).ok_or_else(|| Error::Validation(format!("Entry {}: unknown tracker.", i + 1)))?;
            insert_entry(&tx, t, Some(e.id.clone()), &e.date, e.value, &e.note, &e.source)
                .map_err(|er| Error::Validation(format!("Entry {}: {er}", i + 1)))?;
        }
        for f in &b.freezes {
            if !by_id.contains_key(&f.tracker_id) { return invalid("A freeze day refers to an unknown tracker."); }
            crate::dates::parse_date(&f.date)?;
            tx.execute("INSERT OR IGNORE INTO freezes(tracker_id,date) VALUES(?1,?2)", params![f.tracker_id, f.date])?;
        }
        for sc in &b.sources {
            if !by_id.contains_key(&sc.tracker_id) || sc.kind != "git" { return invalid("A data source is invalid."); }
            tx.execute("INSERT INTO sources(id,tracker_id,kind,path,last_synced) VALUES(?1,?2,?3,?4,?5)", params![sc.id, sc.tracker_id, sc.kind, sc.path, sc.last_synced])?;
        }
        tx.execute("DELETE FROM settings", [])?;
        for (k, val) in &b.settings {
            if SETTING_KEYS.contains(&k.as_str()) && val.len() <= 2000 {
                tx.execute("INSERT INTO settings(key,value) VALUES(?1,?2)", params![k, val])?;
            }
        }
        tx.commit()?;
        Ok(ImportReport { trackers: b.trackers.len(), entries: b.entries.len(), skipped: 0 })
    }

    pub fn export_csv(&self, tracker_id: Option<&str>) -> Result<String> {
        let mut out = String::from("tracker,date,value,note\n");
        let mut st = self.conn.prepare(
            "SELECT t.name,e.date,e.value,e.note FROM entries e JOIN trackers t ON t.id=e.tracker_id
             WHERE (?1 IS NULL OR t.id=?1) ORDER BY t.sort,t.rowid,e.date,e.created_at,e.rowid")?;
        let rows = st.query_map([tracker_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, f64>(2)?, r.get::<_, String>(3)?)))?;
        for row in rows {
            let (n, d, v, note) = row?;
            out.push_str(&format!("{},{},{},{}\n", csv_cell(&n), d, v, csv_cell(&note)));
        }
        Ok(out)
    }

    /// Append entries from CSV (`tracker,date,value,note`). Tracker names must already exist.
    /// Identical existing rows are skipped. All-or-nothing.
    pub fn import_csv(&mut self, text: &str) -> Result<ImportReport> {
        let rows = parse_csv(text.trim_start_matches('\u{feff}'));
        let mut it = rows.into_iter().enumerate();
        let Some((_, header)) = it.next() else { return invalid("The file is empty.") };
        let h: Vec<String> = header.iter().map(|s| s.trim().to_lowercase()).collect();
        if h.len() < 3 || h[0] != "tracker" || h[1] != "date" || h[2] != "value" {
            return invalid("Expected the header: tracker,date,value,note");
        }
        let trackers = self.list_trackers()?;
        let tx = self.conn.transaction()?;
        let mut rep = ImportReport::default();
        for (idx, r) in it {
            if r.iter().all(|c| c.trim().is_empty()) { continue; }
            let n = idx + 1;
            let ctx = |e: Error| Error::Validation(format!("Row {n}: {e}"));
            if r.len() < 3 { return Err(ctx(Error::Validation("expected at least 3 columns".into()))); }
            let name = r[0].trim().to_lowercase();
            let t = trackers.iter().find(|t| t.name.to_lowercase() == name)
                .ok_or_else(|| ctx(Error::Validation(format!("unknown tracker “{}”. Create it first.", r[0].trim()))))?;
            let value: f64 = r[2].trim().parse().map_err(|_| ctx(Error::Validation("value is not a number".into())))?;
            let note = unquote_formula(r.get(3).map_or("", |s| s.as_str()));
            let date = r[1].trim();
            let exists: i64 = tx.query_row(
                "SELECT COUNT(*) FROM entries WHERE tracker_id=?1 AND date=?2 AND value=?3 AND note=?4",
                params![t.id, date, value, note.trim()], |r| r.get(0))?;
            if exists > 0 { rep.skipped += 1; continue; }
            insert_entry(&tx, t, None, date, value, &note, "manual").map_err(ctx)?;
            rep.entries += 1;
        }
        tx.commit()?;
        Ok(rep)
    }
}

fn legacy(v: &Value) -> Result<Backup> {
    let trackers: Vec<Tracker> = serde_json::from_value(v["trackers"].clone())?;
    let mut entries = vec![];
    let bad = || Error::Validation("Unrecognised legacy backup structure.".into());
    for (tid, days) in v["entries"].as_object().ok_or_else(bad)? {
        for (date, list) in days.as_object().ok_or_else(bad)? {
            for e in list.as_array().ok_or_else(bad)? {
                entries.push(Entry {
                    id: Uuid::new_v4().to_string(), tracker_id: tid.clone(), date: date.clone(),
                    value: e["v"].as_f64().ok_or_else(bad)?, note: e["note"].as_str().unwrap_or("").into(), source: "manual".into(),
                });
            }
        }
    }
    Ok(Backup { version: 1, trackers, entries, settings: BTreeMap::new(), freezes: vec![], sources: vec![] })
}

/// Quote for CSV; a leading apostrophe stops spreadsheets from treating text as a formula.
fn csv_cell(s: &str) -> String {
    let s = if s.starts_with(['=', '+', '-', '@', '\t']) { format!("'{s}") } else { s.to_string() };
    if s.contains([',', '"', '\n', '\r']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s }
}

fn unquote_formula(s: &str) -> String {
    match s.strip_prefix('\'') {
        Some(rest) if rest.starts_with(['=', '+', '-', '@', '\t']) => rest.to_string(),
        _ => s.to_string(),
    }
}

pub fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let (mut rows, mut row, mut field, mut quoted) = (vec![], vec![], String::new(), false);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match (quoted, c) {
            (true, '"') => if chars.peek() == Some(&'"') { field.push('"'); chars.next(); } else { quoted = false },
            (true, _) => field.push(c),
            (false, '"') => quoted = true,
            (false, ',') => row.push(std::mem::take(&mut field)),
            (false, '\r') => {}
            (false, '\n') => { row.push(std::mem::take(&mut field)); rows.push(std::mem::take(&mut row)); }
            (false, _) => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() { row.push(field); rows.push(row); }
    rows
}
