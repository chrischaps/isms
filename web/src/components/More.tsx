// More (docs/style.md §7.10): a disclosure for layer-4 content — rules,
// budgets, multipliers, history. The label names what is inside ("More about
// today's work"), never just "More". Closed by default on phones; a card that
// is otherwise short may open it on md+ with `open`.

import type { ReactNode } from "react";

export function More({ summary, open, children, testId }: { summary: string; open?: boolean; children: ReactNode; testId?: string }) {
  return (
    <details className="more border-line mt-3 border-t border-dashed pt-2.5" open={open} data-testid={testId}>
      <summary className="text-accent flex min-h-7 cursor-pointer items-center gap-1.5 text-sm font-bold">{summary}</summary>
      <div className="mt-2">{children}</div>
    </details>
  );
}
