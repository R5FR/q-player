/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      // All colors reference CSS custom properties so dark/light theme works
      // The `<alpha-value>` placeholder enables Tailwind opacity modifiers
      colors: {
        "qs-dark":          "rgb(var(--qs-bg) / <alpha-value>)",
        "qs-surface":       "rgb(var(--qs-surface) / <alpha-value>)",
        "qs-surface-light": "rgb(var(--qs-surface-2) / <alpha-value>)",
        "qs-surface-3":     "rgb(var(--qs-surface-3) / <alpha-value>)",
        "qs-accent":        "rgb(var(--qs-accent) / <alpha-value>)",
        "qs-accent-2":      "rgb(var(--qs-accent-2) / <alpha-value>)",
        "qs-accent-light":  "rgb(var(--qs-accent-light) / <alpha-value>)",
        "qs-green":         "rgb(var(--qs-green) / <alpha-value>)",
        "qs-red":           "rgb(var(--qs-red) / <alpha-value>)",
        "qs-text":          "rgb(var(--qs-text) / <alpha-value>)",
        "qs-text-dim":      "rgb(var(--qs-text-dim) / <alpha-value>)",
      },
      fontFamily: {
        sans:      ["'Barlow'", "system-ui", "sans-serif"],
        condensed: ["'Barlow Condensed'", "system-ui", "sans-serif"],
        display:   ["'Bebas Neue'", "system-ui", "sans-serif"],
        mono:      ["'Azeret Mono'", "'JetBrains Mono'", "Consolas", "ui-monospace", "monospace"],
      },
      backdropBlur: {
        xs: "2px",
      },
      animation: {
        "pulse-slow":  "pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite",
        "spin-slow":   "spin 8s linear infinite",
        "glow-pulse":  "glow-pulse 2.8s ease-in-out infinite",
        "lime-flicker":"lime-flicker 4s ease-in-out infinite",
        "scan":        "scan 4s linear infinite",
        "float":       "float 3s ease-in-out infinite",
        "slide-up":    "slide-up 0.4s cubic-bezier(0.16, 1, 0.3, 1) both",
      },
      keyframes: {
        "glow-pulse": {
          "0%, 100%": { boxShadow: "0 0 10px rgb(var(--qs-accent) / 0.35)" },
          "50%":       { boxShadow: "0 0 28px rgb(var(--qs-accent) / 0.75), 0 0 60px rgb(var(--qs-accent) / 0.2)" },
        },
        "lime-flicker": {
          "0%, 100%": { opacity: "0.65" },
          "40%":      { opacity: "1" },
          "60%":      { opacity: "0.85" },
        },
        scan: {
          "0%":   { transform: "translateY(-100%)", opacity: "0" },
          "10%":  { opacity: "1" },
          "90%":  { opacity: "1" },
          "100%": { transform: "translateY(300%)", opacity: "0" },
        },
        float: {
          "0%, 100%": { transform: "translateY(0px)" },
          "50%":       { transform: "translateY(-5px)" },
        },
        "slide-up": {
          from: { opacity: "0", transform: "translateY(10px)" },
          to:   { opacity: "1", transform: "translateY(0)" },
        },
      },
      boxShadow: {
        "neon-lime":   "0 0 12px rgb(var(--qs-accent) / 0.55), 0 0 32px rgb(var(--qs-accent) / 0.18)",
        "neon-orange": "0 0 12px rgb(var(--qs-accent-2) / 0.5), 0 0 28px rgb(var(--qs-accent-2) / 0.15)",
        "neon-green":  "0 0 10px rgb(var(--qs-green) / 0.45), 0 0 24px rgb(var(--qs-green) / 0.14)",
        "neon-sm":     "0 0 7px rgb(var(--qs-accent) / 0.55)",
        "inner-glow":  "inset 0 0 28px rgb(var(--qs-accent) / 0.05)",
        "card":        "0 1px 3px rgb(0 0 0 / 0.55), 0 8px 24px rgb(0 0 0 / 0.35)",
        "player":      "0 -1px 48px rgb(0 0 0 / 0.6), 0 -1px 0 rgb(var(--qs-accent) / 0.08)",
      },
    },
  },
  plugins: [],
};
