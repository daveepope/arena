import { expect, test, type Page } from "@playwright/test";
import type { ChildProcessWithoutNullStreams } from "node:child_process";

import {
  CATALOG_ARENA,
  FAULTED_ARENA,
  ORDERS_ARENA,
  PAGE_BACKGROUND,
  startDashboardServer,
  startDemoLifecycle,
  stopProcess,
} from "./arenaStack";

const PROFILED_COMPONENT = "caterpillar-service";
const HOTSPOT_FUNCTION = "regex::Regex::captures";

const ORDERS_SUBJECTS = [
  "duchess-db (postgres)",
  "tweedledee-service (http)",
  "rabbit-hole-events (kafka)",
  "queen-of-hearts-db (postgres)",
  "mad-hatter-service (http)",
  "white-rabbit-queue (rabbitmq)",
  "cheshire-api",
  "dormouse-worker",
];

let server: ChildProcessWithoutNullStreams | null = null;
let demo: ChildProcessWithoutNullStreams | null = null;

test.afterEach(() => {
  stopProcess(demo);
  stopProcess(server);
  demo = null;
  server = null;
});

function arenaPanel(page: Page, arenaId: string) {
  return page.getByTestId(`arena-panel-${arenaId}`);
}

function profilerLabel(page: Page) {
  return arenaPanel(page, CATALOG_ARENA).getByTestId(`profiler-label-${PROFILED_COMPONENT}`);
}

function flamegraphFrames(page: Page) {
  return page.frameLocator('[data-testid="cpu-flamegraph"]').locator("#frames g");
}

test("arenaLifecycleStreamsToTheDashboardAndAProfiledComponentReportsItsFlamegraph", async ({
  page,
}) => {
  const started = await startDashboardServer();
  server = started.child;
  await page.goto(started.url);

  await expect(page.getByRole("heading", { name: "Arena Topology Dashboard" })).toBeVisible();
  const background = await page
    .locator("#root > div")
    .first()
    .evaluate((element) => getComputedStyle(element).backgroundColor);
  expect(background).toBe(PAGE_BACKGROUND);
  await expect(page.getByText("connected", { exact: true })).toBeVisible();

  demo = await startDemoLifecycle(started.url);

  const orders = arenaPanel(page, ORDERS_ARENA);
  await expect(orders.getByRole("button", { name: /arena_open/ })).toBeVisible();
  await expect(orders.locator(".react-flow__node")).toHaveCount(ORDERS_SUBJECTS.length);
  for (const label of ORDERS_SUBJECTS) {
    await expect(orders.locator(".react-flow__node").filter({ hasText: label })).toHaveCount(1);
  }

  for (const arenaId of [ORDERS_ARENA, CATALOG_ARENA, FAULTED_ARENA]) {
    await expect(page.getByRole("heading", { name: `arena ${arenaId}`, exact: true })).toBeVisible();
  }

  await expect(page.getByTestId(`event-log-${ORDERS_ARENA}`)).toContainText("duchess-db (postgres)");

  const playbooks = page.getByTestId(`playbooks-${ORDERS_ARENA}`);
  await expect(playbooks.getByTestId("playbook-apply-schema-migrations")).toBeVisible();
  await expect(playbooks.getByTestId("playbook-reset-tea-party-db")).toBeVisible();
  await playbooks.getByTestId("playbook-apply-schema-migrations").click();
  await expect(page.getByText("kind: playbook")).toBeVisible();
  await page.getByRole("button", { name: "Close" }).click();

  await expect(profilerLabel(page)).toHaveText("cpu profiler started");
  await expect(profilerLabel(page)).toBeDisabled();

  await orders.locator(".react-flow__node").filter({ hasText: "duchess-db (postgres)" }).first().click();
  await expect(page.getByRole("heading", { name: "duchess-db (postgres)" })).toBeVisible();
  await expect(page.getByText("kind: dependency")).toBeVisible();
  await page.getByRole("button", { name: "Close" }).click();
  await expect(page.getByText("kind: dependency")).toBeHidden();

  demo.stdin.end();

  await expect(arenaPanel(page, CATALOG_ARENA).getByRole("button", { name: /arena_closed/ })).toBeVisible();
  await expect(profilerLabel(page)).toHaveText("cpu profile report");
  await expect(profilerLabel(page)).toBeEnabled();

  await profilerLabel(page).click();

  await expect(page.getByText(`cpu profile · ${PROFILED_COMPONENT}`)).toBeVisible();
  await expect(flamegraphFrames(page).first()).toBeAttached();
  await expect(page.getByTestId(`cpu-hotspot-${HOTSPOT_FUNCTION}`)).toBeVisible();

  const flamegraph = page.frameLocator('[data-testid="cpu-flamegraph"]');
  await expect(flamegraph.locator("#unzoom")).toBeHidden();
  await flamegraphFrames(page).filter({ hasText: HOTSPOT_FUNCTION }).first().click();
  await expect(flamegraph.locator("#unzoom")).toBeVisible();
});
