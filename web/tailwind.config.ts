import type { Config } from "tailwindcss";

// Tokens espelhando o tema "Pastel Futebol/Copa" da versão Dioxus.
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        bg: "#f4f7f0",
        mint: { DEFAULT: "#a8e6cf", dark: "#5fbf9f" },
        yellow: { DEFAULT: "#ffe08a", dark: "#f4c95d" },
        sky: { DEFAULT: "#a0d2eb", dark: "#6fb6de" },
        ink: { DEFAULT: "#2d3a3a", muted: "#6b7a7a" },
        card: "#ffffff",
        danger: { DEFAULT: "#ff8c8c", bg: "#ffe7e7" },
        success: "#5fbf9f",
      },
      borderRadius: {
        sm: "10px",
        md: "16px",
        lg: "20px",
        pill: "999px",
      },
      boxShadow: {
        card: "0 4px 20px rgba(45, 58, 58, 0.08)",
        "card-hover": "0 8px 28px rgba(45, 58, 58, 0.12)",
        glow: "0 0 0 6px rgba(168, 230, 207, 0.16)",
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
