import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const chartMocks = vi.hoisted(() => ({
  destroy: vi.fn(),
  setSize: vi.fn(),
  disconnect: vi.fn(),
}));

vi.mock("uplot", () => ({
  default: class MockUPlot {
    setSize = chartMocks.setSize;
    destroy = chartMocks.destroy;
  },
}));

import { TimeSeriesChart } from "../components/TimeSeriesChart";
import { buildRecordingTimelineData } from "../lib/diagnostics-chart";

class ResizeObserverMock {
  observe() {}
  disconnect = chartMocks.disconnect;
}

describe("System Diagnostics timeline", () => {
  beforeEach(() => {
    vi.stubGlobal("ResizeObserver", ResizeObserverMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.clearAllMocks();
  });

  it("builds bounded CPU and RAM percentage series", () => {
    const samples = Array.from({ length: 121 }, (_, index) => ({
      collectedAtMs: index + 1,
      system: {
        cpuTotalPercent: index === 120 ? 140 : index === 1 ? -5 : 25,
        memory: { usedBytes: 50, totalBytes: 100 },
      },
    }));

    const result = buildRecordingTimelineData(samples);

    expect(result.timestampsMs).toHaveLength(120);
    expect(result.timestampsMs[0]).toBe(2);
    expect(result.cpuPercent[0]).toBe(0);
    expect(result.cpuPercent.at(-1)).toBe(100);
    expect(result.ramPercent.every((value) => value === 50)).toBe(true);
  });

  it("renders an accessible chart and cleans up plot resources", () => {
    const { unmount } = render(
      <TimeSeriesChart
        timestampsMs={[1, 2]}
        series={[
          {
            label: "CPU",
            values: [10, 20],
            stroke: "#4bd6de",
            fill: "rgba(75, 214, 222, 0.12)",
            formatValue: (value) => `${value}%`,
          },
        ]}
        formatAxisValue={(value) => `${value}%`}
        fixedMaximum={100}
        ariaLabel="System diagnostics timeline test"
      />,
    );

    expect(screen.getByLabelText("System diagnostics timeline test")).toHaveTextContent("CPU 20%");
    unmount();
    expect(chartMocks.disconnect).toHaveBeenCalledOnce();
    expect(chartMocks.destroy).toHaveBeenCalledOnce();
  });
});
