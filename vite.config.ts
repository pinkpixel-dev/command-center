import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

const packageJson = JSON.parse(
  readFileSync(new URL("./package.json", import.meta.url), "utf-8"),
) as { version: string };

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// Where the self-hosted server listens by default, so the web dev server
// reaches a local one without any extra configuration.
// @ts-expect-error process is a nodejs global
const apiOrigin: string = process.env.COMMAND_CENTER_API ?? "http://127.0.0.1:8787";

/**
 * Two applications, one frontend. `--mode web` builds the bundle the
 * self-hosted server serves; anything else builds the desktop app. The mode
 * picks which platform implementation `@platform` resolves to, so neither
 * bundle carries the other's code.
 */
export default defineConfig(({ mode }) => {
  const web = mode === "web";

  return {
    plugins: [react()],

    resolve: {
      alias: {
        "@platform": fileURLToPath(
          new URL(web ? "./src/lib/ipc-http.ts" : "./src/lib/ipc-tauri.ts", import.meta.url),
        ),
      },
    },

    build: {
      // Kept apart so a web build never overwrites what Tauri bundles.
      outDir: web ? "dist-web" : "dist",
      emptyOutDir: true,
    },

    // package.json is the single source of truth for the version shown in the UI.
    define: {
      __APP_VERSION__: JSON.stringify(packageJson.version),
    },

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent Vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
      port: web ? 1430 : 1420,
      strictPort: true,
      // The web bundle expects the API on its own origin. In development that
      // is this dev server, so calls, uploads, and the event feed are
      // forwarded to wherever the server is actually running.
      proxy: web ? { "/api": { target: apiOrigin, changeOrigin: true } } : undefined,
      host: host || false,
      hmr: host
        ? {
            protocol: "ws",
            host,
            port: 1421,
          }
        : undefined,
      watch: {
        // 3. tell Vite to ignore watching `src-tauri`
        ignored: ["**/src-tauri/**"],
      },
    },

    test: {
      environment: "jsdom",
      globals: true,
      setupFiles: ["./src/test/setup.ts"],
      css: false,
      include: ["src/**/*.test.{ts,tsx}"],
    },
  };
});
