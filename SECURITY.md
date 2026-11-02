# Security

Heatmap is local-first: it makes no network requests for its core features and stores data in a local SQLite file.

If you find a vulnerability (for example in import parsing, export writing or the Git integration), please report it **privately** through GitHub's *Security → Report a vulnerability* on this repository rather than a public issue. Include steps to reproduce and the version. You'll get an acknowledgement as soon as possible.

Areas that deserve extra scrutiny: JSON/CSV import, SVG/PNG export paths, the `git` subprocess calls (arguments are passed without a shell), and the per-window Tauri capabilities.
