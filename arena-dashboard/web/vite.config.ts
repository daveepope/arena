import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig(({ mode }) => ({
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
  resolve: mode === "test" ? { preserveSymlinks: true } : {},
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    coverage: {
      provider: "v8",
      reporter: ["lcovonly"],
      reportsDirectory: "coverage-raw",
      include: ["src/**/*.{ts,tsx}"],
      exclude: ["src/**/*.test.{ts,tsx}", "src/testSetup.ts", "src/main.tsx"],
      reportOnFailure: true,
    },
  },
}));
