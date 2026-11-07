import { fixed, shade } from "./color";
import type { Studio } from "../types";

export const DEFAULT_STUDIO: Studio = { palette: "tracker", shape: "rounded", gap: 5 };
export const PALETTES: Record<string, { label: string; levels?: string[] }> = {
  tracker: { label: "Tracker colour" },
  forest: { label: "Forest", levels: ["#9be9a8", "#40c463", "#30a14e", "#216e39"] },
  ocean: { label: "Ocean", levels: ["#bfe3ff", "#6fb7f5", "#2f84d8", "#14508f"] },
  sunset: { label: "Sunset", levels: ["#ffd9a8", "#ffa561", "#f0623a", "#b02a2a"] },
  neon: { label: "Neon", levels: ["#0b6e7a", "#12b5c7", "#7af0ff", "#ff4fd8"] },
  grape: { label: "Grape", levels: ["#e6d4ff", "#b98cf5", "#8450d8", "#4b2394"] },
  mono: { label: "Monochrome", levels: ["#d4d4d4", "#a3a3a3", "#6b6b6b", "#2b2b2b"] },
};
export const SHAPES = { rounded: 0.2, square: 0.06, round: 0.5 } as const;

export function parseStudio(raw: string | undefined): Studio {
  try {
    const v = JSON.parse(raw ?? "");
    return {
      palette: v.palette in PALETTES ? v.palette : "tracker",
      shape: v.shape in SHAPES ? v.shape : "rounded",
      gap: Math.max(1, Math.min(12, Number(v.gap) || 5)),
    };
  } catch { return DEFAULT_STUDIO; }
}

/** Colours for intensity levels 1–4: from the studio palette, or the tracker colour blended into the background. */
export function levelStyles(trackerColor: string, bgHex: string, studio: Studio) {
  const levels = PALETTES[studio.palette]?.levels;
  return [1, 2, 3, 4].map((l) => (levels ? fixed(levels[l - 1]) : shade(trackerColor, l, bgHex)));
}
