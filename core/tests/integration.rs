use heatmap_core::io::parse_csv;
use heatmap_core::{Db, Error, GoalMode, TrackerInput, TrackerType};
use rusqlite::Connection;

fn inp(name: &str, kind: TrackerType, goal: f64) -> TrackerInput {
    TrackerInput { name: name.into(), kind, unit: "".into(), goal, color: "#3ddc84".into(), active_days: 127, start_date: None, end_date: None,
      goal_mode: GoalMode::Daily, weekly_target: 7, freezes_per_month: 0 }
}
fn mem() -> Db { Db::open_memory().unwrap() }
fn is_validation<T: std::fmt::Debug>(r: heatmap_core::Result<T>) -> bool { matches!(r, Err(Error::Validation(_))) }

#[test]
fn data_survives_close_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sub/heatmap.db");
    let id = {
        let mut db = Db::open(&path).unwrap();
        let t = db.create_tracker(inp("Reading", TrackerType::Duration, 30.0)).unwrap();
        db.add_entry(&t.id, "2024-02-29", 25.0, "leap day").unwrap();
        db.set_setting("theme", "light").unwrap();
        t.id
    };
    let db = Db::open(&path).unwrap();
    assert_eq!(db.list_trackers().unwrap().len(), 1);
    let e = db.list_entries(&id, "2024-02-29").unwrap();
    assert_eq!((e.len(), e[0].value, e[0].note.as_str()), (1, 25.0, "leap day"));
    assert_eq!(db.get_settings().unwrap()["theme"], "light");
    assert_eq!(db.schema_version().unwrap(), 2);
}

#[test]
fn migration_upgrades_existing_v1_database_without_losing_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.db");
    {
        let c = Connection::open(&path).unwrap();
        c.execute_batch(include_str!("../migrations/0001_init.sql")).unwrap();
        c.execute("INSERT INTO trackers(id,name,kind,goal,color) VALUES('x','Old','count',1,'#112233')", []).unwrap();
        c.execute("INSERT INTO entries(id,tracker_id,date,value) VALUES('e','x','2023-12-31',2)", []).unwrap();
        c.pragma_update(None, "user_version", 1).unwrap();
    }
    let db = Db::open(&path).unwrap(); // v1 -> v2 upgrade keeps every row and fills new columns with defaults
    assert_eq!(db.schema_version().unwrap(), 2);
    let e = &db.list_entries("x", "2023-12-31").unwrap()[0];
    assert_eq!((e.value, e.source.as_str()), (2.0, "manual"));
    let t = db.get_tracker("x").unwrap();
    assert_eq!((t.goal_mode, t.weekly_target, t.freezes_per_month), (GoalMode::Daily, 7, 0));
    drop(db);
    assert_eq!(Db::open(&path).unwrap().schema_version().unwrap(), 2, "re-opening must not re-run migrations");
}

#[test]
fn fresh_database_is_migrated_and_newer_database_is_refused() {
    assert_eq!(mem().schema_version().unwrap(), 2);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("future.db");
    Connection::open(&path).unwrap().pragma_update(None, "user_version", 99).unwrap();
    assert!(Db::open(&path).is_err());
}

#[test]
fn multiple_entries_aggregate_per_type_and_zero_differs_from_missing() {
    let mut db = mem();
    let dur = db.create_tracker(inp("Study", TrackerType::Duration, 60.0)).unwrap();
    let rate = db.create_tracker(inp("Mood", TrackerType::Rating, 4.0)).unwrap();
    for v in [30.0, 45.0, 15.0] { db.add_entry(&dur.id, "2025-10-02", v, "").unwrap(); }
    db.add_entry(&dur.id, "2025-10-03", 0.0, "rest").unwrap();
    for v in [2.0, 4.0] { db.add_entry(&rate.id, "2025-10-02", v, "").unwrap(); }

    let a = db.day_aggregates(&dur.id, "2025-10-01", "2025-10-31").unwrap();
    assert_eq!(a.len(), 2, "2025-10-01 has no record and must be absent");
    assert_eq!((a[0].date.as_str(), a[0].value, a[0].count, a[0].level), ("2025-10-02", 90.0, 3, 4));
    assert_eq!((a[1].value, a[1].level), (0.0, 0), "explicit zero is present with level 0");
    let r = db.day_aggregates(&rate.id, "2025-10-01", "2025-10-31").unwrap();
    assert_eq!(r[0].value, 3.0, "ratings are averaged");
}

