import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, errMsg } from "../api";
import { todayKey } from "../lib/dates";
import type { Capture } from "../types";

/** Quick-add window: type a sentence, see what will be logged, press Enter. Esc closes. */
export default function Quick() {
  const [text, setText] = useState("");
  const [preview, setPreview] = useState<Capture | null>(null);
  const [error, setError] = useState("");
  const [done, setDone] = useState("");
  const ref = useRef<HTMLInputElement>(null);
  const close = () => getCurrentWindow().close();
  useEffect(() => { ref.current?.focus(); }, []);
  useEffect(() => {
    if (!text.trim()) { setPreview(null); setError(""); return; }
    const id = setTimeout(() => api.capturePreview(text, todayKey()).then((c) => { setPreview(c); setError(""); }).catch((e) => { setPreview(null); setError(errMsg(e)); }), 120);
    return () => clearTimeout(id);
  }, [text]);
  const submit = async () => {
    try { const c = await api.quickCapture(text, todayKey()); setDone(`Saved: ${c.trackerName} · ${c.value} · ${c.date}`); setTimeout(close, 700); }
    catch (e) { setError(errMsg(e)); }
  };
  return (
    <div className="qwrap" data-tauri-drag-region onKeyDown={(e) => e.key === "Escape" && close()}>
      <div className="qbox">
        <input ref={ref} value={text} onChange={(e) => setText(e.target.value)} onKeyDown={(e) => e.key === "Enter" && !done && submit()}
          placeholder="coding 90m yesterday   ·   gym   ·   reading 20 // chapter 4" aria-label="Quick add" spellCheck={false} />
        <div className={`qprev${error ? " err" : ""}`} role="status">
          {done || (error ? error : preview ? `${preview.trackerName} · ${preview.value} · ${preview.date}${preview.note ? " · “" + preview.note + "”" : ""}   ⏎ to save` : "Type a tracker name, then a value and optionally a date (yesterday, mon, 2 days ago, 2025-03-01).")}
        </div>
      </div>
    </div>
  );
}
