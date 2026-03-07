import path from "path";
import { defineConfig } from "vite";
import checker from "vite-plugin-checker";

export default defineConfig({
  base: "/syntaqlite/",
  plugins: [checker({ typescript: true })],
  resolve: {
    alias: {
      "@syntaqlite/js": path.resolve(__dirname, "../syntaqlite-js/src/index.ts"),
    },
  },
  server: {
    port: 8080,
  },
});
