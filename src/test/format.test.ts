import { describe, expect, it } from "vitest";
import { refreshInterval } from "../app/queries";
import { formatBytes, formatDuration, formatPercent } from "../lib/format";

describe("formatBytes", () => {
  it.each([
    [0, "0 B"],
    [-1, "0 B"],
    [Number.NaN, "0 B"],
    [512, "512 B"],
    [1536, "1.5 KB"],
    [5 * 1024 ** 3, "5.0 GB"],
  ])("formats %s bytes as %s", (value, expected) => {
    expect(formatBytes(value)).toBe(expected);
  });

  it("honors precision for scaled values", () => {
    expect(formatBytes(1536, 2)).toBe("1.50 KB");
  });
});

describe("other presentation helpers", () => {
  it("clamps negative percentages without hiding values over 100", () => {
    expect(formatPercent(-4)).toBe("0.0%");
    expect(formatPercent(112.25)).toBe("112.3%");
  });

  it("formats durations at minute, hour, and day boundaries", () => {
    expect(formatDuration(59)).toBe("0m");
    expect(formatDuration(3_660)).toBe("1h 1m");
    expect(formatDuration(90_060)).toBe("1d 1h 1m");
  });
});

describe("refreshInterval", () => {
  it.each([
    ["normal", 2_500],
    ["fast", 1_000],
    [undefined, 2_500],
  ] as const)("maps %s mode to %i ms", (mode, interval) => {
    expect(refreshInterval(mode)).toBe(interval);
  });
});
