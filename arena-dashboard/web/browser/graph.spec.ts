import { expect, test, type Page } from "@playwright/test";

import {
  NESTED_DEPENDENCY_EDGE_SUFFIX,
  ORDERS_ARENA,
  startArenaStack,
  stopArenaStack,
  type ArenaStack,
} from "./arenaStack";

let stack: ArenaStack | null = null;

test.beforeAll(async () => {
  stack = await startArenaStack();
});

test.afterAll(() => {
  stopArenaStack(stack);
  stack = null;
});

function arenaPanel(page: Page, arenaId: string) {
  return page.getByTestId(`arena-panel-${arenaId}`);
}

test("openArenaDrawsAnEdgeBetweenEveryLinkedSubject", async ({ page }) => {
  await page.goto(stack!.url);
  const orders = arenaPanel(page, ORDERS_ARENA);

  await expect(orders.locator(".react-flow__edge")).not.toHaveCount(0);
  await expect(
    orders.locator(`.react-flow__edge[data-id$="${NESTED_DEPENDENCY_EDGE_SUFFIX}"]`),
  ).toHaveCount(1);
});
