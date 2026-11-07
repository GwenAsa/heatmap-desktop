use heatmap_core::capture::parse_capture;
use heatmap_core::dates::parse_date;
use heatmap_core::{Db, Error, GoalMode, TrackerInput, TrackerType};
use std::process::Command;

fn inp(name: &str, kind: TrackerType) -> TrackerInput {
    TrackerInput { name: name.into(), kind, unit: "".into(), goal: 1.0, color: "#3ddc84".into(), active_days: 127,
                   start_date: None, end_date: None, goal_mode: GoalMode::Daily, weekly_target: 7, freezes_per_month: 0 }
}
fn mem() -> Db { Db::open_memory().unwrap() }

#[test]
fn freeze_days_are_limited_per_month_and_keep_the_streak_alive() {
    let mut db = mem();
    let mut i = inp("Run", TrackerType::Count);
    i.freezes_per_month = 1;
    let t = db.create_tracker(i).unwrap();
    for d in ["2025-03-01", "2025-03-02", "2025-03-04", "2025-03-05"] { db.add_entry(&t.id, d, 1.0, "").unwrap(); }
    assert_eq!(db.stats(&t.id, "2025-03-05", 0).unwrap().current_streak, 2);
    db.set_freeze(&t.id, "2025-03-03", true).unwrap();
    assert_eq!(db.stats(&t.id, "2025-03-05", 0).unwrap().current_streak, 4);
    let e = db.set_freeze(&t.id, "2025-03-10", true).unwrap_err().to_string();
    assert!(e.contains("freeze days for this month are used"), "{e}");
    db.set_freeze(&t.id, "2025-04-01", true).unwrap(); // a new month has its own allowance
    db.set_freeze(&t.id, "2025-03-03", true).unwrap(); // re-freezing the same day is idempotent
    db.set_freeze(&t.id, "2025-03-03", false).unwrap();
    assert_eq!(db.list_freezes(&t.id, "2025-03-01", "2025-03-31").unwrap().len(), 0);
    db.set_freeze(&t.id, "2025-03-10", true).unwrap();
}

#[test]
fn freezing_is_refused_when_a_tracker_has_none() {
    let mut db = mem();
    let t = db.create_tracker(inp("A", TrackerType::Count)).unwrap();
    assert!(matches!(db.set_freeze(&t.id, "2025-03-03", true), Err(Error::Validation(m)) if m.contains("no freeze days")));
}

#[test]
fn weekly_goal_is_stored_validated_and_used_by_stats() {
    let mut db = mem();
    let mut i = inp("Gym", TrackerType::Boolean);
    i.goal_mode = GoalMode::Weekly;
    i.weekly_target = 3;
    let t = db.create_tracker(i.clone()).unwrap();
    assert_eq!((t.goal_mode, t.weekly_target), (GoalMode::Weekly, 3));
    for d in ["2025-03-03", "2025-03-05", "2025-03-07"] { db.add_entry(&t.id, d, 1.0, "").unwrap(); }
    let s = db.stats(&t.id, "2025-03-09", 1).unwrap();
    assert_eq!((s.unit.as_str(), s.current_streak, s.best_streak), ("weeks", 1, 1));
    i.weekly_target = 8;
    assert!(db.update_tracker(&t.id, i).is_err());
}

#[test]
fn quick_capture_resolves_against_stored_trackers() {
    let mut db = mem();
    db.create_tracker(inp("Coding", TrackerType::Duration)).unwrap();
    let c = parse_capture("coding 1.5h yesterday // parser", parse_date("2025-10-22").unwrap(), &db.list_trackers().unwrap()).unwrap();
    db.add_entry(&c.tracker_id, &c.date, c.value, &c.note).unwrap();
    let e = db.list_entries(&c.tracker_id, "2025-10-21").unwrap();
    assert_eq!((e[0].value, e[0].note.as_str()), (90.0, "parser"));
}

fn git(dir: &std::path::Path, args: &[&str], date: Option<&str>) {
    let mut c = Command::new("git");
    c.arg("-C").arg(dir).args(args);
    if let Some(d) = date { c.env("GIT_AUTHOR_DATE", d).env("GIT_COMMITTER_DATE", d); }
    assert!(c.output().unwrap().status.success(), "git {args:?} failed");
}

