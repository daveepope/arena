import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: ".",
    emptyOutDir: false,
    rollupOptions: {
      input: "src/main.tsx",
      output: {
        entryFileNames: "bundle.js",
        format: "es",
      },
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/testSetup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
