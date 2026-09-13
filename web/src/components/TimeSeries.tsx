// A time series on uPlot (TDD 11): ticks along x, one or more series along y.

import { useEffect, useRef } from "react";
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";

export type Series = { label: string; values: (number | null)[] };

export function TimeSeries({
  ticks,
  series,
  height = 160,
  format = (v: number) => v.toFixed(2),
}: {
  ticks: number[];
  series: Series[];
  height?: number;
  format?: (v: number) => string;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const width = el.clientWidth || 480;
    const palette = ["#8b3a2f", "#3b6d3a", "#4a463f", "#a86b12"];
    const plot = new uPlot(
      {
        width,
        height,
        scales: { x: { time: false } },
        axes: [
          { label: "tick", stroke: "#7a746a", grid: { stroke: "#d9d3c7" } },
          { stroke: "#7a746a", grid: { stroke: "#d9d3c7" }, values: (_u, vals) => vals.map(format) },
        ],
        series: [
          {},
          ...series.map((s, i) => ({ label: s.label, stroke: palette[i % palette.length], width: 1.5 })),
        ],
        legend: { show: series.length > 1 },
      },
      [ticks, ...series.map((s) => s.values)] as uPlot.AlignedData,
      el,
    );
    return () => plot.destroy();
  }, [ticks, series, height, format]);
  return <div ref={ref} className="w-full" data-testid="timeseries" />;
}
