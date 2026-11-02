import { describe, expect, it } from "vitest";
import { yearSvg, type SvgInput } from "./svg";

const base = (over: Partial<SvgInput> = {}): SvgInput => ({
  year: 2026, weekStart: 0, title: "Coding", levels: new Map(), palette: [1, 2, 3, 4].map((i) => ({ bg: `#00${i}000`, fg: "#fff" })),
  page: "#0a0a0b", text: "#fff", muted: "#888", empty: "#222", ...over });

describe("SVG export", () => {
  it("prints a day number for every date of a leap and a common year", () => {
    const leap = yearSvg(base({ year: 2024 })), common = yearSvg(base({ year: 2025 }));
    expect(leap.match(/class="d"/g)).toHaveLength(366);
    expect(common.match(/class="d"/g)).toHaveLength(365);
    expect(leap).toContain(">29</text>");
  });
  it("places dates in their weekday row (2026-01-01 is a Thursday → 5th row with Sunday first)", () => {
    const svg = yearSvg(base());
    const y = (d: string) => Number(new RegExp(`data-date="${d}" x="\\d+" y="(\\d+)"`).exec(svg)![1]);
    const step = 34;
    expect((y("2026-01-01") - y("2026-01-04")) / step).toBe(4);   // Sunday is row 0, Thursday row 4
    expect(y("2026-01-04")).toBe(y("2026-01-11"));                  // two Sundays share a row
    expect(yearSvg(base({ weekStart: 1 })).includes('>Mo</text>')).toBe(true);
  });
  it("draws explicit zeros dashed and filled levels from the palette", () => {
    const svg = yearSvg(base({ levels: new Map([["2026-03-03", 0], ["2026-03-04", 4]]) }));
    expect(/data-date="2026-03-03"[^>]*stroke-dasharray/.test(svg)).toBe(true);
    expect(/data-date="2026-03-04"[^>]*fill="#004000"/.test(svg)).toBe(true);
  });
  it("escapes user-provided text so no markup can be injected", () => {
    const svg = yearSvg(base({ title: `</text><script>alert(1)</script><img onerror="x">` }));
    expect(svg).not.toContain("<script");
    expect(svg).not.toContain("<img");
    expect(svg).toContain("&lt;script&gt;");
  });
});
