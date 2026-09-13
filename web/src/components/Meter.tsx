// A need meter (GDD 4.2): 0 to 100, with the hardship line drawn where it is.

export function Meter({
  label,
  value,
  threshold = 20,
}: {
  label: string;
  value: number;
  threshold?: number;
}) {
  const v = Math.max(0, Math.min(100, value));
  const low = v < threshold;
  return (
    <div className="flex items-center gap-3" role="meter" aria-valuenow={v} aria-valuemin={0} aria-valuemax={100} aria-label={label}>
      <span className="w-20 text-sm">{label}</span>
      <span className="bg-paper-2 border-line relative h-2 flex-1 overflow-hidden rounded-sm border">
        <span
          data-testid="meter-fill"
          className={`absolute inset-y-0 left-0 ${low ? "bg-bad" : "bg-ink-2"}`}
          style={{ width: `${v}%` }}
        />
        <span
          className="bg-accent absolute inset-y-0 w-px opacity-60"
          style={{ left: `${threshold}%` }}
          aria-hidden
        />
      </span>
      <span className={`num w-12 text-right text-sm ${low ? "text-bad" : ""}`}>{v.toFixed(0)}</span>
    </div>
  );
}
