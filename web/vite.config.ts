import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

// Em dev, o backend Axum roda em :8080. O proxy encaminha /api para lá,
// então o cookie de sessão (same-origin via proxy) funciona sem CORS.
export default defineConfig(({ mode }) => {
  // O frontend mora em `web/`, mas este repo centraliza o `.env` na raiz.
  // Com `envDir: ".."`, variaveis `VITE_*` da raiz ficam disponiveis em dev/build.
  const envDir = "..";
  const env = loadEnv(mode, path.resolve(__dirname, envDir), "");
  const contactEmail = env.VITE_CONTACT_EMAIL?.trim() || env.WEB_PUSH_CONTACT_EMAIL?.trim() || "";

  return {
    envDir,
    define: {
      __CONTACT_EMAIL__: JSON.stringify(contactEmail),
    },
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
  };
});
