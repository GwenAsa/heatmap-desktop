import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { confirm, open, save } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { api, errMsg } from "./api";
import { MonthView, YearView } from "./components/Heatmap";
import Insights from "./components/Insights";
import Studio from "./components/Studio";
import Tools from "./components/Tools";
import { DEFAULT_STUDIO, SHAPES, levelStyles, parseStudio } from "./lib/theme";
import { svgToPng, yearSvg } from "./lib/svg";
import { dk, monthRange, parseKey, todayKey } from "./lib/dates";
import { fmtValue, round2 } from "./lib/format";
import type { DayAgg, Entry, Kind, Stats, Studio as StudioCfg, Tracker, TrackerInput } from "./types";

const MN = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const DAY_NAMES = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
const BLANK: TrackerInput = { name: "", type: "boolean", unit: "", goal: 1, color: "#3ddc84", activeDays: 127, startDate: null, endDate: null, goalMode: "daily", weeklyTarget: 7, freezesPerMonth: 0 };

function useBg(theme: string) {
  const [bg, setBg] = useState("#0a0a0b");
  useEffect(() => {
    const read = () => setBg(getComputedStyle(document.documentElement).getPropertyValue("--bg").trim() || "#0a0a0b");
    document.documentElement.dataset.theme = theme;
    read();
    const mq = matchMedia("(prefers-color-scheme: dark)");
    mq.addEventListener("change", read);
    return () => mq.removeEventListener("change", read);
  }, [theme]);
  return bg;
}

