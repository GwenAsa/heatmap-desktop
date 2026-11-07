import { useCallback, useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, errMsg } from "../api";
import type { Source, Tracker } from "../types";

const WIDGETS = [
  { kind: "ring", name: "Glass ring", desc: "Today’s progress ring with one-tap logging" },
  { kind: "month", name: "Neon month", desc: "Terminal-style month calendar" },
  { kind: "strip", name: "Paper strip", desc: "Last 14 weeks on one minimal strip" },
  { kind: "sheet", name: "Sticky sheet", desc: "All trackers for today + type-to-log line" },
];

export default function Tools({ tracker, onClose, say, fail, refresh }: { tracker?: Tracker; onClose: () => void; say: (m: string) => void; fail: (e: unknown) => void; refresh: () => void }) {
  const ref = useRef<HTMLDialogElement>(null);
  const [openKinds, setOpenKinds] = useState<string[]>([]);
  const [auto, setAuto] = useState(false);
  const [sources, setSources] = useState<Source[]>([]);
  const [busy, setBusy] = useState("");
  const load = useCallback(() => {
    api.listOpenWidgets().then(setOpenKinds).catch(fail);
    api.getAutostart().then(setAuto).catch(fail);
    if (tracker) api.listSources(tracker.id).then(setSources).catch(fail); else setSources([]);
  }, [tracker, fail]);
  useEffect(() => { ref.current?.showModal(); load(); }, [load]);

  const toggleWidget = async (kind: string) => {
    try { openKinds.includes(kind) ? await api.closeWidget(kind) : await api.openWidget(kind); setTimeout(load, 300); } catch (e) { fail(e); }
  };
  const addRepo = async () => {
    if (!tracker) return;
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir !== "string") return;
    try { const s = await api.addGitSource(tracker.id, dir); const r = await api.syncSource(s.id); say(`Connected. Found ${r.commits} commits on ${r.days} days.`); load(); refresh(); } catch (e) { fail(e); }
  };
  const sync = async (s: Source) => {
    setBusy(s.id);
    try { const r = await api.syncSource(s.id); say(`Synced ${r.commits} commits on ${r.days} days.`); load(); refresh(); } catch (e) { fail(e); } finally { setBusy(""); }
  };
  const remove = async (s: Source) => {
    try { await api.removeSource(s.id, true); say("Source removed with the entries it created."); load(); refresh(); } catch (e) { fail(e); }
  };
  return (
    <dialog ref={ref} className="wide" onClose={onClose}>
      <h3 style={{ margin: 0 }}>Widgets & tools</h3>
      <h4>Desktop widgets</h4>
      <p className="mut small" style={{ margin: "0 0 6px" }}>Drag a widget anywhere. Its position, tracker and pin mode (📌 on top · ▫ normal · ⬇ behind windows) are remembered.</p>
      {WIDGETS.map((w) => (
        <div className="row" key={w.kind}>
          <button className={openKinds.includes(w.kind) ? "on" : ""} aria-pressed={openKinds.includes(w.kind)} onClick={() => toggleWidget(w.kind)}>{openKinds.includes(w.kind) ? "Close" : "Open"}</button>
          <span><b>{w.name}</b> <span className="mut small">{w.desc}</span></span>
        </div>))}
      <div className="row"><label style={{ margin: 0, color: "var(--fg)", fontSize: 14 }}>
        <input type="checkbox" checked={auto} onChange={async (e) => { try { await api.setAutostart(e.target.checked); setAuto(e.target.checked); } catch (er) { fail(er); } }} /> Start with my computer (opens your widgets, window hidden)</label></div>
      <h4>Quick add</h4>
      <p className="mut small" style={{ margin: 0 }}>Press <b>Ctrl+Alt+H</b> (X11), or bind <code>heatmap-app --quick</code> to any shortcut in Settings → Keyboard (works on Wayland too). Examples: <code>coding 90m yesterday</code>, <code>gym</code>, <code>reading 20 // chapter 4</code>.</p>
      <div className="row"><button onClick={() => api.openQuick().catch(fail)}>Open quick add</button></div>
      <h4>Git repositories{tracker ? ` → “${tracker.name}”` : ""}</h4>
      {!tracker ? <p className="mut small">Create a Count tracker first.</p> : tracker.type !== "count" ? <p className="mut small">Select a <b>Count</b> tracker to feed it from git commits. Only your own commits are counted (by your git user.email), by local date. Everything is read locally.</p> : <>
        <p className="mut small" style={{ margin: "0 0 6px" }}>Counts your own commits per day (by your git user.email, all branches, merges excluded). Read-only and local. Your manual entries are never touched.</p>
        {sources.map((s) => (
          <div className="src" key={s.id}>
            <span title={s.path}>{s.path}</span>
            <button disabled={busy === s.id} onClick={() => sync(s)}>{busy === s.id ? "…" : "Sync"}</button>
            <button onClick={() => remove(s)} aria-label="Remove repository">✕</button>
          </div>))}
        <div className="row"><button onClick={addRepo}>+ Add repository…</button></div>
      </>}
      <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 16 }}><button className="on" onClick={() => ref.current?.close()}>Done</button></div>
    </dialog>
  );
}
// errMsg is re-exported for callers that want to format raw errors consistently.
export { errMsg };
