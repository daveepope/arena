import { expect, test, type Page } from "@playwright/test";

import { ORDERS_ARENA, startArenaStack, stopArenaStack, type ArenaStack } from "./arenaStack";

const PROFILED_COMPONENT = "orders-api";
const SAMPLE_FOLDED = "main;handler;compute 42\nmain;handler;idle 8\n";
const DIMMED_OPACITY = "0.15";

let stack: ArenaStack | null = null;

test.beforeAll(async () => {
  stack = await startArenaStack();
});

test.afterAll(() => {
  stopArenaStack(stack);
  stack = null;
});

async function postProfileState(page: Page, state: string, folded?: string): Promise<void> {
  const data: Record<string, string> = { component: PROFILED_COMPONENT, state };
  if (folded !== undefined) data.folded = folded;
  const response = await page.request.post(`${stack!.url}/profile/cpu`, { data });
  expect(response.status()).toBe(202);
}

function profilerLabel(page: Page) {
  return page
    .getByTestId(`arena-panel-${ORDERS_ARENA}`)
    .getByTestId(`profiler-label-${PROFILED_COMPONENT}`);
}

function flamegraphFrames(page: Page) {
  return page.frameLocator('[data-testid="cpu-flamegraph"]').locator("#frames g");
}

async function openProfilePanel(page: Page): Promise<void> {
  await postProfileState(page, "stopped", SAMPLE_FOLDED);
  await expect(profilerLabel(page)).toHaveText("profiler available");
  await profilerLabel(page).click();
  await expect(page.getByTestId("cpu-profile-panel")).toBeVisible();
}

test.beforeEach(async ({ page }) => {
  await page.goto(stack!.url);
  await expect(page.getByText("connected")).toBeVisible();
});

test("samplingProfilerShowsADisabledLabelOnItsNode", async ({ page }) => {
  await postProfileState(page, "started");

  await expect(profilerLabel(page)).toHaveText("profiler sampling");
  await expect(profilerLabel(page)).toBeDisabled();
});

test("availableProfilerOpensTheProfileInContext", async ({ page }) => {
  await openProfilePanel(page);

  await expect(page.getByText(`cpu profile · ${PROFILED_COMPONENT}`)).toBeVisible();
  await expect(flamegraphFrames(page).first()).toBeAttached();
  await expect(page.getByTestId("cpu-hotspot-compute")).toBeVisible();
  await expect(page.getByTestId("cpu-hotspot-idle")).toBeVisible();
});

test("filterDimsTheFramesAndRowsThatDoNotMatch", async ({ page }) => {
  await openProfilePanel(page);

  await page.getByLabel("Filter functions").fill("compute");

  await expect(page.getByTestId("cpu-hotspot-idle")).toBeHidden();
  await expect(page.getByTestId("cpu-hotspot-compute")).toBeVisible();
  const dimmed = await flamegraphFrames(page)
    .filter({ hasText: "idle" })
    .first()
    .evaluate((frame) => (frame as SVGGElement).style.opacity);
  expect(dimmed).toBe(DIMMED_OPACITY);
});

test("clearingTheFilterRestoresEveryFrame", async ({ page }) => {
  await openProfilePanel(page);
  await page.getByLabel("Filter functions").fill("compute");

  await page.getByTestId("cpu-profile-panel").getByRole("button", { name: "Clear" }).click();

  await expect(page.getByTestId("cpu-hotspot-idle")).toBeVisible();
  const restored = await flamegraphFrames(page)
    .filter({ hasText: "idle" })
    .first()
    .evaluate((frame) => (frame as SVGGElement).style.opacity);
  expect(restored).toBe("1");
});

test("hoveringAHotspotRowOutlinesThatFunctionInTheGraph", async ({ page }) => {
  await openProfilePanel(page);

  await page.getByTestId("cpu-hotspot-compute").hover();

  const stroke = await flamegraphFrames(page)
    .filter({ hasText: "compute" })
    .first()
    .locator("rect")
    .evaluate((rect) => (rect as SVGRectElement).style.strokeWidth);
  expect(stroke).toBe("2");
});

test("layoutButtonTogglesTheHotspotTableBesideTheGraph", async ({ page }) => {
  await openProfilePanel(page);

  await page.getByRole("button", { name: "Side by side" }).click();

  await expect(page.getByRole("button", { name: "Stacked" })).toBeVisible();
});

test("pinnedPanelSurvivesInteractingElsewhereInTheView", async ({ page }) => {
  await openProfilePanel(page);

  await page.getByRole("button", { name: "Pin" }).click();
  await page.getByRole("heading", { name: "Arena Topology Dashboard" }).click();

  await expect(page.getByTestId("cpu-profile-panel")).toBeVisible();
  await expect(page.getByRole("button", { name: "Unpin" })).toBeVisible();
});

test("unpinnedPanelClosesWhenClickingAwayFromIt", async ({ page }) => {
  await openProfilePanel(page);

  await page.mouse.click(5, 5);

  await expect(page.getByTestId("cpu-profile-panel")).toBeHidden();
});

test("openInNewTabLandsOnTheFullScreenProfileView", async ({ page, context }) => {
  await openProfilePanel(page);

  const [opened] = await Promise.all([
    context.waitForEvent("page"),
    page.getByRole("link", { name: "Open in new tab" }).click(),
  ]);
  await opened.waitForLoadState();

  await expect(
    opened.getByRole("heading", { name: `cpu profile · ${PROFILED_COMPONENT}` }),
  ).toBeVisible();
  await expect(opened.getByTestId("cpu-hotspot-compute")).toBeVisible();
  await expect(opened.getByRole("link", { name: "Open in new tab" })).toBeHidden();
});
