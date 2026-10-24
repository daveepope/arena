import path from "node:path";

import { defineConfig } from "@playwright/test";

function runfile(variable: string): string {
  const relative = process.env[variable];
  if (!relative) throw new Error(`${variable} is not set`);
  const runfiles = process.env.JS_BINARY__RUNFILES;
  if (!runfiles) throw new Error("JS_BINARY__RUNFILES is not set");
  return path.join(runfiles, relative);
}

export default defineConfig({
  testDir: "./browser",
  fullyParallel: false,
  workers: 1,
  forbidOnly: true,
  retries: 0,
  timeout: 30_000,
  expect: { timeout: 10_000 },
  reporter: [["list"]],
  use: {
    headless: true,
    launchOptions: {
      executablePath: runfile("ARENA_CHROME_BINARY"),
      args: ["--no-sandbox", "--disable-gpu", "--disable-dev-shm-usage"],
    },
  },
});
