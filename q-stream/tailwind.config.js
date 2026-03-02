/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        "qs-dark":          "#04060f",   // near-black, blue-tinted
        "qs-surface":       "#080c1a",   // card background
        "qs-surface-light": "#0d1228",   // elevated surface
        "qs-surface-3":     "#131b38",   // hover state
        "qs-accent":        "#00d4ff",   // neon cyan — primary
        "qs-accent-2":      "#8b5cf6",   // electric purple — secondary
        "qs-accent-light":  "#67e8f9",   // light cyan for text
        "qs-green":         "#06ffa5",   // neon green
        "qs-red":           "#ff3358",   // neon red
        "qs-text":          "#c5d8f0",   // primary text (cool white)
        "qs-text-dim":      "#3b5470",   // muted text
      },
      backdropBlur: {
        xs: "2px",
      },
      fontFamily: {
        mono: ["'JetBrains Mono'", "'Fira Code'", "Consolas", "ui-monospace", "monospace"],
      },
      animation: {
        "pulse-slow":  "pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite",
        "spin-slow":   "spin 8s linear infinite",
        "glow-pulse":  "glow-pulse 2.5s ease-in-out infinite",
        "scan":        "scan 4s linear infinite",
        "float":       "float 3s ease-in-out infinite",
      },
      keyframes: {
        "glow-pulse": {
          "0%, 100%": { boxShadow: "0 0 8px rgba(0,212,255,0.3)" },
          "50%":       { boxShadow: "0 0 24px rgba(0,212,255,0.7), 0 0 48px rgba(0,212,255,0.2)" },
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
      },
      boxShadow: {
        "neon-cyan":   "0 0 12px rgba(0,212,255,0.45), 0 0 28px rgba(0,212,255,0.15)",
        "neon-purple": "0 0 12px rgba(139,92,246,0.45), 0 0 28px rgba(139,92,246,0.15)",
        "neon-green":  "0 0 12px rgba(6,255,165,0.45), 0 0 28px rgba(6,255,165,0.15)",
        "neon-sm":     "0 0 6px rgba(0,212,255,0.5)",
        "inner-glow":  "inset 0 0 24px rgba(0,212,255,0.04)",
      },
    },
  },
  plugins: [],
};
