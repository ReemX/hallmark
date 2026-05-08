import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";

// Two entry points: popup.html (popup webview) and index.html (companion webview).
// Both build into ../dist/ which tauri.conf.json points to via frontendDist.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    rollupOptions: {
      input: {
        companion: resolve(__dirname, "index.html"),
        popup: resolve(__dirname, "popup.html"),
      },
    },
  },
});
