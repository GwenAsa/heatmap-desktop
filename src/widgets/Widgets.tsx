import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, errMsg } from "../api";
import { shade } from "../lib/color";
import { addDays, dk, monthGrid, parseKey, todayKey, weekColumn } from "../lib/dates";
import { fmtValue, round2 } from "../lib/format";
import type { DayAgg, Stats, Tracker } from "../types";

type Prefs = Record<string, { tracker?: string }>;
const MODES = ["top", "normal", "bottom"] as const;
const MODE_ICON = { top: "📌", normal: "▫", bottom: "⬇" } as const;
const MODE_TIP = { top: "Always on top", normal: "Normal window", bottom: "On the desktop, behind other windows" } as const;
const MONTHS = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];

/** One-tap actions for logging today from a widget. */
export function quickActions(t: Tracker): { label: string; value: number }[] {
  switch (t.type) {
    case "boolean": return [{ label: "✓ Done", value: 1 }];
    case "count": return [{ label: "+1", value: 1 }, { label: "+2", value: 2 }, { label: "+5", value: 5 }];
    case "duration": return [{ label: "+15m", value: 15 }, { label: "+30m", value: 30 }, { label: "+60m", value: 60 }];
    case "rating": return [1, 2, 3, 4, 5].map((n) => ({ label: `★${n}`, value: n }));
    default: return [{ label: "+1", value: 1 }, { label: "+10", value: 10 }];
  }
}

function useWidget(kind: string) {
  const [trackers, setTrackers] = useState<Tracker[]>([]);
  const [prefs, setPrefs] = useState<Prefs>({});
  const [weekStart, setWeekStart] = useState(0);
  const [mode, setMode] = useState<(typeof MODES)[number]>("top");
  const [tick, setTick] = useState(0);
  const [today, setToday] = useState(todayKey());
  const [error, setError] = useState("");

  useEffect(() => {
    let live = true;
    Promise.all([api.listTrackers(), api.getSettings()]).then(([ts, s]) => {
      if (!live) return;
      setTrackers(ts);
      try { setPrefs(JSON.parse(s.widgetPrefs ?? "{}")); } catch { setPrefs({}); }
      try { const m = JSON.parse(s.widgetCfg ?? "{}")[kind]?.mode; if (MODES.includes(m)) setMode(m); } catch { /* default */ }
      setWeekStart(s.weekStart === "1" ? 1 : 0);
      setError("");
    }).catch((e) => live && setError(errMsg(e)));
    return () => { live = false; };
  }, [tick, kind]);

  useEffect(() => {
    const un = listen("data-changed", () => setTick((t) => t + 1));
    const iv = setInterval(() => { setToday(todayKey()); setTick((t) => t + 1); }, 60_000);
    return () => { un.then((f) => f()); clearInterval(iv); };
  }, []);

  const tracker = trackers.find((t) => t.id === prefs[kind]?.tracker) ?? trackers[0];
  const cycle = useCallback((n: number) => {
    if (trackers.length < 2 || !tracker) return;
    const i = (trackers.findIndex((t) => t.id === tracker.id) + n + trackers.length) % trackers.length;
    const next = { ...prefs, [kind]: { tracker: trackers[i].id } };
    setPrefs(next);
    api.setSetting("widgetPrefs", JSON.stringify(next)).catch(() => undefined);
  }, [trackers, tracker, prefs, kind]);
  const nextMode = () => {
    const m = MODES[(MODES.indexOf(mode) + 1) % MODES.length];
    setMode(m);
    api.setWidgetMode(kind, m).catch((e) => setError(errMsg(e)));
  };
  return { trackers, tracker, cycle, mode, nextMode, today, tick, weekStart, error, setError };
}

