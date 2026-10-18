import { expect, test } from "@playwright/test";
import type { ChildProcessWithoutNullStreams } from "node:child_process";

import {
  ORDERS_ARENA,
  startArenaStack,
  startDashboardServer,
  stopArenaStack,
  stopProcess,
  type ArenaStack,
} from "./arenaStack";

let server: ChildProcessWithoutNullStreams | null = null;
let stack: ArenaStack | null = null;

test.afterEach(() => {
  stopProcess(server);
  server = null;
  stopArenaStack(stack);
  stack = null;
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

test("serverLostWithAnArenaOnScreenMarksThatCanvasStale", async ({ page }) => {
  stack = await startArenaStack();
  await page.goto(stack.url);
  const panel = page.getByTestId(`arena-panel-${ORDERS_ARENA}`);
  await expect(panel).toBeVisible();

  stopArenaStack(stack);
  stack = null;

  await expect(panel.getByTestId(`stale-notice-${ORDERS_ARENA}`)).toContainText(
    "last known state",
  );
  await expect(panel.getByRole("button", { name: "Replay" })).toBeVisible();
});
