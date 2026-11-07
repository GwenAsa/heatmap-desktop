import { describe, expect, it } from "vitest";
import { contrast, shade } from "./color";

describe("day-number readability", () => {
  const colors = ["#3ddc84", "#ff5a5f", "#4f8cff", "#ffd400", "#a855f7", "#ffffff", "#000000", "#808080", "#00ff00"];
  for (const bg of ["#0a0a0b", "#fafafa"]) {
    it(`keeps ≥4.5:1 contrast at every intensity on ${bg}`, () => {
      for (const c of colors) for (let level = 1; level <= 4; level++) {
        const s = shade(c, level, bg);
        const fg = s.fg === "#ffffff" ? [255, 255, 255] : [0, 0, 0];
        expect(contrast(s.rgb, fg), `${c} L${level} on ${bg}`).toBeGreaterThanOrEqual(4.5);
      }
    });
  }
  it("gets stronger with the level", () => {
    const a = shade("#3ddc84", 1, "#0a0a0b").rgb[1], b = shade("#3ddc84", 4, "#0a0a0b").rgb[1];
    expect(b).toBeGreaterThan(a);
  });
});
