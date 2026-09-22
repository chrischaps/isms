// Segmented control (docs/style.md §10 Work): a few named levels — low,
// normal, high — as one bar of choices, the chosen one in the accent-soft
// pill. A radiogroup to a screen reader, so its label names what it sets
// ("Effort at Legacy Farm No. 1"); a cost line beneath says what the choice
// costs, from the preset, never from the client.

export function Segmented<V extends string>({
  label,
  value,
  options,
  onChange,
  testId,
}: {
  label: string;
  value: V;
  options: { value: V; label: string }[];
  onChange: (v: V) => void;
  testId?: string;
}) {
  return (
    <div role="radiogroup" aria-label={label} data-testid={testId} className="border-line bg-surface inline-flex max-w-full rounded-md border p-0.5">
      {options.map((o) => {
        const on = o.value === value;
        return (
          <button
            key={o.value}
            type="button"
            role="radio"
            aria-checked={on}
            onClick={() => onChange(o.value)}
            className={[
              "min-h-[40px] flex-1 rounded-[8px] px-3.5 text-sm transition-colors",
              on ? "bg-accent-soft text-accent font-bold" : "text-muted hover:text-ink",
            ].join(" ")}
          >
            {o.label}
          </button>
        );
      })}
    </div>
  );
}
