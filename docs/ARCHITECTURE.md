# Architecture

```
core/            heatmap-core: Rust library, no GUI. Everything testable lives here.
  migrations/    append-only SQL migrations (PRAGMA user_version)
  src/dates.rs   calendar maths, weekday alignment, schedules
  src/stats.rs   aggregation, intensity levels, streaks (daily + weekly + freezes), statistics
  src/db.rs      trackers, entries, freezes, sources; every write validated, multi-step writes in transactions
  src/io.rs      JSON/CSV import and export, backup, settings whitelist
  src/capture.rs quick-capture sentence parser
  src/insights.rs weekday profile, trend, outliers, correlation
  src/git.rs     read-only git log -> daily counts
src-tauri/       thin shell: IPC commands, windows (widgets, quick add), tray, shortcut, single instance
src/             React + TypeScript; one bundle serves the main window (#/), widgets (#/w/<kind>) and quick add (#/quick)
scripts/         artwork generator
```

## Principles
- **Dates are timezone-free** `YYYY-MM-DD` strings. The UI passes "today", so DST and time zones cannot move a day.
- **Missing ≠ zero.** Days without records are absent; an explicit zero is a record (dashed cell).
- **One validation path.** Logging, JSON import and CSV import all go through the same insert function.
- **All-or-nothing imports.** JSON import runs in a single transaction after validating everything; an automatic safety copy is written first.
- **Rest days and freezes never break streaks.** Weekly goals count qualifying days per week; the unfinished week never breaks a streak.
- **Windows stay in sync** through a `data-changed` event emitted after every mutation.
- **Least privilege.** One capability file lists exactly the window permissions widgets need.

## Data flow
UI → `invoke("command")` → Tauri command → `heatmap-core` → SQLite (WAL). Mutating commands emit `data-changed`; every open window reloads what it shows.
