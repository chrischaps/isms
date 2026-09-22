// Isms — Tailwind preset that maps utilities onto tokens.css.
// Use in web/tailwind.config.ts:  presets: [require('./tailwind.preset.js')]
// Every color is a CSS variable, so dark mode needs no `dark:` variants:
// `bg-surface text-ink border-line` are correct in both themes.
// `dark:` variants are still available (class strategy on <html data-theme>)
// but should be rare — reach for a token first.

/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: ['selector', '[data-theme="dark"]'],
  theme: {
    // Replace, don't extend: an agent should never find `bg-slate-500` compiling.
    colors: {
      transparent: 'transparent',
      current: 'currentColor',
      bg: 'var(--bg)',
      surface: { DEFAULT: 'var(--surface)', 2: 'var(--surface-2)' },
      ink: 'var(--ink)',
      muted: 'var(--muted)',
      faint: 'var(--faint)',
      line: { DEFAULT: 'var(--line)', strong: 'var(--line-strong)' },
      accent: { DEFAULT: 'var(--accent)', hover: 'var(--accent-hover)', soft: 'var(--accent-soft)', on: 'var(--on-accent)' },
      good: { DEFAULT: 'var(--good)', fill: 'var(--good-fill)', soft: 'var(--good-soft)' },
      attn: { DEFAULT: 'var(--attn)', fill: 'var(--attn-fill)', soft: 'var(--attn-soft)' },
      crit: { DEFAULT: 'var(--crit)', fill: 'var(--crit-fill)', soft: 'var(--crit-soft)' },
      info: { DEFAULT: 'var(--info)', soft: 'var(--info-soft)' },
      chart: { 1: 'var(--chart-1)', 2: 'var(--chart-2)', 3: 'var(--chart-3)', 4: 'var(--chart-4)', 5: 'var(--chart-5)', grid: 'var(--chart-grid)', you: 'var(--chart-you)' },
    },
    fontFamily: {
      display: ['Outfit', 'Segoe UI', 'system-ui', 'sans-serif'],
      body: ['Atkinson Hyperlegible', 'Segoe UI', 'system-ui', 'sans-serif'],
      mono: ['ui-monospace', 'Cascadia Mono', 'JetBrains Mono', 'Menlo', 'monospace'],
    },
    fontSize: {
      xs: ['0.8125rem', { lineHeight: '1.4' }],
      sm: ['0.875rem', { lineHeight: '1.45' }],
      md: ['1rem', { lineHeight: '1.5' }],
      lg: ['1.1875rem', { lineHeight: '1.4' }],
      xl: ['1.375rem', { lineHeight: '1.2' }],
      '2xl': ['1.75rem', { lineHeight: '1.1' }],
      '3xl': ['2rem', { lineHeight: '1.1' }],
    },
    borderRadius: { none: '0', sm: 'var(--r-sm)', md: 'var(--r-md)', lg: 'var(--r-lg)', pill: 'var(--r-pill)' },
    boxShadow: { none: 'none', 1: 'var(--shadow-1)', 2: 'var(--shadow-2)' },
    screens: {
      // Mobile first. `sm` is the phone→tablet seam; `md` is where side-by-side layouts appear.
      sm: '480px',
      md: '720px',
      lg: '960px',
    },
    extend: {
      spacing: { touch: 'var(--touch)', gutter: 'var(--gutter)', tabbar: 'var(--tabbar-h)' },
      maxWidth: { page: 'var(--page-max)', 'page-wide': 'var(--page-max-wide)' },
      transitionDuration: { fast: '120ms', base: '180ms' },
      transitionTimingFunction: { out: 'cubic-bezier(.2,.7,.2,1)' },
    },
  },
  corePlugins: { preflight: true },
};
