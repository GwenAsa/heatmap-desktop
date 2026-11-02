import { describe, expect, it } from "vitest";
import { addDays, daysInMonth, dk, isLeap, monthGrid, todayKey, weekColumn, yearGrid } from "./dates";

describe("calendar math", () => {
  it("handles leap years", () => {
    expect([2024, 2000].every(isLeap)).toBe(true);
    expect([1900, 2100, 2023].some(isLeap)).toBe(false);
    expect(daysInMonth(2024, 1)).toBe(29);
    expect(daysInMonth(2025, 1)).toBe(28);
  });
  it("crosses month, year and leap-day boundaries", () => {
    expect(addDays("2024-12-31", 1)).toBe("2025-01-01");
    expect(addDays("2025-01-01", -1)).toBe("2024-12-31");
    expect(addDays("2024-02-28", 2)).toBe("2024-03-01");
    expect(addDays("2023-02-28", 1)).toBe("2023-03-01");
  });
  it("is immune to DST transitions", () => {
    expect(addDays("2024-03-09", 2)).toBe("2024-03-11");
    expect(addDays("2024-03-30", 2)).toBe("2024-04-01");
    expect(addDays("2024-11-02", 2)).toBe("2024-11-04");
  });
  it("uses the local calendar date for today", () => {
    expect(todayKey(new Date(2025, 9, 1, 23, 59))).toBe("2025-10-01");
    expect(todayKey(new Date(2025, 9, 2, 0, 0))).toBe("2025-10-02");
  });
  it("aligns weekdays (2025-10-01 is a Wednesday)", () => {
    expect(weekColumn("2025-10-01", 0)).toBe(3);
    expect(weekColumn("2025-10-01", 1)).toBe(2);
  });
  it("month grid places October 1st in the right weekday column", () => {
    const sun = monthGrid(2025, 9, 0), mon = monthGrid(2025, 9, 1);
    expect(sun.indexOf("2025-10-01")).toBe(3);
    expect(mon.indexOf("2025-10-01")).toBe(2);
    expect(sun.filter(Boolean)).toHaveLength(31);
    expect(monthGrid(2024, 1, 0).filter(Boolean)).toHaveLength(29);
  });
  it("year grid has every date exactly once, in the correct weekday row", () => {
    for (const [y, ws] of [[2024, 0], [2025, 0], [2025, 1], [2026, 1], [2000, 0]] as const) {
      const { cells, cols, labels } = yearGrid(y, ws);
      const real = cells.filter((c): c is string => !!c);
      expect(real).toHaveLength(isLeap(y) ? 366 : 365);
      expect(new Set(real).size).toBe(real.length);
      expect(real[0]).toBe(dk(y, 0, 1));
      expect(real[real.length - 1]).toBe(dk(y, 11, 31));
      cells.forEach((c, i) => { if (c) expect(i % 7).toBe(weekColumn(c, ws)); });
      expect(cols).toBe(Math.ceil(cells.length / 7));
      expect(labels).toHaveLength(12);
    }
  });
});
