// SB.1 done gate: the theme choice round-trips through storage and the
// <html data-theme> attribute, and "system" leaves no attribute behind.

import { beforeEach, describe, expect, it } from "vitest";
import { getTheme, isDark, setTheme, THEME_KEY } from "./theme";

describe("theme", () => {
  beforeEach(() => {
    localStorage.clear();
    delete document.documentElement.dataset.theme;
  });

  it("defaults to system with no attribute", () => {
    expect(getTheme()).toBe("system");
    expect(document.documentElement.dataset.theme).toBeUndefined();
  });

  it("stores an explicit choice and applies it to the root", () => {
    setTheme("dark");
    expect(localStorage.getItem(THEME_KEY)).toBe("dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(isDark()).toBe(true);
    setTheme("light");
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(isDark()).toBe(false);
  });

  it("forgets the choice on system", () => {
    setTheme("dark");
    setTheme("system");
    expect(localStorage.getItem(THEME_KEY)).toBeNull();
    expect(document.documentElement.dataset.theme).toBeUndefined();
    expect(getTheme()).toBe("system");
  });

  it("ignores junk in storage", () => {
    localStorage.setItem(THEME_KEY, "sepia");
    expect(getTheme()).toBe("system");
  });
});
