import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

// Vite configuration tuned for Tauri, per the Tauri v2 frontend guide.
// The non-obvious settings are the ones that matter:
//  - `clearScreen: false` keeps Rust compiler errors on screen; Vite would
//    otherwise wipe them the moment it reloads.
//  - `strictPort` makes a port collision fail loudly instead of silently
//    serving on a port `devUrl` is not pointing at, which presents as a blank window.
//  - ignoring `src-tauri/**` stops Vite from thrashing on every Rust rebuild.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**", "**/crates/**", "**/target/**"] },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    // Tauri uses Chromium on Windows and WebKit on macOS/Linux.
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13",
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});
