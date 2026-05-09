import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";

// Four entry points: companion (index.html), popup (popup.html), settings, wizard.
// All build into ../dist/ which tauri.conf.json points to via frontendDist.
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
        settings: resolve(__dirname, "settings.html"),
        wizard: resolve(__dirname, "wizard.html"),
      },
    },
  },
  // DEV-MODE COLD-START FIX (Phase 4 gap closure 04-09 — UAT test 4 + 14 root cause #A).
  // vite has 4 entries but esbuild auto-discovery only walks index.html;
  // popup.html / wizard.html / settings.html each pay a 10-30s cold-bundle cost on
  // first WebView GET. Listing them in optimizeDeps.entries forces esbuild to
  // pre-bundle all shared dependencies at dev-server start, so the first GET to
  // popup.html / wizard.html / settings.html is fast.
  //
  // optimizeDeps.include explicitly enumerates heavy npm deps that benefit from
  // being pre-bundled (CommonJS-to-ESM conversion is slow on cold transform).
  // Production builds bypass this entirely (cargo tauri build runs `vite build`
  // which uses Rollup, not esbuild).
  optimizeDeps: {
    entries: [
      "index.html",
      "popup.html",
      "settings.html",
      "wizard.html",
    ],
    include: [
      "react",
      "react-dom",
      "react-dom/client",
      "framer-motion",
      "@tauri-apps/api/core",
      "@tauri-apps/api/event",
      "@tauri-apps/api/webviewWindow",
      // Phase 4 gap closure (04-11): pre-bundle the new shell plugin so the
      // first cold transform of Settings/UpdateModal doesn't reintroduce the
      // warmup delay 04-09 closed.
      "@tauri-apps/plugin-shell",
    ],
  },
});
