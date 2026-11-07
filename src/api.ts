// All persistence goes through the Rust backend (SQLite). There is no browser-storage fallback.
import { invoke } from "@tauri-apps/api/core";
import type { Capture, Correlation, DayAgg, Entry, ImportReport, Insights, Source, Stats, SyncReport, Tracker, TrackerInput } from "./types";

export const api = {
  listTrackers: () => invoke<Tracker[]>("list_trackers"),
  createTracker: (input: TrackerInput) => invoke<Tracker>("create_tracker", { input }),
  updateTracker: (id: string, input: TrackerInput) => invoke<Tracker>("update_tracker", { id, input }),
  deleteTracker: (id: string) => invoke<void>("delete_tracker", { id }),
  dayAggregates: (trackerId: string, from: string, to: string) => invoke<DayAgg[]>("day_aggregates", { trackerId, from, to }),
  listEntries: (trackerId: string, date: string) => invoke<Entry[]>("list_entries", { trackerId, date }),
  addEntry: (trackerId: string, date: string, value: number, note: string) => invoke<Entry>("add_entry", { trackerId, date, value, note }),
  updateEntry: (id: string, value: number, note: string) => invoke<Entry>("update_entry", { id, value, note }),
  deleteEntry: (id: string) => invoke<void>("delete_entry", { id }),
  stats: (trackerId: string, today: string, weekStart: number) => invoke<Stats>("get_stats", { trackerId, today, weekStart }),
  getSettings: () => invoke<Record<string, string>>("get_settings"),
  setSetting: (key: string, value: string) => invoke<void>("set_setting", { key, value }),
  exportToFile: (kind: "json" | "csv", path: string, trackerId?: string) => invoke<void>("export_to_file", { kind, path, trackerId: trackerId ?? null }),
  importFromFile: (path: string) => invoke<ImportReport>("import_from_file", { path }),
  listFreezes: (trackerId: string, from: string, to: string) => invoke<string[]>("list_freezes", { trackerId, from, to }),
  setFreeze: (trackerId: string, date: string, on: boolean) => invoke<void>("set_freeze", { trackerId, date, on }),
  listSources: (trackerId: string) => invoke<Source[]>("list_sources", { trackerId }),
  addGitSource: (trackerId: string, path: string) => invoke<Source>("add_git_source", { trackerId, path }),
  syncSource: (sourceId: string) => invoke<SyncReport>("sync_source", { sourceId }),
  removeSource: (sourceId: string, deleteEntries: boolean) => invoke<void>("remove_source", { sourceId, deleteEntries }),
  capturePreview: (text: string, today: string) => invoke<Capture>("capture_preview", { text, today }),
  quickCapture: (text: string, today: string) => invoke<Capture>("quick_capture", { text, today }),
  insights: (trackerId: string, today: string) => invoke<Insights>("get_insights", { trackerId, today }),
  correlation: (a: string, b: string, lag: number, days: number, today: string) => invoke<Correlation | null>("get_correlation", { a, b, lag, days, today }),
  writeExport: (path: string, text?: string, dataUrl?: string) => invoke<void>("write_export", { path, text: text ?? null, dataUrl: dataUrl ?? null }),
  openWidget: (kind: string) => invoke<void>("open_widget", { kind }),
  closeWidget: (kind: string) => invoke<void>("close_widget", { kind }),
  setWidgetMode: (kind: string, mode: string) => invoke<void>("set_widget_mode", { kind, mode }),
  listOpenWidgets: () => invoke<string[]>("list_open_widgets"),
  openQuick: () => invoke<void>("open_quick"),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (on: boolean) => invoke<void>("set_autostart", { on }),
  backupDb: (path: string) => invoke<void>("backup_db", { path }),
};

export const errMsg = (e: unknown): string => (typeof e === "string" ? e : e instanceof Error ? e.message : "Something went wrong.");