#[test]
fn git_source_counts_your_commits_per_day_idempotently() {
    if Command::new("git").arg("--version").output().is_err() { eprintln!("git missing: skipped"); return; }
    let repo = tempfile::tempdir().unwrap();
    let p = repo.path();
    git(p, &["init", "-q"], None);
    git(p, &["config", "user.email", "me@example.dev"], None);
    git(p, &["config", "user.name", "Me"], None);
    git(p, &["commit", "-q", "--allow-empty", "-m", "a"], Some("2025-03-01T12:00:00"));
    git(p, &["commit", "-q", "--allow-empty", "-m", "b"], Some("2025-03-01T15:00:00"));
    git(p, &["commit", "-q", "--allow-empty", "-m", "c"], Some("2025-03-02T12:00:00"));
    git(p, &["commit", "-q", "--allow-empty", "-m", "other", "--author=Other <other@example.dev>"], Some("2025-03-03T12:00:00"));

    let mut db = mem();
    let t = db.create_tracker(inp("Commits", TrackerType::Count)).unwrap();
    db.add_entry(&t.id, "2025-03-01", 1.0, "manual note").unwrap();
    let src = db.add_git_source(&t.id, p.to_str().unwrap()).unwrap();
    assert!(db.add_git_source(&t.id, p.to_str().unwrap()).is_err(), "duplicate source");
    for _ in 0..2 {
        let r = db.sync_source(&src.id).unwrap();
        assert_eq!((r.days, r.commits), (2, 3), "the other author's commit is excluded");
    }
    let day1 = db.list_entries(&t.id, "2025-03-01").unwrap();
    assert_eq!(day1.len(), 2, "manual entry kept next to the synced one");
    assert_eq!(db.day_aggregates(&t.id, "2025-03-01", "2025-03-03").unwrap().iter().map(|a| a.value).collect::<Vec<_>>(), [3.0, 1.0]);
    assert!(db.list_entries(&t.id, "2025-03-03").unwrap().is_empty());
    assert!(db.list_sources(&t.id).unwrap()[0].last_synced.is_some());

    // The source survives a JSON backup round trip (with its generated entries tagged).
    let json = db.export_json().unwrap();
    let mut b = mem();
    b.import_json(&json).unwrap();
    assert_eq!(b.list_sources(&t.id).unwrap().len(), 1);
    assert!(b.list_entries(&t.id, "2025-03-01").unwrap().iter().any(|e| e.source.starts_with("git:")));

    db.remove_source(&src.id, true).unwrap();
    let left = db.list_entries(&t.id, "2025-03-01").unwrap();
    assert_eq!((left.len(), left[0].note.as_str()), (1, "manual note"));
}

#[test]
fn git_source_validation() {
    let mut db = mem();
    let c = db.create_tracker(inp("C", TrackerType::Count)).unwrap();
    let d = db.create_tracker(inp("D", TrackerType::Duration)).unwrap();
    let plain = tempfile::tempdir().unwrap();
    assert!(db.add_git_source(&d.id, plain.path().to_str().unwrap()).is_err(), "needs a Count tracker");
    assert!(db.add_git_source(&c.id, "/definitely/not/here").is_err());
    if Command::new("git").arg("--version").output().is_ok() {
        assert!(db.add_git_source(&c.id, plain.path().to_str().unwrap()).is_err(), "not a repository");
    }
    assert!(matches!(db.sync_source("nope"), Err(Error::NotFound(_))));
}

#[test]
fn json_backup_keeps_freezes() {
    let mut a = mem();
    let mut i = inp("F", TrackerType::Count);
    i.freezes_per_month = 2;
    let t = a.create_tracker(i).unwrap();
    a.set_freeze(&t.id, "2025-05-05", true).unwrap();
    let mut b = mem();
    b.import_json(&a.export_json().unwrap()).unwrap();
    assert_eq!(b.list_freezes(&t.id, "2025-05-01", "2025-05-31").unwrap(), ["2025-05-05"]);
    assert_eq!(b.get_tracker(&t.id).unwrap().freezes_per_month, 2);
}

#[test]
fn insights_run_on_stored_data() {
    let mut db = mem();
    let t = db.create_tracker(inp("C", TrackerType::Count)).unwrap();
    for (d, v) in [("2025-03-03", 5.0), ("2025-03-10", 7.0), ("2025-03-04", 1.0)] { db.add_entry(&t.id, d, v, "").unwrap(); }
    let ins = heatmap_core::insights::compute_insights(&db.daily_values(&t.id).unwrap(), parse_date("2025-03-12").unwrap());
    assert_eq!((ins.recorded_days, ins.best_weekday), (3, Some(1)));
}
