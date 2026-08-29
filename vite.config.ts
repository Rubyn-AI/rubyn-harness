import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";

export default defineConfig({
  plugins: [react()],
  resolve: {
    // The ESM barrel makes Rollup open every Lucide icon at once on macOS.
    // The equivalent CJS bundle is a single file and still tree-shakes the
    // named icons used by Harness without exhausting the process file limit.
    alias: {
      "lucide-react": fileURLToPath(new URL("./node_modules/lucide-react/dist/cjs/lucide-react.js", import.meta.url)),
    },
  },
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: { target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13" }
});