export default function App() {
  const [ready, setReady] = useState(false);
  const [trackers, setTrackers] = useState<Tracker[]>([]);
  const [selId, setSelId] = useState("");
  const [view, setView] = useState<"month" | "year">("month");
  const [today, setToday] = useState(todayKey());
  const [day, setDay] = useState(todayKey());
  const [year, setYear] = useState(parseKey(todayKey()).y);
  const [month, setMonth] = useState(parseKey(todayKey()).m);
  const [weekStart, setWeekStart] = useState(0);
  const [cs, setCs] = useState(56);
  const [theme, setTheme] = useState("");
  const [reduceMotion, setReduceMotion] = useState(false);
  const [version, setVersion] = useState(0);
  const [aggs, setAggs] = useState<Map<string, DayAgg>>(new Map());
  const [entries, setEntries] = useState<Entry[]>([]);
  const [stats, setStats] = useState<Stats | null>(null);
  const [toast, setToast] = useState<{ msg: string; err: boolean } | null>(null);
  const [studio, setStudio] = useState<StudioCfg>(DEFAULT_STUDIO);
  const [frozen, setFrozen] = useState<Set<string>>(new Set());
  const [showStudio, setShowStudio] = useState(false);
  const [showTools, setShowTools] = useState(false);
  const [quickText, setQuickText] = useState("");
  const [form, setForm] = useState<{ id: string | null; v: TrackerInput }>({ id: null, v: BLANK });
  const dlg = useRef<HTMLDialogElement>(null);
  const focusAfter = useRef(false);
  const bgHex = useBg(theme);
  const tracker = trackers.find((t) => t.id === selId) ?? trackers[0];

  const say = useCallback((msg: string, err = false) => {
    setToast({ msg, err });
    window.clearTimeout((say as unknown as { h?: number }).h);
    (say as unknown as { h?: number }).h = window.setTimeout(() => setToast(null), err ? 5000 : 2400);
  }, []);
  const fail = useCallback((e: unknown) => say(errMsg(e), true), [say]);
  const put = useCallback((k: string, v: string) => { api.setSetting(k, v).catch(fail); }, [fail]);
  const refresh = useCallback(() => setVersion((v) => v + 1), []);

  // ---- initial load from SQLite ----
  useEffect(() => {
    Promise.all([api.listTrackers(), api.getSettings()]).then(([ts, s]) => {
      setTrackers(ts);
      setSelId(ts.find((t) => t.id === s.selectedTracker)?.id ?? ts[0]?.id ?? "");
      if (s.view === "year" || s.view === "month") setView(s.view);
      if (s.weekStart === "1") setWeekStart(1);
      if (s.zoom && +s.zoom >= 28 && +s.zoom <= 64) setCs(+s.zoom);
      if (s.theme === "light" || s.theme === "dark") setTheme(s.theme);
      setReduceMotion(s.reduceMotion === "1");
      setStudio(parseStudio(s.studio));
      setReady(true);
    }).catch(fail);
  }, [fail]);

  useEffect(() => { document.documentElement.style.setProperty("--cs", `${cs}px`); }, [cs]);
  useEffect(() => { document.documentElement.classList.toggle("rm", reduceMotion); }, [reduceMotion]);
  useEffect(() => {
    document.documentElement.style.setProperty("--gap", `${studio.gap}px`);
    document.documentElement.style.setProperty("--rf", String(SHAPES[studio.shape]));
  }, [studio]);
  // Widgets and quick add run in other windows; any change they make refreshes this one.
  useEffect(() => {
    const un = listen("data-changed", () => { api.listTrackers().then(setTrackers).catch(fail); refresh(); });
    return () => { un.then((f) => f()); };
  }, [fail, refresh]);
  useEffect(() => { const i = setInterval(() => setToday(todayKey()), 60_000); return () => clearInterval(i); }, []);

  // ---- data for the visible range, the selected day and the statistics ----
  const [from, to] = view === "month" ? monthRange(year, month) : [dk(year, 0, 1), dk(year, 11, 31)];
  useEffect(() => {
    if (!tracker) return;
    let live = true;
    api.dayAggregates(tracker.id, from, to).then((a) => live && setAggs(new Map(a.map((x) => [x.date, x])))).catch(fail);
    api.listFreezes(tracker.id, from, to).then((f) => live && setFrozen(new Set(f))).catch(fail);
    return () => { live = false; };
  }, [tracker?.id, from, to, version, fail]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    if (!tracker) return;
    let live = true;
    api.listEntries(tracker.id, day).then((e) => live && setEntries(e)).catch(fail);
    api.stats(tracker.id, today, weekStart).then((s) => live && setStats(s)).catch(fail);
    return () => { live = false; };
  }, [tracker?.id, day, today, weekStart, version, fail]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    if (!focusAfter.current) return;
    focusAfter.current = false;
    document.querySelector<HTMLElement>(`[data-date="${day}"]`)?.focus({ preventScroll: false });
  });

  const select = useCallback((d: string, focus = false) => {
    focusAfter.current = focus;
    setDay(d);
    const p = parseKey(d);
    setYear(p.y);
    setMonth(p.m);
  }, []);

  // ---- actions (all persisted through the backend) ----
  const run = async (f: () => Promise<unknown>, ok?: string) => { try { await f(); refresh(); if (ok) say(ok); } catch (e) { fail(e); } };
  const nav = (n: number) => {
    if (view === "year") setYear(n ? year + n : parseKey(today).y);
    else if (!n) { const p = parseKey(today); setYear(p.y); setMonth(p.m); }
    else { const t = new Date(Date.UTC(year, month + n, 1)); setYear(t.getUTCFullYear()); setMonth(t.getUTCMonth()); }
  };
  const openForm = (t?: Tracker) => {
    setForm(t ? { id: t.id, v: { name: t.name, type: t.type, unit: t.unit, goal: t.goal, color: t.color, activeDays: t.activeDays, startDate: t.startDate, endDate: t.endDate, goalMode: t.goalMode, weeklyTarget: t.weeklyTarget, freezesPerMonth: t.freezesPerMonth } } : { id: null, v: BLANK });
    dlg.current?.showModal();
  };
  const saveForm = async () => {
    try {
      const t = form.id ? await api.updateTracker(form.id, form.v) : await api.createTracker(form.v);
      setTrackers(await api.listTrackers());
      setSelId(t.id); put("selectedTracker", t.id);
      dlg.current?.close(); refresh();
    } catch (e) { fail(e); }
  };
  const removeTracker = async () => {
    if (!tracker || !(await confirm(`Delete “${tracker.name}” and all of its history? Export a backup first if unsure.`, { title: "Delete tracker", kind: "warning" }))) return;
    try {
      await api.deleteTracker(tracker.id);
      const ts = await api.listTrackers();
      setTrackers(ts); setSelId(ts[0]?.id ?? ""); setAggs(new Map()); setEntries([]); setStats(null); refresh();
    } catch (e) { fail(e); }
  };
  const doExport = async (kind: "json" | "csv") => {
    const path = await save({ defaultPath: kind === "json" ? "heatmap-backup.json" : "heatmap.csv", filters: [{ name: kind.toUpperCase(), extensions: [kind] }] });
    if (path) await run(() => api.exportToFile(kind, path), "Exported");
  };
  const doImport = async () => {
    const path = await open({ multiple: false, filters: [{ name: "Backup (JSON) or entries (CSV)", extensions: ["json", "csv"] }] });
    if (typeof path !== "string") return;
    const isJson = !path.toLowerCase().endsWith(".csv");
    if (isJson && !(await confirm("Importing a JSON backup REPLACES all current trackers and entries. A safety copy of the current database is saved automatically first. Continue?", { title: "Replace all data", kind: "warning" }))) return;
    try {
      const r = await api.importFromFile(path);
      const ts = await api.listTrackers();
      setTrackers(ts);
      if (!ts.some((t) => t.id === selId)) setSelId(ts[0]?.id ?? "");
      refresh();
      say(`Imported ${r.entries} entries${r.trackers ? ` and ${r.trackers} trackers` : ""}${r.skipped ? `, skipped ${r.skipped} duplicates` : ""}`);
    } catch (e) { fail(e); }
  };
  const doBackup = async () => {
    const stamp = new Date().toISOString().slice(0, 10);
    const path = await save({ defaultPath: `heatmap-${stamp}.db`, filters: [{ name: "SQLite database", extensions: ["db"] }] });
    if (path) await run(() => api.backupDb(path), "Database backup saved");
  };

  const levels = useMemo(() => (tracker ? levelStyles(tracker.color, bgHex, studio) : []), [tracker, bgHex, studio]);
  const legend = levels.map((l) => l.bg);
  const bool = tracker?.type === "boolean";
  const addRef = useRef<HTMLInputElement>(null), noteRef = useRef<HTMLInputElement>(null);
  const add = (zero = false) => {
    if (!tracker) return;
    const v = zero ? 0 : bool ? 1 : parseFloat(addRef.current?.value ?? "");
    if (Number.isNaN(v)) return say("Enter a number first.", true);
    run(async () => { await api.addEntry(tracker.id, day, v, noteRef.current?.value ?? ""); if (addRef.current) addRef.current.value = ""; if (noteRef.current) noteRef.current.value = ""; });
  };

  const changeStudio = (n: StudioCfg) => { setStudio(n); put("studio", JSON.stringify(n)); };
  const exportYear = async (kind: "svg" | "png") => {
    if (!tracker) return;
    try {
      const path = await save({ defaultPath: `${tracker.name.replace(/[^\w-]+/g, "_")}-${year}.${kind}`, filters: [{ name: kind.toUpperCase(), extensions: [kind] }] });
      if (!path) return;
      const data = await api.dayAggregates(tracker.id, dk(year, 0, 1), dk(year, 11, 31));
      const css = getComputedStyle(document.documentElement), v = (n: string) => css.getPropertyValue(n).trim();
      const svg = yearSvg({ year, weekStart, title: tracker.name, levels: new Map(data.map((a) => [a.date, a.level])), palette: levels,
        page: v("--bg"), text: v("--fg"), muted: v("--mut"), empty: v("--bg2"), gap: studio.gap, radiusFrac: SHAPES[studio.shape] });
      if (kind === "svg") await api.writeExport(path, svg); else await api.writeExport(path, undefined, await svgToPng(svg));
      say(`Saved ${kind.toUpperCase()}`);
    } catch (e) { fail(e); }
  };
  const submitQuick = async () => {
    if (!quickText.trim()) return;
    try { const c = await api.quickCapture(quickText, today); setQuickText(""); refresh(); say(`Logged ${c.trackerName}: ${c.value} on ${c.date}`); } catch (e) { fail(e); }
  };
  const monthFreezes = [...frozen].filter((d) => d.slice(0, 7) === day.slice(0, 7)).length;
  const canFreeze = !!tracker && tracker.freezesPerMonth > 0;
  const isFrozen = frozen.has(day);
  const toggleFreeze = () => tracker && run(() => api.setFreeze(tracker.id, day, !isFrozen), isFrozen ? "Freeze removed" : "Day frozen: your streak is protected");

  if (!ready) return <div className="empty mut">Opening your data…</div>;
  const gp = tracker && { tracker, aggs, today, selected: day, weekStart, bgHex, onSelect: select, palette: levels, frozen };
  const d = parseKey(day);
  const S = stats;
  const cards: [string, string | number][] = S ? [
    ["Active days", S.activeDays], ["Recorded zero", S.zeroDays], ["Total", tracker?.type === "rating" ? "–" : round2(S.total)],
    ["Average", S.average === null ? "–" : round2(S.average)], ["Median", S.median === null ? "–" : round2(S.median)], ["Max", S.max === null ? "–" : round2(S.max)],
    [`Best streak (${S.unit})`, S.bestStreak], [`Current streak (${S.unit})`, S.currentStreak], [S.unit === "weeks" ? "Weeks on target" : "Goal hit", S.goalHitPct === null ? "–" : `${Math.round(S.goalHitPct)}%`],
  ] : [];

  return (
    <>
      <header>
        <h1>Heatmap</h1>
        {tracker && <select aria-label="Tracker" value={tracker.id} onChange={(e) => { setSelId(e.target.value); put("selectedTracker", e.target.value); }}>
          {trackers.map((t) => <option key={t.id} value={t.id}>{t.name}</option>)}
        </select>}
        <button onClick={() => openForm()}>+ Tracker</button>
        {tracker && <><button onClick={() => openForm(tracker)}>Edit</button><button onClick={removeTracker}>Delete</button></>}
        <span style={{ flex: 1 }} />
        {tracker && <>
          <button className={view === "month" ? "on" : ""} onClick={() => { setView("month"); put("view", "month"); }}>Month</button>
          <button className={view === "year" ? "on" : ""} onClick={() => { setView("year"); put("view", "year"); }}>Year</button>
          <select aria-label="First weekday" value={weekStart} onChange={(e) => { setWeekStart(+e.target.value); put("weekStart", e.target.value); }}>
            <option value={0}>Sun first</option><option value={1}>Mon first</option>
          </select>
          <label className="mut">Zoom <input type="range" min={28} max={64} value={cs} style={{ width: 90, padding: 0 }}
            onChange={(e) => setCs(+e.target.value)} onPointerUp={() => put("zoom", String(cs))} onKeyUp={() => put("zoom", String(cs))} /></label>
        </>}
        <button onClick={() => { const dark = bgHex.toLowerCase() === "#0a0a0b"; const n = dark ? "light" : "dark"; setTheme(n); put("theme", n); }}>Theme</button>
        <button aria-pressed={reduceMotion} onClick={() => { setReduceMotion(!reduceMotion); put("reduceMotion", reduceMotion ? "0" : "1"); }}>Motion: {reduceMotion ? "off" : "on"}</button>
        <button onClick={() => doExport("json")}>Export JSON</button>
        <button onClick={() => doExport("csv")}>Export CSV</button>
        <button onClick={doImport}>Import</button>
        <button onClick={doBackup}>Backup DB</button>
        {tracker && <button onClick={() => setShowStudio(true)}>Studio</button>}
        <button onClick={() => setShowTools(true)}>Widgets & tools</button>
        {tracker && <div className="quick"><input value={quickText} onChange={(e) => setQuickText(e.target.value)} onKeyDown={(e) => e.key === "Enter" && submitQuick()}
          placeholder="Quick add: coding 90m yesterday" aria-label="Quick add" /><button onClick={submitQuick}>Log</button></div>}
      </header>
      <main>
        {!tracker || !gp ? (
          <div className="stage"><div className="empty">
            <h2>Track anything, see it at a glance</h2>
            <p className="mut">Create your first tracker — a habit, a duration, a count, a rating. Everything stays on this computer.</p>
            <button className="on" onClick={() => openForm()}>Create a tracker</button>
            <p className="mut small">Have a backup? Use Import to restore a JSON backup or add entries from CSV.</p>
          </div></div>
        ) : <>
          <div className="stage">
            <div className="nav">
              <button aria-label={view === "year" ? "Previous year" : "Previous month"} onClick={() => nav(-1)}>‹</button>
              <span>{view === "year" ? year : `${MN[month]} ${year}`}</span>
              <button aria-label={view === "year" ? "Next year" : "Next month"} onClick={() => nav(1)}>›</button>
              <button onClick={() => { nav(0); select(today); }}>Today</button>
            </div>
            {view === "month" ? <MonthView {...gp} year={year} month={month} /> : <YearView {...gp} year={year} />}
            <div className="leg"><span>Missing</span><i style={{ background: "var(--bg2)" }} /><span>Zero</span>
              <i style={{ background: "var(--bg2)", border: "1.5px dashed var(--mut)" }} /><span>Rest day</span>
              <i style={{ background: "var(--bg2)", backgroundImage: "repeating-linear-gradient(135deg,transparent 0 3px,var(--mut) 3px 4px)" }} />
              <span>Less</span>{legend.map((c, i) => <i key={i} style={{ background: c }} />)}<span>More</span>
              <span>· full intensity = {tracker.goal} {tracker.type === "duration" ? "min" : tracker.unit}</span></div>
            <div className="stats">{cards.map(([k, v]) => <div className="stat" key={k}><b>{v}</b><span>{k}</span></div>)}</div>
            <p className="mut small">Streaks count scheduled days with a value above zero; rest days never break a streak. Averages use recorded days only.</p>
            <Insights tracker={tracker} trackers={trackers} today={today} version={version} />
          </div>
          <aside aria-label="Day details">
            <h3 style={{ margin: 0 }}>{new Date(d.y, d.m, d.d).toLocaleDateString(undefined, { weekday: "long", day: "numeric", month: "long", year: "numeric" })}</h3>
            <p className="mut">{tracker.name} · {fmtValue(tracker, aggs.get(day)?.value)}{day > today ? " · future date" : ""}</p>
            {entries.length === 0 && <p className="mut">No entries for this day.</p>}
            {entries.map((e) => (
              <div className="ent" key={`${e.id}:${e.value}:${e.note}`}>
                {e.source !== "manual" && <span className="badge" title="Created by a connected source">git</span>}
                {bool ? <span>{e.value > 0 ? "Done" : "Not done"}</span>
                  : <input type="number" step="any" defaultValue={e.value} aria-label="Value" readOnly={e.source !== "manual"}
                      onBlur={(ev) => { const v = parseFloat(ev.target.value); if (v !== e.value) run(() => api.updateEntry(e.id, v, e.note)); }} />}
                <input className="n" defaultValue={e.note} placeholder="Note" aria-label="Note"
                  onBlur={(ev) => { if (ev.target.value !== e.note) run(() => api.updateEntry(e.id, e.value, ev.target.value)); }} />
                <button aria-label="Delete entry" onClick={() => run(() => api.deleteEntry(e.id))}>✕</button>
              </div>
            ))}
            <hr style={{ border: 0, borderTop: "1px solid var(--bd)", margin: "14px 0" }} />
            <div className="ent">
              {!bool && <input ref={addRef} type="number" step="any" min={0} aria-label="New value" placeholder={tracker.type === "duration" ? "minutes" : tracker.unit || "value"} onKeyDown={(e) => e.key === "Enter" && add()} />}
              <input ref={noteRef} className="n" placeholder="Note" aria-label="New note" onKeyDown={(e) => e.key === "Enter" && add()} />
              <button className="on" onClick={() => add()}>{bool ? "Mark done" : "Add"}</button>
            </div>
            <button style={{ width: "100%", marginTop: 6 }} onClick={() => add(true)}>{bool ? "Record as not done" : "Record explicit zero"}</button>
            {canFreeze && <button style={{ width: "100%", marginTop: 6 }} aria-pressed={isFrozen} onClick={toggleFreeze}>{isFrozen ? "❄ Frozen: click to unfreeze" : `❄ Freeze this day (${Math.max(0, tracker.freezesPerMonth - monthFreezes)} left this month)`}</button>}
            <p className="mut small" style={{ marginTop: 14 }}>Arrow keys move between days. Data is stored in a local SQLite database.</p>
          </aside>
        </>}
      </main>
      <dialog ref={dlg} onClose={() => undefined}>
        <form method="dialog" onSubmit={(e) => { e.preventDefault(); saveForm(); }}>
          <h3 style={{ margin: "0 0 4px" }}>{form.id ? "Edit tracker" : "New tracker"}</h3>
          <label htmlFor="f-name">Name</label>
          <input id="f-name" required maxLength={40} value={form.v.name} onChange={(e) => setForm({ ...form, v: { ...form.v, name: e.target.value } })} />
          <label htmlFor="f-type">Type {form.id ? "(fixed once entries exist)" : ""}</label>
          <select id="f-type" value={form.v.type} onChange={(e) => { const t = e.target.value as Kind; setForm({ ...form, v: { ...form.v, type: t, goal: form.id ? form.v.goal : t === "duration" ? 60 : t === "rating" ? 4 : 1, unit: form.id ? form.v.unit : t === "duration" ? "min" : "" } }); }}>
            <option value="boolean">Boolean (done / not done)</option><option value="count">Count</option><option value="duration">Duration (minutes)</option>
            <option value="numeric">Numeric (custom unit)</option><option value="rating">Rating (1–5)</option>
          </select>
          <label htmlFor="f-unit">Unit</label>
          <input id="f-unit" maxLength={12} value={form.v.unit} onChange={(e) => setForm({ ...form, v: { ...form.v, unit: e.target.value } })} />
          <label htmlFor="f-goal">Daily goal (full intensity)</label>
          <input id="f-goal" type="number" min={0.01} step="any" required value={form.v.goal} onChange={(e) => setForm({ ...form, v: { ...form.v, goal: parseFloat(e.target.value) } })} />
          <label htmlFor="f-color">Color</label>
          <input id="f-color" type="color" style={{ height: 36, padding: 2 }} value={form.v.color} onChange={(e) => setForm({ ...form, v: { ...form.v, color: e.target.value } })} />
          <label>Scheduled weekdays (unchecked = rest day)</label>
          <div className="days">{DAY_NAMES.map((n, i) => (
            <label key={n}><input type="checkbox" checked={((form.v.activeDays >> i) & 1) === 1}
              onChange={() => setForm({ ...form, v: { ...form.v, activeDays: form.v.activeDays ^ (1 << i) } })} />{n}</label>))}</div>
          <label htmlFor="f-gm">Goal style</label>
          <select id="f-gm" value={form.v.goalMode} onChange={(e) => setForm({ ...form, v: { ...form.v, goalMode: e.target.value as "daily" | "weekly" } })}>
            <option value="daily">Daily: every scheduled day counts</option><option value="weekly">Weekly: reach a number of days per week</option>
          </select>
          {form.v.goalMode === "weekly" && <><label htmlFor="f-wt">Days per week to reach the goal</label>
            <input id="f-wt" type="number" min={1} max={7} value={form.v.weeklyTarget} onChange={(e) => setForm({ ...form, v: { ...form.v, weeklyTarget: Math.max(1, Math.min(7, parseInt(e.target.value) || 1)) } })} /></>}
          <label htmlFor="f-fz">Freeze days per month (protect a streak)</label>
          <input id="f-fz" type="number" min={0} max={10} value={form.v.freezesPerMonth} onChange={(e) => setForm({ ...form, v: { ...form.v, freezesPerMonth: Math.max(0, Math.min(10, parseInt(e.target.value) || 0)) } })} />
          <label htmlFor="f-start">Start date (optional)</label>
          <input id="f-start" type="date" value={form.v.startDate ?? ""} onChange={(e) => setForm({ ...form, v: { ...form.v, startDate: e.target.value || null } })} />
          <label htmlFor="f-end">End date (optional)</label>
          <input id="f-end" type="date" value={form.v.endDate ?? ""} onChange={(e) => setForm({ ...form, v: { ...form.v, endDate: e.target.value || null } })} />
          <div style={{ display: "flex", gap: 8, marginTop: 16, justifyContent: "flex-end" }}>
            <button type="button" onClick={() => dlg.current?.close()}>Cancel</button>
            <button className="on" type="submit">Save</button>
          </div>
        </form>
      </dialog>
      {showStudio && tracker && <Studio studio={studio} color={tracker.color} bgHex={bgHex} onChange={changeStudio} onExport={exportYear} onClose={() => setShowStudio(false)} />}
      {showTools && <Tools tracker={tracker} onClose={() => setShowTools(false)} say={say} fail={fail} refresh={refresh} />}
      {toast && <div className={`toast${toast.err ? " err" : ""}`} role="status">{toast.msg}</div>}
    </>
  );
}
