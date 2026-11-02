# Contributing

Thanks for helping! The short version:

1. Fork, create a branch, make your change.
2. Run the checks locally — they are the same ones CI runs:
   ```bash
   cargo test -p heatmap-core
   npx tsc --noEmit && npm test
   ```
3. Open a pull request describing *why* the change is needed.

## Ground rules
- **Business logic belongs in `core/`** (Rust, no GUI) with tests. The UI only renders and calls commands.
- **Every real date shows its day number** and stays readable (contrast ≥ 4.5:1). Don't remove labels to make things fit.
- **Never rewrite history silently**: goal or schedule changes must not alter recorded values; imports are all-or-nothing.
- Local-first: no network calls for core features, no telemetry.
- New database changes are a **new migration file** in `core/migrations/` (never edit an applied one) plus a test that upgrades a populated older database.

## Regenerating README artwork
```bash
python3 scripts/make-assets.py
```
