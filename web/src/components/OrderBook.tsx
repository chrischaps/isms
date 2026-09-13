// Order-book depth as a small hand-rolled SVG (TDD 11): bids to the left of
// the spread, asks to the right, cumulative quantity as height.

import { credits } from "../api/client";

export type Level = { price: number; qty: number };

function cumulative(levels: Level[], ascending: boolean): { price: number; total: number }[] {
  const sorted = [...levels].sort((a, b) => (ascending ? a.price - b.price : b.price - a.price));
  let total = 0;
  return sorted.map((l) => {
    total += l.qty;
    return { price: l.price, total };
  });
}

export function OrderBook({ bids, asks, width = 320, height = 96 }: { bids: Level[]; asks: Level[]; width?: number; height?: number }) {
  const b = cumulative(bids, false); // best bid first
  const a = cumulative(asks, true); // best ask first
  const max = Math.max(1, b.at(-1)?.total ?? 0, a.at(-1)?.total ?? 0);
  const half = width / 2;
  const bar = (side: { price: number; total: number }[], leftToRight: boolean) => {
    const n = side.length || 1;
    const w = half / n;
    return side.map((l, i) => {
      const h = (l.total / max) * height;
      const x = leftToRight ? half + i * w : half - (i + 1) * w;
      return <rect key={`${l.price}-${i}`} x={x} y={height - h} width={Math.max(1, w - 1)} height={h} />;
    });
  };
  const bestBid = b[0]?.price;
  const bestAsk = a[0]?.price;
  return (
    <figure className="m-0">
      <svg width={width} height={height} role="img" aria-label="Order book depth" className="block">
        <g className="fill-good">{bar(b, false)}</g>
        <g className="fill-accent">{bar(a, true)}</g>
        <line x1={half} x2={half} y1={0} y2={height} className="stroke-line" />
      </svg>
      <figcaption className="num text-muted mt-1 flex justify-between text-xs">
        <span>bid {bestBid !== undefined ? credits(bestBid) : "—"}</span>
        <span>ask {bestAsk !== undefined ? credits(bestAsk) : "—"}</span>
      </figcaption>
    </figure>
  );
}