function Chrome(p: { kind: string; title: string; w: ReturnType<typeof useWidget>; switcher?: boolean; children: ReactNode }) {
  const { w } = p;
  return (
    <div className={`w w-${p.kind}`} data-tauri-drag-region>
      <div className="wbar" data-tauri-drag-region>
        {p.switcher && <button aria-label="Previous tracker" onClick={() => w.cycle(-1)}>‹</button>}
        <span className="wname" data-tauri-drag-region>{p.switcher ? w.tracker?.name ?? p.title : p.title}</span>
        {p.switcher && <button aria-label="Next tracker" onClick={() => w.cycle(1)}>›</button>}
        <span className="grow" data-tauri-drag-region />
        <button title={MODE_TIP[w.mode]} aria-label={MODE_TIP[w.mode]} onClick={w.nextMode}>{MODE_ICON[w.mode]}</button>
        <button title="Close widget" aria-label="Close widget" onClick={() => api.closeWidget(p.kind).catch((e) => w.setError(errMsg(e)))}>✕</button>
      </div>
      {w.error && <div className="werr" role="alert">{w.error}</div>}
      {p.children}
    </div>
  );
}

function Empty() { return <div className="wempty">Open Heatmap and create a tracker first.</div>; }

function useToday(t: Tracker | undefined, today: string, tick: number, weekStart: number) {
  const [agg, setAgg] = useState<DayAgg | undefined>();
  const [stats, setStats] = useState<Stats | null>(null);
  const [lastId, setLastId] = useState<string | undefined>();
  useEffect(() => {
    if (!t) return;
    let live = true;
    Promise.all([api.dayAggregates(t.id, today, today), api.stats(t.id, today, weekStart), api.listEntries(t.id, today)]).then(([a, s, e]) => {
      if (!live) return;
      setAgg(a[0]); setStats(s); setLastId(e.filter((x) => x.source === "manual").slice(-1)[0]?.id);
    }).catch(() => undefined);
    return () => { live = false; };
  }, [t?.id, today, tick, weekStart]); // eslint-disable-line react-hooks/exhaustive-deps
  return { agg, stats, lastId };
}

function useLog(w: ReturnType<typeof useWidget>) {
  return (t: Tracker, date: string, value: number) => api.addEntry(t.id, date, value, "").catch((e) => w.setError(errMsg(e)));
}

// 1 — Glass ring: today's progress at a glance, one-tap logging.
export function RingWidget() {
  const w = useWidget("ring"), t = w.tracker, d = useToday(t, w.today, w.tick, w.weekStart), log = useLog(w);
  if (!t) return <Chrome kind="ring" title="Today" w={w}><Empty /></Chrome>;
  const value = d.agg?.value ?? 0, pct = Math.min(1, value / t.goal), C = 2 * Math.PI * 78;
  const light = shade(t.color, 2, "#ffffff").bg;
  return (
    <Chrome kind="ring" title="Today" w={w} switcher>
      <div className="ringbox" data-tauri-drag-region>
        <svg viewBox="0 0 200 200" width="190" height="190" role="img" aria-label={`${round2(value)} of ${t.goal}`}>
          <defs><linearGradient id="rg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stopColor={light} /><stop offset="1" stopColor={t.color} /></linearGradient></defs>
          <circle cx="100" cy="100" r="78" fill="none" stroke="rgba(255,255,255,.1)" strokeWidth="14" />
          <circle cx="100" cy="100" r="78" fill="none" stroke="url(#rg)" strokeWidth="14" strokeLinecap="round" transform="rotate(-90 100 100)"
            strokeDasharray={`${C * pct} ${C}`} style={{ transition: "stroke-dasharray .5s cubic-bezier(.2,.7,.3,1)" }} />
          <text x="100" y="98" textAnchor="middle" className="rv">{pct >= 1 ? "✓" : round2(value)}</text>
          <text x="100" y="124" textAnchor="middle" className="rs">{pct >= 1 ? "goal reached" : `of ${t.goal}${t.type === "duration" ? " min" : t.unit ? " " + t.unit : ""}`}</text>
        </svg>
      </div>
      <div className="rstreak">🔥 {d.stats?.currentStreak ?? 0} {d.stats?.unit ?? "days"} <small>· best {d.stats?.bestStreak ?? 0}</small></div>
      <div className="wactions">
        {quickActions(t).map((a) => <button key={a.label} onClick={() => log(t, w.today, a.value)}>{a.label}</button>)}
        {d.lastId && <button className="ghost" title="Remove your last entry today" onClick={() => api.deleteEntry(d.lastId!).catch((e) => w.setError(errMsg(e)))}>↶</button>}
      </div>
    </Chrome>
  );
}

