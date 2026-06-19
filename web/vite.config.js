import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Config JS simples para evitar o loader TS/esbuild no build do Windows.
export default defineConfig(() => {
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
      },
    },
  };
});
