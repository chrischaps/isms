// A time series on uPlot (TDD 11; docs/style.md §7.16): ticks along x, read
// as days and hours (lib/when), one or more series along y. uPlot paints a
// canvas, which cannot read `var()`, so the chart reads its colours from the
// tokens at mount and again when the theme changes. A series named "you" is
// the player's own and is always drawn in ink, 2px, with its last point marked.

import { useEffect, useRef } from "react";
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";
import { onThemeChange } from "../lib/theme";
import { hourName } from "../lib/when";

/** Axis steps in ticks: quarter days up to weeks, so a gridline is always a day or a round hour. */
const STEPS = [6, 12, 24, 48, 120, 240, 480, 1200];

export type Series = { label: string; values: (number | null)[]; you?: boolean };

/** The chart tokens as painted right now, read from the element so an island's theme wins. */
export function chartColors(el: Element) {
  const cs = getComputedStyle(el);
  const v = (name: string) => cs.getPropertyValue(name).trim();
  return {
    series: [v("--chart-1"), v("--chart-2"), v("--chart-3"), v("--chart-4"), v("--chart-5")],
    you: v("--chart-you"),
    grid: v("--chart-grid"),
    axis: v("--muted"),
  };
}

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
    let plot: uPlot | null = null;
    const draw = () => {
      plot?.destroy();
      const width = el.clientWidth || 480;
      const c = chartColors(el);
      const axis = { stroke: c.axis, grid: { stroke: c.grid, width: 1 }, ticks: { stroke: c.grid, width: 1 }, font: "12px Atkinson Hyperlegible, system-ui, sans-serif" };
      plot = new uPlot(
        {
          width,
          height,
          scales: { x: { time: false } },
          axes: [
            { ...axis, incrs: STEPS, values: (_u, vals) => vals.map((v) => (v % 24 === 0 ? `Day ${v / 24 + 1}` : hourName(v % 24))) },
            { ...axis, values: (_u, vals) => vals.map(format) },
          ],
          series: [
            {},
            ...series.map((s, i) =>
              s.you
                ? { label: s.label, stroke: c.you, width: 2, points: { show: false } }
                : { label: s.label, stroke: c.series[i % c.series.length], width: 1.5 },
            ),
          ],
          legend: { show: series.length > 1 },
        },
        [ticks, ...series.map((s) => s.values)] as uPlot.AlignedData,
        el,
      );
    };
    draw();
    const off = onThemeChange(draw);
    return () => {
      off();
      plot?.destroy();
    };
  }, [ticks, series, height, format]);
  return <div ref={ref} className="w-full" data-testid="timeseries" />;
}
