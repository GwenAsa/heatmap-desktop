import { useEffect, useState } from "react";
import { api, errMsg } from "../api";
import { round2 } from "../lib/format";
import type { Correlation, Insights as Ins, Tracker } from "../types";

const WD = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/** Descriptive patterns from your own records. Nothing here claims that one thing causes another. */
export default function Insights({ tracker, trackers, today, version }: { tracker: Tracker; trackers: Tracker[]; today: string; version: number }) {
  const [ins, setIns] = useState<Ins | null>(null);
  const [other, setOther] = useState("");
  const [lag, setLag] = useState(0);
  const [cor, setCor] = useState<Correlation | null | "none">(null);
  const [err, setErr] = useState("");
  useEffect(() => {
    let live = true;
    api.insights(tracker.id, today).then((i) => live && setIns(i)).catch((e) => live && setErr(errMsg(e)));
    return () => { live = false; };
  }, [tracker.id, today, version]);
  const others = trackers.filter((t) => t.id !== tracker.id);
  const compare = async () => {
    try { const c = await api.correlation(tracker.id, other, lag, 365, today); setCor(c ?? "none"); setErr(""); } catch (e) { setErr(errMsg(e)); }
  };
  const max = Math.max(...(ins?.weekdayAvg.map((v) => v ?? 0) ?? [0]), 0.0001);
  return (
    <details>
      <summary>Insights for “{tracker.name}”</summary>
      {err && <p role="alert" style={{ color: "#e5484d" }}>{err}</p>}
      {ins && ins.recordedDays < 7 ? <p className="mut">Record at least a week of data to see patterns.</p> : ins && <>
        <h4 style={{ margin: "12px 0 0" }}>Average by weekday</h4>
        <div className="bars" aria-label="Average by weekday">
          {ins.weekdayAvg.map((v, i) => (
            <div key={i} className={ins.bestWeekday === i ? "best" : ""} title={`${WD[i]}: ${v === null ? "no data" : round2(v)} (${ins.weekdayDays[i]} days)`}>
              <i style={{ height: `${((v ?? 0) / max) * 100}%` }} />{WD[i].slice(0, 2)}
            </div>))}
        </div>
        <p className="small mut">Based on {ins.recordedDays} recorded days.{ins.bestWeekday !== null && <> Strongest weekday: <b style={{ color: "var(--fg)" }}>{WD[ins.bestWeekday]}</b>.</>}</p>
        <p>
          Last 30 days: <b>{ins.last30Avg === null ? "–" : round2(ins.last30Avg)}</b> avg per recorded day vs <b>{ins.prev30Avg === null ? "–" : round2(ins.prev30Avg)}</b> before
          {ins.changePct !== null && <> ({ins.changePct >= 0 ? "▲" : "▼"} {Math.abs(Math.round(ins.changePct))}%)</>}.
        </p>
        {ins.busiest && <p className="small">Best day ever: <b>{ins.busiest.date}</b> ({round2(ins.busiest.value)}).</p>}
        {ins.outliers.length > 0 && <p className="small">Unusual recent days: {ins.outliers.map((o) => `${o.date} (${round2(o.value)}, ${o.direction})`).join(" · ")}</p>}
      </>}
      <h4 style={{ margin: "14px 0 4px" }}>Compare with another tracker</h4>
      {others.length === 0 ? <p className="mut small">Create a second tracker to compare.</p> : (
        <div className="row">
          <select aria-label="Other tracker" value={other} onChange={(e) => { setOther(e.target.value); setCor(null); }}>
            <option value="">Choose…</option>{others.map((t) => <option key={t.id} value={t.id}>{t.name}</option>)}
          </select>
          <select aria-label="Time shift" value={lag} onChange={(e) => { setLag(+e.target.value); setCor(null); }}>
            <option value={0}>same day</option><option value={1}>next day</option><option value={2}>2 days later</option><option value={-1}>previous day</option>
          </select>
          <button disabled={!other} onClick={compare}>Compare</button>
        </div>)}
      {cor === "none" && <p className="small mut">Not enough shared days yet (at least 14 days where both were recorded in the last year).</p>}
      {cor && cor !== "none" && <p>
        Over <b>{cor.n}</b> shared days, the relationship is <b>{cor.strength}</b> (r = {cor.r.toFixed(2)}{cor.lag ? `, shifted ${cor.lag} day${Math.abs(cor.lag) > 1 ? "s" : ""}` : ""}).
        <br /><span className="small mut">{cor.caution}</span></p>}
    </details>
  );
}
