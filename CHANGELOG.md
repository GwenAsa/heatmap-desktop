# Changelog

All notable changes are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added
- Android build (experimental): desktop-only code split into `desktop.rs`/`mobile.rs`, phone layout, GitHub Actions workflow producing a debug-signed APK.

### Fixed
- Studio: palette radio buttons were stretched by the dialog input style.
- Dialogs taller than the window can now scroll, so *Save* is always reachable.

## [0.2.0]

### Added
- **Desktop widgets** in four styles (glass ring, neon month, paper strip, sticky sheet) with pin modes, remembered positions and autostart.
- **Quick capture**: sentence parser with live preview, quick-add window, `--quick` command and `Ctrl+Alt+H` on X11.
- **Flexible goals**: weekly targets, **freeze days**, and weekly streaks.
- **Git tracker**: daily commit counts from local repositories (your own commits only).
- **Insights**: weekday profile, 30-day trend, unusual days, two-tracker correlation with time shift.
- **Studio**: seven palettes, three cell shapes, gap; SVG and PNG export of a year.
- System tray menu, single-instance handling (`--widget`, `--quick`, `--show`, `--background`).
- Database schema v2 (automatic, non-destructive upgrade from v1).

### Fixed
- Quick-capture units written with a space (`2 hours`) are understood.
- Intensity colours in the neon widget could be hidden, leaving unreadable day numbers.

## [0.1.0]
- First release: tracker types, annual and monthly heatmaps with day numbers, SQLite storage with migrations, streaks and schedules with rest days, JSON/CSV import/export, backup, `.deb` and AppImage.
