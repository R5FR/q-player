/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        "qs-dark": "#121218",
        "qs-surface": "#1a1a24",
        "qs-surface-light": "#242432",
        "qs-accent": "#6366f1",
        "qs-accent-light": "#818cf8",
        "qs-green": "#22c55e",
        "qs-text": "#e2e8f0",
        "qs-text-dim": "#94a3b8",
      },
      backdropBlur: {
        xs: "2px",
      },
      animation: {
        "pulse-slow": "pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite",
        "spin-slow": "spin 8s linear infinite",
      },
    },
  },
  plugins: [],
};
