-- Schema version 1. Dates are local calendar dates (YYYY-MM-DD) without a time zone.
CREATE TABLE trackers (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 40),
  kind        TEXT NOT NULL CHECK (kind IN ('boolean','count','duration','numeric','rating')),
  unit        TEXT NOT NULL DEFAULT '' CHECK (length(unit) <= 12),
  goal        REAL NOT NULL CHECK (goal > 0),
  color       TEXT NOT NULL CHECK (color GLOB '#[0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F]'),
  -- Bit n set = scheduled on weekday n (0 = Sunday ... 6 = Saturday). Unset bits are rest days.
  active_days INTEGER NOT NULL DEFAULT 127 CHECK (active_days BETWEEN 1 AND 127),
  start_date  TEXT,
  end_date    TEXT,
  archived    INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0,1)),
  sort        INTEGER NOT NULL DEFAULT 0,
  created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE entries (
  id         TEXT PRIMARY KEY,
  tracker_id TEXT NOT NULL REFERENCES trackers(id) ON DELETE CASCADE,
  date       TEXT NOT NULL CHECK (date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
  value      REAL NOT NULL CHECK (value >= 0),
  note       TEXT NOT NULL DEFAULT '' CHECK (length(note) <= 500),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX idx_entries_tracker_date ON entries(tracker_id, date);
CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
