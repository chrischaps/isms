// The theme choice (docs/style.md §4.5): light, follow the system, or dark.
// A segmented radio group; the choice is per browser (lib/theme).

import { useTheme, type ThemePref } from "../lib/theme";

const OPTIONS: { value: ThemePref; label: string }[] = [
  { value: "light", label: "Light" },
  { value: "system", label: "System" },
  { value: "dark", label: "Dark" },
];

export function ThemeControl() {
  const [pref, set] = useTheme();
  return (
    <fieldset className="bg-surface border-line m-0 inline-flex rounded-pill border p-[3px]" aria-label="Theme">
      <legend className="sr-only">Theme</legend>
      {OPTIONS.map((o) => (
        <label
          key={o.value}
          className={`cursor-pointer rounded-pill px-3 py-1.5 text-sm font-bold ${pref === o.value ? "bg-ink text-bg" : "text-muted hover:text-ink"}`}
        >
          <input type="radio" name="theme" value={o.value} checked={pref === o.value} onChange={() => set(o.value)} className="sr-only" />
          {o.label}
        </label>
      ))}
    </fieldset>
  );
}
