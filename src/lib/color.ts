const hex = (h: string) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
const lin = (c: number) => { const s = c / 255; return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4; };
export const luminance = (rgb: number[]) => 0.2126 * lin(rgb[0]) + 0.7152 * lin(rgb[1]) + 0.0722 * lin(rgb[2]);
export const contrast = (a: number[], b: number[]) => { const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p); return (x + 0.05) / (y + 0.05); };

/** Whichever of black/white has the higher WCAG contrast; this always yields at least 4.5:1. */
export const readableOn = (rgb: number[]) => (contrast(rgb, [255, 255, 255]) >= contrast(rgb, [0, 0, 0]) ? "#ffffff" : "#000000");

const ALPHA = [0, 0.3, 0.52, 0.76, 1]; // intensity levels 1..4 blend the tracker colour into the page background
export function shade(color: string, level: number, bgHex: string) {
  const bg = hex(bgHex), c = hex(color), a = ALPHA[Math.max(1, Math.min(4, level))];
  const m = c.map((x, i) => Math.round(bg[i] + (x - bg[i]) * a));
  return { bg: `rgb(${m.join(",")})`, fg: readableOn(m), rgb: m };
}
/** A fixed colour (from a palette) with a readable label colour. */
export function fixed(color: string) { const m = hex(color); return { bg: color, fg: readableOn(m), rgb: m }; }
