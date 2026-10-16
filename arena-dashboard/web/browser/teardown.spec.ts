import { expect, test, type Page } from "@playwright/test";

import {
  CATALOG_ARENA,
  startArenaStack,
  stopArenaStack,
  type ArenaStack,
} from "./arenaStack";

function arenaPanel(page: Page, arenaId: string) {
  return page.getByTestId(`arena-panel-${arenaId}`);
}

let stack: ArenaStack | null = null;

test.beforeAll(async () => {
  stack = await startArenaStack();
});

test.afterAll(() => {
  stopArenaStack(stack);
  stack = null;
});

test.beforeEach(async ({ page }) => {
  await page.goto(stack!.url);
});

test("arenaTeardownStreamsClosedStateToOpenPage", async ({ page }) => {
  await expect(arenaPanel(page, CATALOG_ARENA).getByRole("button", { name: /arena_open/ })).toBeVisible();
  stack!.demo.stdin.end();
  await expect(arenaPanel(page, CATALOG_ARENA).getByRole("button", { name: /arena_closed/ })).toBeVisible();
});