#[test]
fn range_query_respects_month_and_year_boundaries() {
    let mut db = mem();
    let t = db.create_tracker(inp("C", TrackerType::Count, 1.0)).unwrap();
    for d in ["2024-12-31", "2025-01-01", "2025-01-31", "2025-02-01"] { db.add_entry(&t.id, d, 1.0, "").unwrap(); }
    let jan: Vec<_> = db.day_aggregates(&t.id, "2025-01-01", "2025-01-31").unwrap().into_iter().map(|a| a.date).collect();
    assert_eq!(jan, ["2025-01-01", "2025-01-31"]);
}

#[test]
fn validation_rejects_bad_input() {
    let mut db = mem();
    assert!(is_validation(db.create_tracker(inp("  ", TrackerType::Count, 1.0))));
    assert!(is_validation(db.create_tracker(inp("A", TrackerType::Count, 0.0))));
    assert!(is_validation(db.create_tracker(inp("A", TrackerType::Count, f64::NAN))));
    let mut bad = inp("A", TrackerType::Count, 1.0);
    bad.color = "green".into();
    assert!(is_validation(db.create_tracker(bad)));
    let mut bad = inp("A", TrackerType::Count, 1.0);
    bad.active_days = 0;
    assert!(is_validation(db.create_tracker(bad)));

    let c = db.create_tracker(inp("C", TrackerType::Count, 1.0)).unwrap();
    assert!(is_validation(db.add_entry(&c.id, "2025-02-30", 1.0, "")));
    assert!(is_validation(db.add_entry(&c.id, "2025-02-03", -1.0, "")));
    assert!(is_validation(db.add_entry(&c.id, "2025-02-03", f64::INFINITY, "")));
    assert!(is_validation(db.add_entry(&c.id, "2025-02-03", 1.0, &"x".repeat(501))));
    let r = db.create_tracker(inp("R", TrackerType::Rating, 4.0)).unwrap();
    assert!(is_validation(db.add_entry(&r.id, "2025-02-03", 6.0, "")));
    assert!(db.add_entry("missing", "2025-02-03", 1.0, "").is_err());
    assert!(db.list_trackers().unwrap().len() == 2 && db.list_entries(&c.id, "2025-02-03").unwrap().is_empty());
}

#[test]
fn boolean_cannot_be_done_twice_and_type_is_locked_after_entries() {
    let mut db = mem();
    let b = db.create_tracker(inp("Gym", TrackerType::Boolean, 1.0)).unwrap();
    let e = db.add_entry(&b.id, "2025-05-05", 1.0, "").unwrap();
    assert!(is_validation(db.add_entry(&b.id, "2025-05-05", 1.0, "")));
    assert!(is_validation(db.update_entry(&e.id, 0.5, "")));
    db.update_entry(&e.id, 0.0, "skipped").unwrap();
    db.add_entry(&b.id, "2025-05-05", 1.0, "").unwrap(); // allowed again because the first is now 0
    assert!(is_validation(db.update_tracker(&b.id, inp("Gym", TrackerType::Count, 1.0))));
    assert!(db.update_tracker(&b.id, inp("Gym 2", TrackerType::Boolean, 1.0)).is_ok());
}

