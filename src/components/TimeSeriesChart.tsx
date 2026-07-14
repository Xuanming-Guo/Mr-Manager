import { useEffect, useRef } from "react";
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";

export interface TimeSeriesChartSeries {
  label: string;
  values: number[];
  stroke: string;
  fill: string;
  formatValue: (value: number) => string;
}

interface TimeSeriesChartProps {
  timestampsMs: number[];
  series: TimeSeriesChartSeries[];
  formatAxisValue: (value: number) => string;
  ariaLabel: string;
  compact?: boolean;
  height?: number;
  fixedMaximum?: number;
  minimumDynamicMaximum?: number;
  className?: string;
}

export function TimeSeriesChart({
  timestampsMs,
  series,
  formatAxisValue,
  ariaLabel,
  compact = false,
  height = 210,
  fixedMaximum,
  minimumDynamicMaximum = 1,
  className = "",
}: TimeSeriesChartProps) {
  const chartRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const target = chartRef.current;
    if (!target || timestampsMs.length === 0 || series.length === 0) return;

    const data = [
      timestampsMs.map((timestamp) => timestamp / 1_000),
      ...series.map((item) => item.values),
    ] as uPlot.AlignedData;
    const chartHeight = compact ? 92 : height;
    const plot = new uPlot(
      {
        width: Math.max(260, target.clientWidth),
        height: chartHeight,
        cursor: { drag: { x: false, y: false } },
        legend: { show: !compact },
        scales: {
          x: { time: true },
          y: {
            range: (_plot, min, max) => [
              0,
              fixedMaximum ?? Math.max(minimumDynamicMaximum, max ?? min ?? minimumDynamicMaximum),
            ],
          },
        },
        axes: compact
          ? [{ show: false }, { show: false }]
          : [
              { stroke: "#78889c", grid: { stroke: "rgba(148, 163, 184, 0.10)" } },
              {
                stroke: "#78889c",
                grid: { stroke: "rgba(148, 163, 184, 0.12)" },
                values: (_plot, values) => values.map((value) => formatAxisValue(value)),
                size: 72,
              },
            ],
        series: [
          {},
          ...series.map((item) => ({
            label: item.label,
            stroke: item.stroke,
            width: 2,
            fill: item.fill,
            value: (_plot: uPlot, value: number | null) =>
              value === null ? "N/A" : item.formatValue(value),
          })),
        ],
      },
      data,
      target,
    );
    const observer = new ResizeObserver(([entry]) => {
      if (entry) {
        plot.setSize({ width: Math.max(260, entry.contentRect.width), height: chartHeight });
      }
    });
    observer.observe(target);

    return () => {
      observer.disconnect();
      plot.destroy();
    };
  }, [compact, fixedMaximum, formatAxisValue, height, minimumDynamicMaximum, series, timestampsMs]);

  return (
    <div
      className={`time-series-chart${compact ? "compact" : ""}${className ? ` ${className}` : ""}`}
      aria-label={ariaLabel}
    >
      <div className="time-series-chart-heading">
        {series.map((item) => {
          const latest = item.values.at(-1) ?? 0;
          return (
            <span key={item.label}>
              <i style={{ background: item.stroke }} />
              {item.label} {item.formatValue(latest)}
            </span>
          );
        })}
      </div>
      <div ref={chartRef} />
    </div>
  );
}
