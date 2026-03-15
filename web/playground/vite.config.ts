import path from "path";
import { defineConfig } from "vite";
import checker from "vite-plugin-checker";

export default defineConfig({
  test: {
    exclude: ["e2e/**", "node_modules/**"],
  },
  base: "/",
  plugins: [checker({ typescript: true })],
  resolve: {
    alias: {
      "syntaqlite": path.resolve(__dirname, "../syntaqlite-js/src/index.ts"),
    },
  },
  server: {
    port: 8080,
  },
});
