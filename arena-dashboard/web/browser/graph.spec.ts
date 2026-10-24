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

test("narrowViewportKeepsTheHeaderClearOfTheFirstArena", async ({ page }) => {
  await page.setViewportSize({ width: 620, height: 900 });
  await page.goto(stack!.url);
  const panel = arenaPanel(page, ORDERS_ARENA);
  await expect(panel).toBeVisible();

  const header = await page.getByTestId("dashboard-header").boundingBox();
  const arena = await panel.boundingBox();

  expect(header).not.toBeNull();
  expect(arena).not.toBeNull();
  expect(header!.height).toBeGreaterThan(60);
  expect(arena!.y).toBeGreaterThanOrEqual(header!.y + header!.height);

  const actions = await page.getByTestId("server-actions").boundingBox();
  const title = await page.getByRole("heading", { level: 1 }).boundingBox();
  expect(actions).not.toBeNull();
  expect(actions!.y).toBeLessThan(title!.y + title!.height);
  expect(actions!.x + actions!.width).toBeGreaterThan(header!.width - 40);
});

test("arenaStateChipStopsShortOfTheZoomControls", async ({ page }) => {
  await page.setViewportSize({ width: 620, height: 900 });
  await page.goto(stack!.url);
  const panel = arenaPanel(page, ORDERS_ARENA);
  await expect(panel).toBeVisible();

  const chip = await panel.getByTestId(`arena-state-${ORDERS_ARENA}`).boundingBox();
  const controls = await panel.getByTestId("recenter-graph").boundingBox();
  const zoomOut = await panel.getByTestId("zoom-out-graph").boundingBox();

  expect(chip).not.toBeNull();
  expect(controls).not.toBeNull();
  expect(chip!.x + chip!.width).toBeLessThanOrEqual(zoomOut!.x);
});
