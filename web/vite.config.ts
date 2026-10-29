import { defineConfig } from "vite";

export default defineConfig({
  build: {
    target: "es2022",
    // No source maps in production: smaller downloads for school networks.
    sourcemap: false,
  },
  server: { host: "127.0.0.1", port: 5173 },
});