#[test]
fn changing_goal_does_not_rewrite_history() {
    let mut db = mem();
    let t = db.create_tracker(inp("Run", TrackerType::Numeric, 5.0)).unwrap();
    db.add_entry(&t.id, "2025-06-01", 4.0, "").unwrap();
    assert_eq!(db.day_aggregates(&t.id, "2025-06-01", "2025-06-01").unwrap()[0].level, 3);
    db.update_tracker(&t.id, inp("Run", TrackerType::Numeric, 4.0)).unwrap();
    let a = &db.day_aggregates(&t.id, "2025-06-01", "2025-06-01").unwrap()[0];
    assert_eq!((a.value, a.level), (4.0, 4));
}

#[test]
fn deleting_tracker_cascades_and_entries_can_be_edited_and_deleted() {
    let mut db = mem();
    let t = db.create_tracker(inp("C", TrackerType::Count, 1.0)).unwrap();
    let e = db.add_entry(&t.id, "2025-01-01", 1.0, "a").unwrap();
    assert_eq!(db.update_entry(&e.id, 3.0, " b ").unwrap().note, "b");
    db.delete_entry(&e.id).unwrap();
    assert!(matches!(db.delete_entry(&e.id), Err(Error::NotFound(_))));
    db.add_entry(&t.id, "2025-01-02", 1.0, "").unwrap();
    db.delete_tracker(&t.id).unwrap();
    assert!(db.list_trackers().unwrap().is_empty());
    let n: i64 = db.stats("x", "2025-01-01", 0).map(|_| 0).unwrap_or(1);
    assert_eq!(n, 1, "stats for a deleted tracker is an error");
}

#[test]
fn stats_use_schedule_and_today() {
    let mut db = mem();
    let mut i = inp("Work", TrackerType::Count, 1.0);
    i.active_days = 0b0111110; // Mon–Fri
    let t = db.create_tracker(i).unwrap();
    for d in ["2025-03-06", "2025-03-07", "2025-03-10"] { db.add_entry(&t.id, d, 1.0, "").unwrap(); }
    let s = db.stats(&t.id, "2025-03-10", 0).unwrap();
    assert_eq!((s.best_streak, s.current_streak, s.goal_hit_pct), (3, 3, Some(100.0)));
    assert!(is_validation(db.stats(&t.id, "10/03/2025", 0)));
}

#[test]
fn json_roundtrip_preserves_everything() {
    let mut a = mem();
    let t = a.create_tracker(inp("Read, \"daily\"", TrackerType::Duration, 30.0)).unwrap();
    a.add_entry(&t.id, "2025-01-01", 20.0, "multi\nline, \"quoted\"").unwrap();
    a.add_entry(&t.id, "2025-01-01", 15.0, "").unwrap();
    a.set_setting("zoom", "40").unwrap();
    let json = a.export_json().unwrap();
    let mut b = mem();
    let rep = b.import_json(&json).unwrap();
    assert_eq!((rep.trackers, rep.entries), (1, 2));
    assert_eq!(b.list_trackers().unwrap(), a.list_trackers().unwrap());
    assert_eq!(b.list_entries(&t.id, "2025-01-01").unwrap().len(), 2);
    assert_eq!(b.get_settings().unwrap()["zoom"], "40");
}

#[test]
fn failed_import_changes_nothing() {
    let mut db = mem();
    let t = db.create_tracker(inp("Keep", TrackerType::Count, 1.0)).unwrap();
    db.add_entry(&t.id, "2025-01-01", 1.0, "").unwrap();
    let bad = r##"{"version":1,"trackers":[{"id":"a","name":"New","type":"count","goal":1,"color":"#112233"}],
      "entries":[{"id":"1","trackerId":"a","date":"2025-01-01","value":1},{"id":"2","trackerId":"a","date":"2025-13-01","value":1}]}"##;
    assert!(is_validation(db.import_json(bad)));
    assert!(db.import_json("not json").is_err());
    let trackers = db.list_trackers().unwrap();
    assert_eq!((trackers.len(), trackers[0].name.as_str()), (1, "Keep"));
    assert_eq!(db.list_entries(&t.id, "2025-01-01").unwrap().len(), 1);
}

