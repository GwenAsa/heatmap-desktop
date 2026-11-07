import { memo, useMemo, type KeyboardEvent } from "react";
import { shade } from "../lib/color";
import { addDays, isScheduled, monthGrid, yearGrid } from "../lib/dates";
import { fmtValue } from "../lib/format";
import type { DayAgg, Tracker } from "../types";

export interface GridProps {
  tracker: Tracker; aggs: Map<string, DayAgg>; today: string; selected: string;
  weekStart: number; bgHex: string; onSelect: (date: string, focus?: boolean) => void;
  /** Optional studio palette (levels 1–4); defaults to the tracker colour. */
  palette?: { bg: string; fg: string }[];
  frozen?: Set<string>;
}
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const WD = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
export const weekdayLabels = (ws: number) => [...Array(7)].map((_, i) => WD[(i + ws) % 7]);

interface CellProps { date: string; cls: string; bg?: string; fg?: string; tip: string; sel: boolean; tab: boolean; onSelect: GridProps["onSelect"] }

/** Memoised: a selection change re-renders only the two cells whose props actually change. */
const Cell = memo(function Cell({ date, cls, bg, fg, tip, sel, tab, onSelect }: CellProps) {
  return (
    <button type="button" className={cls} style={bg ? { background: bg, color: fg } : undefined}
      data-date={date} title={tip} aria-label={tip} aria-pressed={sel} tabIndex={tab ? 0 : -1}
      onClick={() => onSelect(date)}>
      {Number(date.slice(8))}
    </button>
  );
});

function useCells(p: GridProps, dates: (string | null)[]) {
  const { tracker, aggs, today, selected, bgHex, onSelect, frozen } = p;
  const own = useMemo(() => [1, 2, 3, 4].map((l) => shade(tracker.color, l, bgHex)), [tracker.color, bgHex]);
  const palette = p.palette ?? own;
  const tabDate = dates.includes(selected) ? selected : dates.find(Boolean) ?? "";
  return dates.map((d, i) => {
    if (!d) return <span key={`b${i}`} className="cell blank" aria-hidden="true" />;
    const a = aggs.get(d), rest = !isScheduled(tracker.activeDays, d);
    const c = ["cell"];
    if (a && a.level > 0) c.push("fill");
    if (a && a.level === 0) c.push("zero");
    if (a) c.push("has");
    if (rest && !a) c.push("rest");
    if (frozen?.has(d)) c.push("frz");
    if (d === today) c.push("today");
    if (d === selected) c.push("sel");
    if (d > today) c.push("fut");
    const tip = `${d}: ${fmtValue(tracker, a?.value)}${a && a.level === 0 ? " (recorded zero)" : ""}${rest ? " (rest day)" : ""}${frozen?.has(d) ? " (frozen: streak protected)" : ""}${d > today ? " (future)" : ""}${d === today ? " (today)" : ""}`;
    const s = a && a.level > 0 ? palette[a.level - 1] : undefined;
    return <Cell key={d} date={d} cls={c.join(" ")} bg={s?.bg} fg={s?.fg} tip={tip} sel={d === selected} tab={d === tabDate} onSelect={onSelect} />;
  });
}

function onKey(e: KeyboardEvent, onSelect: GridProps["onSelect"]) {
  const step = ({ ArrowLeft: -1, ArrowRight: 1, ArrowUp: -7, ArrowDown: 7 } as Record<string, number>)[e.key];
  const cell = (e.target as HTMLElement).closest<HTMLElement>("[data-date]");
  if (!step || !cell?.dataset.date) return;
  e.preventDefault();
  onSelect(addDays(cell.dataset.date, step), true);
}

export function MonthView(p: GridProps & { year: number; month: number }) {
  const dates = useMemo(() => monthGrid(p.year, p.month, p.weekStart), [p.year, p.month, p.weekStart]);
  const cells = useCells(p, dates);
  return (
    <div role="group" aria-label="Month calendar">
      <div className="wd">{weekdayLabels(p.weekStart).map((x) => <span key={x}>{x}</span>)}</div>
      <div className="mg" data-testid="grid" onKeyDown={(e) => onKey(e, p.onSelect)}>{cells}</div>
    </div>
  );
}

export function YearView(p: GridProps & { year: number }) {
  const g = useMemo(() => yearGrid(p.year, p.weekStart), [p.year, p.weekStart]);
  const cells = useCells(p, g.cells);
  return (
    <div className="yw" role="group" aria-label="Year calendar">
      <div className="yd">{weekdayLabels(p.weekStart).map((x) => <span key={x}>{x}</span>)}</div>
      <div>
        <div className="yl" style={{ gridTemplateColumns: `repeat(${g.cols}, var(--cs))` }}>
          {g.labels.map((l) => <span key={l.month} style={{ gridColumn: l.col + 1, gridRow: 1 }}>{MONTHS[l.month]}</span>)}
        </div>
        <div className="ys" data-testid="grid" style={{ gridTemplateColumns: `repeat(${g.cols}, var(--cs))` }} onKeyDown={(e) => onKey(e, p.onSelect)}>{cells}</div>
      </div>
    </div>
  );
}
