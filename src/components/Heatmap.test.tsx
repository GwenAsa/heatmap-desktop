import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MonthView, YearView, type GridProps } from "./Heatmap";
import type { DayAgg, Tracker } from "../types";

const tracker: Tracker = { id: "t", name: "Study", type: "duration", unit: "min", goal: 60, color: "#3ddc84",
  activeDays: 0b0111110, startDate: null, endDate: null, archived: false, sort: 0, goalMode: "daily", weeklyTarget: 7, freezesPerMonth: 0 };
const base = (aggs: DayAgg[] = [], over: Partial<GridProps> = {}): GridProps => ({
  tracker, aggs: new Map(aggs.map((a) => [a.date, a])), today: "2025-10-15", selected: "2025-10-15",
  weekStart: 0, bgHex: "#0a0a0b", onSelect: vi.fn(), ...over });
afterEach(cleanup);
const buttons = () => Array.from(screen.getByTestId("grid").children) as HTMLElement[];

describe("every real date shows its day number", () => {
  it("October 2025: 31 cells labelled 1..31, with Oct 1 in the Wednesday column", () => {
    render(<MonthView {...base()} year={2025} month={9} />);
    const real = buttons().filter((b) => b.tagName === "BUTTON");
    expect(real.map((b) => b.textContent)).toEqual(Array.from({ length: 31 }, (_, i) => String(i + 1)));
    expect(buttons().findIndex((b) => b.dataset.date === "2025-10-01")).toBe(3); // three blank spacers first
    expect(buttons().slice(0, 3).every((b) => b.tagName === "SPAN" && b.textContent === "")).toBe(true);
  });
  it("respects a Monday week start", () => {
    render(<MonthView {...base([], { weekStart: 1 })} year={2025} month={9} />);
    expect(buttons().findIndex((b) => b.dataset.date === "2025-10-01")).toBe(2);
  });
  it("February in a leap year has 29 numbered cells; a common year has 28", () => {
    render(<MonthView {...base()} year={2024} month={1} />);
    expect(buttons().filter((b) => b.tagName === "BUTTON")).toHaveLength(29);
    cleanup();
    render(<MonthView {...base()} year={2025} month={1} />);
    expect(buttons().filter((b) => b.tagName === "BUTTON")).toHaveLength(28);
  });
  it("annual view numbers all 366 days of 2024 and 365 of 2025, even on the busiest intensity", () => {
    const a: DayAgg = { date: "2024-07-04", value: 90, count: 2, level: 4 };
    render(<YearView {...base([a])} year={2024} />);
    const real = buttons().filter((b) => b.tagName === "BUTTON");
    expect(real).toHaveLength(366);
    expect(real.find((b) => b.dataset.date === "2024-07-04")!.textContent).toBe("4");
    expect(real.find((b) => b.dataset.date === "2024-02-29")!.textContent).toBe("29");
    cleanup();
    render(<YearView {...base()} year={2025} />);
    expect(buttons().filter((b) => b.tagName === "BUTTON")).toHaveLength(365);
  });
});

describe("daily states are distinguishable without colour alone", () => {
  it("separates data, explicit zero, missing, rest day, today and future", () => {
    const aggs: DayAgg[] = [{ date: "2025-10-02", value: 60, count: 1, level: 4 }, { date: "2025-10-03", value: 0, count: 1, level: 0 }];
    render(<MonthView {...base(aggs)} year={2025} month={9} />);
    const by = (d: string) => buttons().find((b) => b.dataset.date === d)!;
    expect(by("2025-10-02").className).toContain("fill");
    expect(by("2025-10-02").style.background).not.toBe("");
    expect(by("2025-10-03").className).toContain("zero");
    expect(by("2025-10-03").getAttribute("aria-label")).toContain("recorded zero");
    expect(by("2025-10-06").className).not.toMatch(/zero|fill|has/); // missing: plain
    expect(by("2025-10-04").getAttribute("aria-label")).toContain("rest day"); // Saturday
    expect(by("2025-10-15").className).toContain("today");
    expect(by("2025-10-20").getAttribute("aria-label")).toContain("future");
  });
});

describe("interaction", () => {
  it("click selects a date; arrow keys move by day and week across month boundaries", () => {
    const onSelect = vi.fn();
    render(<MonthView {...base([], { onSelect, selected: "2025-10-31" })} year={2025} month={9} />);
    const cell = buttons().find((b) => b.dataset.date === "2025-10-31")!;
    fireEvent.click(cell);
    expect(onSelect).toHaveBeenLastCalledWith("2025-10-31");
    fireEvent.keyDown(cell, { key: "ArrowRight" });
    expect(onSelect).toHaveBeenLastCalledWith("2025-11-01", true);
    fireEvent.keyDown(cell, { key: "ArrowDown" });
    expect(onSelect).toHaveBeenLastCalledWith("2025-11-07", true);
    fireEvent.keyDown(cell, { key: "ArrowUp" });
    expect(onSelect).toHaveBeenLastCalledWith("2025-10-24", true);
  });
  it("exposes exactly one tab stop", () => {
    render(<YearView {...base()} year={2025} />);
    expect(buttons().filter((b) => b.tabIndex === 0)).toHaveLength(1);
  });
});
