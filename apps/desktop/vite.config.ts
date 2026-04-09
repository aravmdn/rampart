import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = fileURLToPath(new URL(".", import.meta.url));

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@rampart/shared-ui": resolve(rootDir, "../../packages/shared-ui/src/index.ts"),
    },
  },
  server: {
    fs: {
      allow: [resolve(rootDir, "../../packages/shared-ui")],
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: "./src/test/setup.ts",
  },
});
