// The theme choice (docs/style.md §4.5): light, dark, or follow the OS. Stored
// per browser; index.html applies it before first paint, this module keeps
// <html data-theme> and the store in step afterwards. Storage may be missing
// or blocked (private windows), so every access is wrapped.

import { useSyncExternalStore } from "react";

export type ThemePref = "light" | "system" | "dark";

export const THEME_KEY = "isms.theme";

const listeners = new Set<() => void>();

function read(): ThemePref {
  try {
    const t = localStorage.getItem(THEME_KEY);
    return t === "light" || t === "dark" ? t : "system";
  } catch {
    return "system";
  }
}

function apply(pref: ThemePref) {
  const root = document.documentElement;
  if (pref === "system") delete root.dataset.theme;
  else root.dataset.theme = pref;
}

export function getTheme(): ThemePref {
  return read();
}

export function setTheme(pref: ThemePref) {
  try {
    if (pref === "system") localStorage.removeItem(THEME_KEY);
    else localStorage.setItem(THEME_KEY, pref);
  } catch {
    // No storage: the choice lasts for this page only.
  }
  apply(pref);
  for (const l of listeners) l();
}

function subscribe(cb: () => void) {
  listeners.add(cb);
  return () => {
    listeners.delete(cb);
  };
}

export function useTheme(): [ThemePref, (p: ThemePref) => void] {
  const pref = useSyncExternalStore(subscribe, read, () => "system" as ThemePref);
  return [pref, setTheme];
}

/** Whether the page currently renders dark, whichever way it was chosen. */
export function isDark(): boolean {
  const t = document.documentElement.dataset.theme;
  if (t === "dark") return true;
  if (t === "light") return false;
  return typeof matchMedia === "function" && matchMedia("(prefers-color-scheme: dark)").matches;
}

/** Calls `cb` whenever the rendered theme may have changed: a choice or an OS switch. */
export function onThemeChange(cb: () => void): () => void {
  const unsub = subscribe(cb);
  const mq = typeof matchMedia === "function" ? matchMedia("(prefers-color-scheme: dark)") : null;
  mq?.addEventListener("change", cb);
  return () => {
    unsub();
    mq?.removeEventListener("change", cb);
  };
}
