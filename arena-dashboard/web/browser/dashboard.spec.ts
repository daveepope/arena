import { expect, test, type Page } from "@playwright/test";

import {
  CATALOG_ARENA,
  FAULTED_ARENA,
  NESTED_DEPENDENCY_EDGE_SUFFIX,
  ORDERS_ARENA,
  PAGE_BACKGROUND,
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

test("packagedServerRendersDashboardHeading", async ({ page }) => {
  await expect(page.getByRole("heading", { name: "Arena Topology Dashboard" })).toBeVisible();
});

test("embeddedStylesheetAppliesDarkBackground", async ({ page }) => {
  const background = await page
    .locator("#root > div")
    .first()
    .evaluate((element) => getComputedStyle(element).backgroundColor);
  expect(background).toBe(PAGE_BACKGROUND);
});

test("eventStreamOpenShowsConnectedStatus", async ({ page }) => {
  await expect(page.getByText("connected", { exact: true })).toBeVisible();
});

test("openArenasRenderOnePanelEach", async ({ page }) => {
  for (const arenaId of [ORDERS_ARENA, CATALOG_ARENA, FAULTED_ARENA]) {
    await expect(page.getByRole("heading", { name: `arena ${arenaId}`, exact: true })).toBeVisible();
  }
});

test("openArenaRendersGraphNodesForEverySubject", async ({ page }) => {
  const panel = arenaPanel(page, ORDERS_ARENA);
  for (const label of [
    "orders-db (postgres)",
    "pricing-service (http)",
    "orders-events (kafka)",
    "payments-db (postgres)",
    "fraud-check-service (http)",
    "refunds-queue (rabbitmq)",
    "orders-api",
    "payments-worker",
  ]) {
    await expect(panel.locator(".react-flow__node").filter({ hasText: label }).first()).toBeVisible();
  }
});

test("openArenaRendersGraphEdges", async ({ page }) => {
  const panel = arenaPanel(page, ORDERS_ARENA);
  await expect(panel.locator(".react-flow__edge")).not.toHaveCount(0);
  await expect(
    panel.locator(`.react-flow__edge[data-id$="${NESTED_DEPENDENCY_EDGE_SUFFIX}"]`),
  ).toHaveCount(1);
});

test("faultedArenaRendersFaultedState", async ({ page }) => {
  await expect(arenaPanel(page, FAULTED_ARENA).getByRole("button", { name: /arena_faulted/ })).toBeVisible();
});

test("graphNodeClickRendersDetailPopover", async ({ page }) => {
  const panel = arenaPanel(page, ORDERS_ARENA);
  await panel.locator(".react-flow__node").filter({ hasText: "orders-db (postgres)" }).first().click();
  await expect(page.getByRole("heading", { name: "orders-db (postgres)" })).toBeVisible();
  await expect(page.getByText("kind: dependency")).toBeVisible();
});

test("arenaEventsPopulateEventLog", async ({ page }) => {
  await expect(page.getByTestId(`event-log-${ORDERS_ARENA}`)).toContainText("orders-db (postgres)");
});