// 2 — Neon terminal: a month grid you can click through.
export function MonthWidget() {
  const w = useWidget("month"), t = w.tracker, log = useLog(w);
  const [ym, setYm] = useState(() => ({ y: parseKey(todayKey()).y, m: parseKey(todayKey()).m }));
  const [sel, setSel] = useState(todayKey());
  const [aggs, setAggs] = useState<Map<string, DayAgg>>(new Map());
  const from = dk(ym.y, ym.m, 1), to = dk(ym.y, ym.m, new Date(ym.y, ym.m + 1, 0).getDate());
  useEffect(() => {
    if (!t) return;
    let live = true;
    api.dayAggregates(t.id, from, to).then((a) => live && setAggs(new Map(a.map((x) => [x.date, x])))).catch(() => undefined);
    return () => { live = false; };
  }, [t?.id, from, to, w.tick]); // eslint-disable-line react-hooks/exhaustive-deps
  const pal = useMemo(() => (t ? [1, 2, 3, 4].map((l) => shade(t.color, l, "#05060a")) : []), [t]);
  const move = (n: number) => { const d = new Date(Date.UTC(ym.y, ym.m + n, 1)); setYm({ y: d.getUTCFullYear(), m: d.getUTCMonth() }); };
  if (!t) return <Chrome kind="month" title="MONTH" w={w}><Empty /></Chrome>;
  const dates = monthGrid(ym.y, ym.m, w.weekStart);
  const names = [...Array(7)].map((_, i) => "SMTWTFS"[(i + w.weekStart) % 7]);
  const act = quickActions(t)[0];
  return (
    <Chrome kind="month" title="MONTH" w={w} switcher>
      <div className="mnav" data-tauri-drag-region>
        <button onClick={() => move(-1)} aria-label="Previous month">‹</button>
        <span data-tauri-drag-region>&gt; {MONTHS[ym.m]}_{ym.y}</span>
        <button onClick={() => move(1)} aria-label="Next month">›</button>
      </div>
      <div className="mgrid" data-tauri-drag-region>
        {names.map((n, i) => <span key={i} className="mwd">{n}</span>)}
        {dates.map((d, i) => {
          if (!d) return <span key={`b${i}`} />;
          const a = aggs.get(d), s = a && a.level > 0 ? pal[a.level - 1] : undefined;
          const cls = ["mc", s ? "fill" : "", a && a.level === 4 ? "glow" : "", a?.level === 0 ? "zero" : "", d === w.today ? "today" : "", d === sel ? "sel" : ""].join(" ");
          return <button key={d} className={cls} style={s ? { background: s.bg, color: s.fg, ["--g" as string]: t.color } : undefined}
            aria-label={`${d}: ${fmtValue(t, a?.value)}`} onClick={() => setSel(d)}>{parseKey(d).d}</button>;
        })}
      </div>
      <div className="mfoot">
        <span>[{sel}] {fmtValue(t, aggs.get(sel)?.value)}</span>
        <button onClick={() => log(t, sel, act.value)} title={`Log ${act.label} on ${sel}`}>[{act.label}]</button>
      </div>
    </Chrome>
  );
}

