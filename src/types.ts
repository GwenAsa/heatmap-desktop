export type Kind = "boolean" | "count" | "duration" | "numeric" | "rating";
export interface Tracker {
  id: string; name: string; type: Kind; unit: string; goal: number; color: string;
  activeDays: number; startDate: string | null; endDate: string | null; archived: boolean; sort: number;
  goalMode: "daily" | "weekly"; weeklyTarget: number; freezesPerMonth: number;
}
export type TrackerInput = Omit<Tracker, "id" | "archived" | "sort">;
export interface Entry { id: string; trackerId: string; date: string; value: number; note: string; source: string }
export interface DayAgg { date: string; value: number; count: number; level: number }
export interface Stats {
  recordedDays: number; activeDays: number; zeroDays: number; total: number;
  average: number | null; median: number | null; min: number | null; max: number | null;
  bestStreak: number; currentStreak: number; scheduledDays: number; goalDays: number; goalHitPct: number | null; unit: "days" | "weeks";
}
export interface ImportReport { trackers: number; entries: number; skipped: number }
export interface Capture { trackerId: string; trackerName: string; date: string; value: number; note: string }
export interface Source { id: string; trackerId: string; kind: string; path: string; lastSynced: string | null }
export interface SyncReport { days: number; commits: number }
export interface Outlier { date: string; value: number; direction: "high" | "low" }
export interface Insights {
  recordedDays: number; weekdayAvg: (number | null)[]; weekdayDays: number[]; bestWeekday: number | null;
  last30Avg: number | null; prev30Avg: number | null; changePct: number | null; busiest: Outlier | null; outliers: Outlier[];
}
export interface Correlation { n: number; r: number; lag: number; strength: string; caution: string }
export interface Studio { palette: string; shape: "rounded" | "square" | "round"; gap: number }
