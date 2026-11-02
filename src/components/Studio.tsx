import { useEffect, useRef } from "react";
import { fixed, shade } from "../lib/color";
import { PALETTES, SHAPES } from "../lib/theme";
import type { Studio as StudioCfg } from "../types";

interface Props { studio: StudioCfg; color: string; bgHex: string; onChange: (s: StudioCfg) => void; onExport: (kind: "svg" | "png") => void; onClose: () => void }

/** Heatmap Studio: every control changes the real heatmap immediately and is saved in the database. */
export default function Studio({ studio, color, bgHex, onChange, onExport, onClose }: Props) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => { ref.current?.showModal(); }, []);
  const swatches = (key: string) => [1, 2, 3, 4].map((l) => (PALETTES[key].levels ? fixed(PALETTES[key].levels![l - 1]) : shade(color, l, bgHex)).bg);
  return (
    <dialog ref={ref} className="wide" onClose={onClose}>
      <h3 style={{ margin: 0 }}>Heatmap Studio</h3>
      <h4>Palette</h4>
      <div role="radiogroup" aria-label="Palette">
        {Object.entries(PALETTES).map(([key, p]) => (
          <div className="row" key={key}>
            <label style={{ display: "flex", gap: 8, alignItems: "center", margin: 0, color: "var(--fg)", fontSize: 14 }}>
              <input type="radio" name="pal" checked={studio.palette === key} onChange={() => onChange({ ...studio, palette: key })} />
              {p.label}<span className="sw">{swatches(key).map((c, i) => <i key={i} style={{ background: c }} />)}</span>
            </label>
          </div>
        ))}
      </div>
      <h4>Cell shape</h4>
      <div className="row">
        {(Object.keys(SHAPES) as (keyof typeof SHAPES)[]).map((k) => (
          <button key={k} className={studio.shape === k ? "on" : ""} onClick={() => onChange({ ...studio, shape: k })}>{k}</button>
        ))}
      </div>
      <h4>Gap between cells: {studio.gap}px</h4>
      <input type="range" min={1} max={12} value={studio.gap} aria-label="Gap" style={{ width: "100%", padding: 0 }} onChange={(e) => onChange({ ...studio, gap: +e.target.value })} />
      <h4>Export the year you are viewing</h4>
      <p className="mut small" style={{ margin: "0 0 6px" }}>Every day keeps its number. Use it on GitHub, a website, or print it.</p>
      <div className="row"><button onClick={() => onExport("svg")}>Save as SVG</button><button onClick={() => onExport("png")}>Save as PNG</button></div>
      <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 16 }}><button className="on" onClick={() => ref.current?.close()}>Done</button></div>
    </dialog>
  );
}
