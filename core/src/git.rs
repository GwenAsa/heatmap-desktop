//! Git commit tracker: turns the commit history of local repositories into daily counts.
//! Only `git log` is run (read-only), arguments are passed without a shell, and nothing leaves the machine.
use crate::dates::parse_date;
use crate::db::{insert_entry, Db};
use crate::error::{invalid, Error, Result};
use crate::model::{Source, TrackerType};
use rusqlite::params;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use uuid::Uuid;

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport { pub days: usize, pub commits: u32 }

fn git(path: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git").arg("-C").arg(path).args(args).output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound { Error::Validation("git is not installed or not in your PATH.".into()) } else { Error::Io(e) }
    })?;
    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr);
        return invalid(format!("git: {}", msg.trim().chars().take(200).collect::<String>()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn regex_escape(s: &str) -> String {
    s.chars().flat_map(|c| if ".^$*+?()[]{}|\\".contains(c) { vec!['\\', c] } else { vec![c] }).collect()
}

impl Db {
    pub fn add_git_source(&mut self, tracker_id: &str, path: &str) -> Result<Source> {
        let t = self.get_tracker(tracker_id)?;
        if t.kind != TrackerType::Count { return invalid("Git sources can feed Count trackers only."); }
        let p = std::fs::canonicalize(path).map_err(|_| Error::Validation("That folder does not exist.".into()))?;
        if !p.is_dir() { return invalid("Choose a folder, not a file."); }
        git(&p, &["rev-parse", "--is-inside-work-tree"]).map_err(|_| Error::Validation("That folder is not a git repository.".into()))?;
        let s = Source { id: Uuid::new_v4().to_string(), tracker_id: t.id, kind: "git".into(), path: p.to_string_lossy().into_owned(), last_synced: None };
        self.conn.execute("INSERT INTO sources(id,tracker_id,kind,path) VALUES(?1,?2,?3,?4)", params![s.id, s.tracker_id, s.kind, s.path])
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(f, _) if f.code == rusqlite::ErrorCode::ConstraintViolation => Error::Validation("That repository is already connected to this tracker.".into()),
                e => e.into(),
            })?;
        Ok(s)
    }

    /// Re-reads the repository and replaces only the entries this source created; manual entries are untouched.
    /// Commits are counted by local calendar date, for your own git identity only (all branches, merges excluded).
    pub fn sync_source(&mut self, source_id: &str) -> Result<SyncReport> {
        let (tracker_id, path): (String, String) = self.conn
            .query_row("SELECT tracker_id,path FROM sources WHERE id=?1", [source_id], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|e| match e { rusqlite::Error::QueryReturnedNoRows => Error::NotFound("Source".into()), e => e.into() })?;
        let t = self.get_tracker(&tracker_id)?;
        let p = Path::new(&path);
        let email = git(p, &["config", "user.email"]).unwrap_or_default().trim().to_string();
        let author = format!("--author={}", regex_escape(&email));
        let mut args = vec!["log", "--all", "--no-merges", "--date=format-local:%Y-%m-%d", "--pretty=format:%ad"];
        if !email.is_empty() { args.push(&author); }
        let out = git(p, &args)?;
        let mut per_day: BTreeMap<String, u32> = BTreeMap::new();
        for line in out.lines() {
            if parse_date(line.trim()).is_ok() { *per_day.entry(line.trim().to_string()).or_default() += 1; }
        }
        let tag = format!("git:{source_id}");
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM entries WHERE source=?1", [&tag])?;
        let mut commits = 0;
        for (date, n) in &per_day {
            insert_entry(&tx, &t, None, date, *n as f64, &format!("{n} commit{}", if *n == 1 { "" } else { "s" }), &tag)?;
            commits += n;
        }
        tx.execute("UPDATE sources SET last_synced=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1", [source_id])?;
        tx.commit()?;
        Ok(SyncReport { days: per_day.len(), commits })
    }
}
