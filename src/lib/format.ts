import type { Tracker } from "../types";
export const round2 = (n: number) => Math.round(n * 100) / 100;
export function fmtValue(t: Tracker, v: number | undefined): string {
  if (v === undefined) return "No data";
  if (t.type === "boolean") return v > 0 ? "Done" : "Not done";
  if (t.type === "duration") return `${round2(v)} min`;
  if (t.type === "rating") return `${round2(v)} / 5`;
  return `${round2(v)}${t.unit ? " " + t.unit : ""}`;
}