#[test]
fn legacy_browser_prototype_backup_can_be_imported() {
    let legacy = r##"{"trackers":[{"id":"t1","name":"Coding","type":"duration","unit":"min","goal":60,"color":"#3ddc84"}],
      "entries":{"t1":{"2025-10-01":[{"id":"a","v":30,"note":"x"},{"id":"b","v":45,"note":""}]}},"view":"month","y":2025}"##;
    let mut db = mem();
    assert_eq!(db.import_json(legacy).unwrap().entries, 2);
    assert_eq!(db.day_aggregates("t1", "2025-10-01", "2025-10-31").unwrap()[0].value, 75.0);
}

#[test]
fn csv_export_import_roundtrip_with_escaping_and_dedup() {
    let mut a = mem();
    let t = a.create_tracker(inp("Notes, etc", TrackerType::Count, 1.0)).unwrap();
    a.add_entry(&t.id, "2025-04-01", 2.0, "has, comma and \"quotes\"").unwrap();
    a.add_entry(&t.id, "2025-04-02", 1.0, "=SUM(A1)").unwrap();
    let csv = a.export_csv(None).unwrap();
    assert!(csv.contains("'=SUM(A1)"), "formula-like notes are neutralised");
    let mut b = mem();
    b.create_tracker(inp("notes, ETC", TrackerType::Count, 1.0)).unwrap(); // case-insensitive match
    let rep = b.import_csv(&csv).unwrap();
    assert_eq!((rep.entries, rep.skipped), (2, 0));
    let id = b.list_trackers().unwrap()[0].id.clone();
    assert_eq!(b.list_entries(&id, "2025-04-01").unwrap()[0].note, "has, comma and \"quotes\"");
    assert_eq!(b.list_entries(&id, "2025-04-02").unwrap()[0].note, "=SUM(A1)");
    let again = b.import_csv(&csv).unwrap();
    assert_eq!((again.entries, again.skipped), (0, 2), "re-import skips identical rows");
}

#[test]
fn csv_import_is_all_or_nothing_and_reports_the_row() {
    let mut db = mem();
    db.create_tracker(inp("A", TrackerType::Count, 1.0)).unwrap();
    let r = db.import_csv("tracker,date,value,note\nA,2025-01-01,1,ok\nA,2025-01-02,abc,bad\n");
    assert!(matches!(&r, Err(Error::Validation(m)) if m.starts_with("Row 3")), "{r:?}");
    assert!(is_validation(db.import_csv("tracker,date,value,note\nNope,2025-01-01,1,\n")));
    assert!(is_validation(db.import_csv("wrong,header\n")));
    let id = db.list_trackers().unwrap()[0].id.clone();
    assert!(db.list_entries(&id, "2025-01-01").unwrap().is_empty());
}

#[test]
fn csv_parser_handles_crlf_quotes_and_embedded_newlines() {
    let rows = parse_csv("a,b\r\n\"x\"\"y\",\"l1\nl2\"\r\nlast,row");
    assert_eq!(rows, vec![vec!["a", "b"], vec!["x\"y", "l1\nl2"], vec!["last", "row"]]);
}

#[test]
fn backup_file_is_a_valid_standalone_database() {
    let mut db = mem();
    let t = db.create_tracker(inp("B", TrackerType::Count, 1.0)).unwrap();
    db.add_entry(&t.id, "2025-01-01", 1.0, "").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("backup.db");
    db.backup_to(&p).unwrap();
    assert!(is_validation(db.backup_to(&p)), "never overwrite an existing file");
    let restored = Db::open(&p).unwrap();
    assert_eq!(restored.list_entries(&t.id, "2025-01-01").unwrap().len(), 1);
}

#[test]
fn settings_are_whitelisted() {
    let mut db = mem();
    assert!(is_validation(db.set_setting("evil", "1")));
    db.set_setting("weekStart", "1").unwrap();
    db.set_setting("weekStart", "0").unwrap();
    assert_eq!(db.get_settings().unwrap()["weekStart"], "0");
}
