import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  base: "/assets/repo-explorer/",
  plugins: [react()],
  build: {
    outDir: "../../crates/ri-api/assets/repo-explorer",
    emptyOutDir: true,
    rollupOptions: {
      output: {
        assetFileNames: "assets/repo-explorer.[ext]",
        chunkFileNames: "assets/[name].js",
        entryFileNames: "assets/repo-explorer.js",
      },
    },
  },
});
