import { expect, test, type Page } from "@playwright/test";

import { CATALOG_ARENA, startArenaStack, stopArenaStack, type ArenaStack } from "./arenaStack";

let stack: ArenaStack | null = null;

test.beforeEach(async ({ page }) => {
  stack = await startArenaStack();
  await page.goto(stack.url);
});

test.afterEach(() => {
  stopArenaStack(stack);
  stack = null;
});

function arenaPanel(page: Page, arenaId: string) {
  return page.getByTestId(`arena-panel-${arenaId}`);
}

test("arenaSittingOpenIsNotReplayedWithoutAsking", async ({ page }) => {
  const panel = arenaPanel(page, CATALOG_ARENA);
  await expect(panel.getByRole("button", { name: /arena_open/ })).toBeVisible();

  await expect(panel.getByRole("button", { name: "Stop replay" })).toBeHidden();
  await expect(panel.getByRole("button", { name: /arena_open/ })).toBeVisible();
});
