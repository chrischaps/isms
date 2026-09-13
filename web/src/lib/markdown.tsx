// The smallest Markdown the copy files use: paragraphs, **bold**, *italics*,
// and `code`. Copy is ours (TDD 14: no user HTML is rendered anywhere), so a
// tiny renderer beats a dependency.

import type { ReactNode } from "react";

function inline(text: string, key: string): ReactNode[] {
  const out: ReactNode[] = [];
  const re = /(\*\*[^*]+\*\*|\*[^*]+\*|`[^`]+`)/g;
  let last = 0;
  let i = 0;
  for (const m of text.matchAll(re)) {
    const at = m.index ?? 0;
    if (at > last) out.push(text.slice(last, at));
    const tok = m[0];
    if (tok.startsWith("**")) out.push(<strong key={`${key}-${i}`}>{tok.slice(2, -2)}</strong>);
    else if (tok.startsWith("`")) out.push(<code key={`${key}-${i}`}>{tok.slice(1, -1)}</code>);
    else out.push(<em key={`${key}-${i}`}>{tok.slice(1, -1)}</em>);
    last = at + tok.length;
    i += 1;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

export function Markdown({ text, className }: { text: string; className?: string }) {
  const paragraphs = text
    .split(/\n\s*\n/)
    .map((p) => p.trim())
    .filter(Boolean);
  return (
    <div className={className}>
      {paragraphs.map((p, i) => (
        <p key={i} className="mt-3 first:mt-0">
          {inline(p.replace(/\n/g, " "), `p${i}`)}
        </p>
      ))}
    </div>
  );
}