// 3 — Mono paper strip: the last 14 weeks, every day numbered.
export function StripWidget() {
  const w = useWidget("strip"), t = w.tracker, d = useToday(t, w.today, w.tick, w.weekStart), log = useLog(w);
  const weeks = 14;
  const first = addDays(w.today, -weekColumn(w.today, w.weekStart) - (weeks - 1) * 7), last = addDays(first, weeks * 7 - 1);
  const [aggs, setAggs] = useState<Map<string, DayAgg>>(new Map());
  useEffect(() => {
    if (!t) return;
    let live = true;
    api.dayAggregates(t.id, first, last).then((a) => live && setAggs(new Map(a.map((x) => [x.date, x])))).catch(() => undefined);
    return () => { live = false; };
  }, [t?.id, first, last, w.tick]); // eslint-disable-line react-hooks/exhaustive-deps
  const pal = useMemo(() => (t ? [1, 2, 3, 4].map((l) => shade(t.color, l, "#f4f1ea")) : []), [t]);
  if (!t) return <Chrome kind="strip" title="STRIP" w={w}><Empty /></Chrome>;
  const cells = Array.from({ length: weeks * 7 }, (_, i) => addDays(first, Math.floor(i / 7) * 7 + (i % 7))); // column-major
  const act = quickActions(t)[0];
  return (
    <Chrome kind="strip" title="STRIP" w={w} switcher>
      <div className="sstats" data-tauri-drag-region>
        <span><b>{d.stats?.currentStreak ?? 0}</b> {d.stats?.unit ?? "days"} streak</span>
        <span><b>{d.stats?.goalHitPct === null || d.stats === null ? "–" : Math.round(d.stats!.goalHitPct!) + "%"}</b> goal hit</span>
        <span className="grow" />
        <button onClick={() => log(t, w.today, act.value)}>Log today {act.label}</button>
      </div>
      <div className="sgrid" data-tauri-drag-region>
        {cells.map((date) => {
          const a = aggs.get(date), s = a && a.level > 0 ? pal[a.level - 1] : undefined;
          return <span key={date} className={`sc${a?.level === 0 ? " zero" : ""}${date === w.today ? " today" : ""}${date > w.today ? " fut" : ""}`}
            style={s ? { background: s.bg, color: s.fg } : undefined} title={`${date}: ${fmtValue(t, a?.value)}`}>{parseKey(date).d}</span>;
        })}
      </div>
    </Chrome>
  );
}

// 4 — Paper sheet: every tracker for today, plus a type-to-log line.
export function SheetWidget() {
  const w = useWidget("sheet"), log = useLog(w);
  const [rows, setRows] = useState<{ t: Tracker; v: number; streak: number; unit: string }[]>([]);
  const [text, setText] = useState("");
  const [msg, setMsg] = useState("");
  useEffect(() => {
    let live = true;
    Promise.all(w.trackers.map(async (t) => {
      const [a, s] = await Promise.all([api.dayAggregates(t.id, w.today, w.today), api.stats(t.id, w.today, w.weekStart)]);
      return { t, v: a[0]?.value ?? 0, streak: s.currentStreak, unit: s.unit };
    })).then((r) => live && setRows(r)).catch(() => undefined);
    return () => { live = false; };
  }, [w.trackers, w.today, w.tick, w.weekStart]);
  const submit = async () => {
    if (!text.trim()) return;
    try { const c = await api.quickCapture(text, w.today); setMsg(`Logged ${c.trackerName}: ${fmtValue(w.trackers.find((t) => t.id === c.trackerId)!, c.value)} · ${c.date}`); setText(""); }
    catch (e) { setMsg(errMsg(e)); }
  };
  const day = parseKey(w.today);
  return (
    <Chrome kind="sheet" title={new Date(day.y, day.m, day.d).toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" })} w={w}>
      {rows.length === 0 && <Empty />}
      <ul className="slist" data-tauri-drag-region>
        {rows.map(({ t, v, streak, unit }) => {
          const done = v >= t.goal, a = quickActions(t)[0];
          return (
            <li key={t.id} className={done ? "done" : ""}>
              <i style={{ background: t.color }} />
              <div className="sinfo"><b>{t.name}</b><small>{fmtValue(t, v || undefined)} · 🔥{streak}{unit === "weeks" ? "w" : "d"}</small>
                <span className="sbar"><span style={{ width: `${Math.min(100, (v / t.goal) * 100)}%`, background: t.color }} /></span></div>
              <button disabled={t.type === "boolean" && done} onClick={() => log(t, w.today, a.value)}>{t.type === "boolean" && done ? "✓" : a.label}</button>
            </li>
          );
        })}
      </ul>
      <div className="sinput">
        <input value={text} onChange={(e) => setText(e.target.value)} onKeyDown={(e) => e.key === "Enter" && submit()} placeholder="coding 90m yesterday…" aria-label="Quick add" />
      </div>
      {msg && <div className="smsg" role="status">{msg}</div>}
    </Chrome>
  );
}
