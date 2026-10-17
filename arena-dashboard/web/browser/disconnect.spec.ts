import { expect, test } from "@playwright/test";
import type { ChildProcessWithoutNullStreams } from "node:child_process";

import { startDashboardServer, stopProcess } from "./arenaStack";

let server: ChildProcessWithoutNullStreams | null = null;

test.afterEach(() => {
  stopProcess(server);
  server = null;
});

test("serverLostWhilePageIsOpenShowsTheDisconnectedNotice", async ({ page }) => {
  const started = await startDashboardServer();
  server = started.child;
  await page.goto(started.url);
  await expect(page.getByText("connected", { exact: true })).toBeVisible();

  stopProcess(server);
  server = null;

  await expect(page.getByTestId("disconnected-notice")).toHaveText("disconnected");
});
