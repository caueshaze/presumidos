import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

// Em dev, o backend Axum roda em :8080. O proxy encaminha /api para lá,
// então o cookie de sessão (same-origin via proxy) funciona sem CORS.
export default defineConfig(() => {
  // O contato público usa fallback do backend em runtime; aqui deixamos
  // apenas um valor opcional para builds locais sem depender do `.env` da raiz.
  const contactEmail = (process.env.VITE_CONTACT_EMAIL ?? process.env.WEB_PUSH_CONTACT_EMAIL ?? "")
    .trim();

  return {
    root: __dirname,
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
        // Assets internos também são servidos pelo backend. Sem este proxy,
        // URLs persistidas como /media/assets/... caem no fallback do Vite e
        // retornam o index.html em vez da imagem.
        "/media": {
          target: "http://127.0.0.1:8080",
          changeOrigin: true,
        },
      },
    },
  };
});
