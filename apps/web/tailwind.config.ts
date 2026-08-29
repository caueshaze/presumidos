import type { Config } from "tailwindcss";

// Tokens "Pastel Futebol/Copa" via CSS variables (canais RGB) — claro + dark.
// As variáveis são definidas em src/index.css (:root e .dark).
const withVar = (name: string) => `rgb(var(${name}) / <alpha-value>)`;

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        bg: withVar("--color-bg"),
        mint: { DEFAULT: withVar("--color-mint"), dark: withVar("--color-mint-dark") },
        yellow: { DEFAULT: withVar("--color-yellow"), dark: withVar("--color-yellow-dark") },
        sky: { DEFAULT: withVar("--color-sky"), dark: withVar("--color-sky-dark") },
        ink: { DEFAULT: withVar("--color-ink"), muted: withVar("--color-ink-muted") },
        card: withVar("--color-card"),
        danger: { DEFAULT: withVar("--color-danger"), bg: withVar("--color-danger-bg") },
        success: withVar("--color-success"),
        "accent-fg": withVar("--color-accent-fg"),
        secondary: {
          DEFAULT: withVar("--color-secondary"),
          hover: withVar("--color-secondary-hover"),
        },
      },
      borderRadius: {
        sm: "10px",
        md: "16px",
        lg: "20px",
        pill: "999px",
      },
      boxShadow: {
        card: "var(--shadow-card)",
        "card-hover": "var(--shadow-card-hover)",
        glow: "0 0 0 6px rgb(var(--color-mint) / 0.18)",
      },
      fontFamily: {
        heading: ['"Fredoka"', '"Segoe UI"', "sans-serif"],
        body: ['"Nunito Sans"', '"Segoe UI"', "sans-serif"],
      },
      transitionTimingFunction: {
        smooth: "cubic-bezier(0.22, 1, 0.36, 1)",
      },
      keyframes: {
        floatBlob: {
          "0%, 100%": { transform: "translate(0, 0) scale(1)" },
          "50%": { transform: "translate(20px, -30px) scale(1.08)" },
        },
      },
      animation: {
        floatBlob: "floatBlob 14s ease-in-out infinite",
      },
    },
  },
  plugins: [],
} satisfies Config;
