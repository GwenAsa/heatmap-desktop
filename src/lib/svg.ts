import { addDays, dk, isLeap, parseKey, yearGrid } from "./dates";

export interface SvgInput {
  year: number; weekStart: number; title: string;
  levels: Map<string, number>;                  // date -> level (0 = recorded zero, 1..4); missing dates are absent
  palette: { bg: string; fg: string }[];        // levels 1..4
  page: string; text: string; muted: string; empty: string;
  cell?: number; gap?: number; radiusFrac?: number;
}
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const DAYS = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
export const esc = (s: string) => s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]!));

/** A standalone SVG of one year. Every real date carries its day number; all text is escaped. */
export function yearSvg(o: SvgInput): string {
  const cell = o.cell ?? 30, gap = o.gap ?? 4, step = cell + gap, left = 38, top = 70;
  const g = yearGrid(o.year, o.weekStart), r = cell * (o.radiusFrac ?? 0.2);
  const W = left + g.cols * step + 20, H = top + 7 * step + 56;
  const out: string[] = [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" font-family="Inter, 'Segoe UI', Ubuntu, sans-serif">`,
    `<rect width="${W}" height="${H}" fill="${esc(o.page)}"/>`,
    `<text x="${left}" y="34" font-size="20" font-weight="700" fill="${esc(o.text)}">${esc(o.title)} · ${o.year}</text>`,
  ];
  for (const l of g.labels) out.push(`<text x="${left + l.col * step}" y="${top - 10}" font-size="12" fill="${esc(o.muted)}">${MONTHS[l.month]}</text>`);
  for (let i = 0; i < 7; i++) out.push(`<text x="${left - 8}" y="${top + i * step + cell / 2}" font-size="11" text-anchor="end" dy=".35em" fill="${esc(o.muted)}">${DAYS[(i + o.weekStart) % 7]}</text>`);
  g.cells.forEach((d, i) => {
    if (!d) return;
    const x = left + Math.floor(i / 7) * step, y = top + (i % 7) * step, lv = o.levels.get(d);
    const filled = lv !== undefined && lv > 0, p = filled ? o.palette[lv - 1] : undefined;
    const stroke = lv === 0 ? ` stroke="${esc(o.muted)}" stroke-width="1.5" stroke-dasharray="4 3"` : "";
    out.push(`<rect data-date="${d}" x="${x}" y="${y}" width="${cell}" height="${cell}" rx="${r}" fill="${esc(p ? p.bg : o.empty)}"${stroke}/>`);
    out.push(`<text class="d" x="${x + cell / 2}" y="${y + cell / 2}" font-size="${Math.round(cell * 0.4)}" font-weight="600" text-anchor="middle" dy=".35em" fill="${esc(p ? p.fg : o.text)}">${parseKey(d).d}</text>`);
  });
  const ly = top + 7 * step + 28;
  out.push(`<text x="${left}" y="${ly + 11}" font-size="12" fill="${esc(o.muted)}">Less</text>`);
  o.palette.forEach((p, i) => out.push(`<rect x="${left + 36 + i * 22}" y="${ly}" width="16" height="16" rx="4" fill="${esc(p.bg)}"/>`));
  out.push(`<text x="${left + 36 + 4 * 22 + 4}" y="${ly + 11}" font-size="12" fill="${esc(o.muted)}">More</text></svg>`);
  return out.join("\n");
}

/** Rasterise an SVG string to a PNG data URL (runs in the webview). */
export function svgToPng(svg: string, scale = 2): Promise<string> {
  return new Promise((resolve, reject) => {
    const m = /width="(\d+)" height="(\d+)"/.exec(svg);
    if (!m) return reject(new Error("Invalid SVG"));
    const img = new Image();
    img.onload = () => {
      const c = document.createElement("canvas");
      c.width = +m[1] * scale; c.height = +m[2] * scale;
      const ctx = c.getContext("2d")!;
      ctx.scale(scale, scale);
      ctx.drawImage(img, 0, 0);
      resolve(c.toDataURL("image/png"));
    };
    img.onerror = () => reject(new Error("Could not render the image"));
    img.src = "data:image/svg+xml;charset=utf-8," + encodeURIComponent(svg);
  });
}
export const yearDays = (y: number) => (isLeap(y) ? 366 : 365);
export const lastOfYear = (y: number) => addDays(dk(y, 11, 31), 0);
