// Calendar math on plain "YYYY-MM-DD" keys. Everything uses UTC arithmetic internally,
// so results never depend on the host time zone or daylight-saving changes.
export const pad = (n: number) => String(n).padStart(2, "0");
export const dk = (y: number, m: number, d: number) => `${y}-${pad(m + 1)}-${pad(d)}`; // m is 0-based
export const isLeap = (y: number) => (y % 4 === 0 && y % 100 !== 0) || y % 400 === 0;
export const daysInMonth = (y: number, m: number) => [31, isLeap(y) ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][m];
export const parseKey = (k: string) => ({ y: +k.slice(0, 4), m: +k.slice(5, 7) - 1, d: +k.slice(8, 10) });
export const weekday = (k: string) => { const p = parseKey(k); return new Date(Date.UTC(p.y, p.m, p.d)).getUTCDay(); };
export const weekColumn = (k: string, weekStart: number) => (weekday(k) + 7 - weekStart) % 7;
export function addDays(k: string, n: number): string {
  const p = parseKey(k), t = new Date(Date.UTC(p.y, p.m, p.d + n));
  return dk(t.getUTCFullYear(), t.getUTCMonth(), t.getUTCDate());
}
export function todayKey(now = new Date()): string { return dk(now.getFullYear(), now.getMonth(), now.getDate()); } // local date
export const isScheduled = (mask: number, k: string) => ((mask >> weekday(k)) & 1) === 1;

/** Cells for a month grid: leading `null`s (empty spacing) then every real date. */
export function monthGrid(y: number, m: number, weekStart: number): (string | null)[] {
  const first = dk(y, m, 1), cells: (string | null)[] = Array(weekColumn(first, weekStart)).fill(null);
  for (let d = 1; d <= daysInMonth(y, m); d++) cells.push(dk(y, m, d));
  return cells;
}

/** Column-major year grid (7 rows). Index i sits in row i % 7 and column floor(i / 7). */
export function yearGrid(y: number, weekStart: number) {
  const lead = weekColumn(dk(y, 0, 1), weekStart), days = isLeap(y) ? 366 : 365;
  const cells: (string | null)[] = Array(lead).fill(null);
  const labels: { col: number; month: number }[] = [];
  for (let i = 0; i < days; i++) {
    const k = addDays(dk(y, 0, 1), i), p = parseKey(k);
    if (p.d === 1) labels.push({ col: Math.floor(cells.length / 7), month: p.m });
    cells.push(k);
  }
  return { cells, cols: Math.ceil(cells.length / 7), labels };
}
export const monthRange = (y: number, m: number): [string, string] => [dk(y, m, 1), dk(y, m, daysInMonth(y, m))];
