import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

// Em dev, o backend Axum roda em :8080. O proxy encaminha /api para lá,
// então o cookie de sessão (same-origin via proxy) funciona sem CORS.
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  server: {
    port: 5173,
    proxy: {
      "/api": {
        target: "http://127.0.0.1:8080",
        changeOrigin: true,
      },
    },
  },
});
