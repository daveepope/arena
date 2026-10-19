import "./testSetup";

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CpuProfilePopover, CpuProfileView } from "./CpuProfilePanel";
import { snapToViewportCorner } from "./FloatingPanel";
import { useDashboardStore } from "./store";
import type { RenderedProfile } from "./types";

function lastProfile() {
  const profiles = useDashboardStore.getState().profiles;
  return profiles.length === 0 ? null : profiles[profiles.length - 1];
}


const SAMPLE_PROFILE: RenderedProfile = {
  svg: "<svg id='flamegraph'></svg>",
  hotspots: [
    { function: "compute", self_count: 42, self_pct: 84.0, severity: "critical" },
    { function: "idle", self_count: 8, self_pct: 16.0, severity: "medium" },
  ],
};

function respondWithProfile(profile: RenderedProfile | null): void {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue({
      ok: profile !== null,
      json: () => Promise.resolve(profile),
    }),
  );
}

beforeEach(() => {
  useDashboardStore.setState({ profiles: [] });
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("CpuProfileView", () => {
  it("pending fetch shows the loading state", () => {
    vi.stubGlobal("fetch", vi.fn().mockReturnValue(new Promise(() => {})));

    render(<CpuProfileView componentId="svc" isFullScreen={false} />);

    expect(screen.getByTestId("cpu-profile-loading")).toBeTruthy();
  });

  it("missing profile shows the unavailable state", async () => {
    respondWithProfile(null);

    render(<CpuProfileView componentId="svc" isFullScreen={false} />);

    await waitFor(() => expect(screen.getByTestId("cpu-profile-unavailable")).toBeTruthy());
  });

  it("failed fetch shows the unavailable state", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));

    render(<CpuProfileView componentId="svc" isFullScreen={false} />);

    await waitFor(() => expect(screen.getByTestId("cpu-profile-unavailable")).toBeTruthy());
  });

  it("loaded profile renders the flamegraph and every hotspot", async () => {
    respondWithProfile(SAMPLE_PROFILE);

    render(<CpuProfileView componentId="svc" isFullScreen={false} />);

    await waitFor(() => expect(screen.getByTestId("cpu-flamegraph")).toBeTruthy());
    expect(screen.getByTestId("cpu-hotspot-compute")).toBeTruthy();
    expect(screen.getByTestId("cpu-hotspot-idle")).toBeTruthy();
    expect(screen.getByText("84.0%")).toBeTruthy();
  });

  it("component id is escaped in the request path", async () => {
    respondWithProfile(SAMPLE_PROFILE);

    render(<CpuProfileView componentId="svc/one" isFullScreen={false} />);

    await waitFor(() => expect(screen.getByTestId("cpu-flamegraph")).toBeTruthy());
    expect(fetch).toHaveBeenCalledWith("/profile/cpu/svc%2Fone");
  });

  it("filter hides the hotspot rows that do not match", async () => {
    respondWithProfile(SAMPLE_PROFILE);
    render(<CpuProfileView componentId="svc" isFullScreen={false} />);
    await waitFor(() => expect(screen.getByTestId("cpu-hotspot-compute")).toBeTruthy());

    fireEvent.change(screen.getByLabelText("Filter functions"), { target: { value: "idle" } });

    expect(screen.queryByTestId("cpu-hotspot-compute")).toBeNull();
    expect(screen.getByTestId("cpu-hotspot-idle")).toBeTruthy();
  });

  it("filter matching nothing shows the empty hotspots message", async () => {
    respondWithProfile(SAMPLE_PROFILE);
    render(<CpuProfileView componentId="svc" isFullScreen={false} />);
    await waitFor(() => expect(screen.getByTestId("cpu-hotspot-compute")).toBeTruthy());

    fireEvent.change(screen.getByLabelText("Filter functions"), { target: { value: "nothing" } });

    expect(screen.getByTestId("cpu-hotspots-empty")).toBeTruthy();
  });

  it("clear button restores every hotspot row", async () => {
    respondWithProfile(SAMPLE_PROFILE);
    render(<CpuProfileView componentId="svc" isFullScreen={false} />);
    await waitFor(() => expect(screen.getByTestId("cpu-hotspot-compute")).toBeTruthy());
    fireEvent.change(screen.getByLabelText("Filter functions"), { target: { value: "idle" } });

    fireEvent.click(screen.getByText("Clear"));

    expect(screen.getByTestId("cpu-hotspot-compute")).toBeTruthy();
    expect(screen.queryByText("Clear")).toBeNull();
  });

  it("layout button toggles between stacked and side by side", async () => {
    respondWithProfile(SAMPLE_PROFILE);
    render(<CpuProfileView componentId="svc" isFullScreen={false} />);
    await waitFor(() => expect(screen.getByTestId("cpu-flamegraph")).toBeTruthy());

    fireEvent.click(screen.getByText("Side by side"));

    expect(screen.getByText("Stacked")).toBeTruthy();
  });

  it("hovering a hotspot row and leaving it does not disturb the rows", async () => {
    respondWithProfile(SAMPLE_PROFILE);
    render(<CpuProfileView componentId="svc" isFullScreen={false} />);
    await waitFor(() => expect(screen.getByTestId("cpu-hotspot-compute")).toBeTruthy());

    fireEvent.mouseEnter(screen.getByTestId("cpu-hotspot-compute"));
    fireEvent.mouseLeave(screen.getByTestId("cpu-hotspot-compute"));

    expect(screen.getByTestId("cpu-hotspot-compute")).toBeTruthy();
  });

  it("loaded flamegraph applies the filter and the hover highlight", async () => {
    respondWithProfile(SAMPLE_PROFILE);
    render(<CpuProfileView componentId="svc" isFullScreen={false} />);
    await waitFor(() => expect(screen.getByTestId("cpu-flamegraph")).toBeTruthy());

    fireEvent.load(screen.getByTestId("cpu-flamegraph"));
    fireEvent.change(screen.getByLabelText("Filter functions"), { target: { value: "compute" } });
    fireEvent.mouseEnter(screen.getByTestId("cpu-hotspot-compute"));

    expect(screen.getByTestId("cpu-hotspot-compute")).toBeTruthy();
  });

  it("panel view offers a link to the full screen view", async () => {
    respondWithProfile(SAMPLE_PROFILE);

    render(<CpuProfileView componentId="svc one" isFullScreen={false} />);

    await waitFor(() => expect(screen.getByText("Open in new tab")).toBeTruthy());
    expect(screen.getByText("Open in new tab").getAttribute("href")).toBe("/?profile=svc%20one");
  });

  it("full screen view omits the new tab link", async () => {
    respondWithProfile(SAMPLE_PROFILE);

    render(<CpuProfileView componentId="svc" isFullScreen />);

    await waitFor(() => expect(screen.getByTestId("cpu-flamegraph")).toBeTruthy());
    expect(screen.queryByText("Open in new tab")).toBeNull();
  });
});

describe("CpuProfilePopover", () => {
  it("no profile view selected renders nothing", () => {
    const { container } = render(<CpuProfilePopover />);

    expect(container.firstChild).toBeNull();
  });

  it("selected component renders the panel titled with its id", async () => {
    respondWithProfile(SAMPLE_PROFILE);
    useDashboardStore.getState().openProfileView("svc", { x: 10, y: 10 });

    render(<CpuProfilePopover />);

    await waitFor(() => expect(screen.getByTestId("cpu-profile-panel")).toBeTruthy());
    expect(screen.getByText("cpu profile · svc")).toBeTruthy();
  });

  it("close button clears the profile view", async () => {
    respondWithProfile(SAMPLE_PROFILE);
    useDashboardStore.getState().openProfileView("svc", { x: 10, y: 10 });
    render(<CpuProfilePopover />);
    await waitFor(() => expect(screen.getByTestId("cpu-profile-panel")).toBeTruthy());

    fireEvent.click(screen.getByText("Close"));

    expect(lastProfile()).toBeNull();
  });

  it("escape clears the profile view", async () => {
    respondWithProfile(SAMPLE_PROFILE);
    useDashboardStore.getState().openProfileView("svc", { x: 10, y: 10 });
    render(<CpuProfilePopover />);
    await waitFor(() => expect(screen.getByTestId("cpu-profile-panel")).toBeTruthy());

    fireEvent.keyDown(window, { key: "Escape" });

    expect(lastProfile()).toBeNull();
  });

  it("pin button keeps the panel open when the backdrop would be clicked", async () => {
    respondWithProfile(SAMPLE_PROFILE);
    useDashboardStore.getState().openProfileView("svc", { x: 10, y: 10 });
    render(<CpuProfilePopover />);
    await waitFor(() => expect(screen.getByTestId("cpu-profile-panel")).toBeTruthy());

    fireEvent.click(screen.getByLabelText("Pin"));

    expect(lastProfile()?.isPinned).toBe(true);
    expect(screen.getByLabelText("Unpin")).toBeTruthy();
  });

  it("unpinned panel closes when the backdrop is clicked", async () => {
    respondWithProfile(SAMPLE_PROFILE);
    useDashboardStore.getState().openProfileView("svc", { x: 10, y: 10 });
    const { container } = render(<CpuProfilePopover />);
    await waitFor(() => expect(screen.getByTestId("cpu-profile-panel")).toBeTruthy());

    fireEvent.click(container.querySelector(".fixed.inset-0")!);

    expect(lastProfile()).toBeNull();
  });

  it("position beyond the viewport is clamped into view", async () => {
    respondWithProfile(SAMPLE_PROFILE);
    useDashboardStore.getState().openProfileView("svc", { x: 100000, y: 100000 });

    render(<CpuProfilePopover />);

    await waitFor(() => expect(screen.getByTestId("cpu-profile-panel")).toBeTruthy());
    const panel = screen.getByTestId("cpu-profile-panel") as HTMLElement;
    expect(Number.parseInt(panel.style.left, 10)).toBeLessThan(window.innerWidth);
    expect(Number.parseInt(panel.style.top, 10)).toBeLessThan(window.innerHeight);
  });

  it("resizeHandlesAreOfferedOnEveryEdgeAndCorner", () => {
    useDashboardStore.getState().openProfileView("svc", { x: 10, y: 10 });
    render(<CpuProfilePopover />);

    for (const edge of ["n", "s", "e", "w", "ne", "nw", "se", "sw"]) {
      expect(screen.getByTestId(`cpu-profile-resize-${edge}`)).toBeTruthy();
    }
  });

  it("dragHandleIsOfferedOnTheHeader", () => {
    useDashboardStore.getState().openProfileView("svc", { x: 10, y: 10 });
    render(<CpuProfilePopover />);

    expect(screen.getByTestId("cpu-profile-drag-handle")).toBeTruthy();
  });
});

describe("snapToViewportCorner", () => {
  const viewport = { width: 1000, height: 800 };

  it.each([
    ["top left", { left: 20, top: 20 }, { left: 8, top: 8 }],
    ["top right", { left: 900, top: 20 }, { left: 892, top: 8 }],
    ["bottom left", { left: 20, top: 700 }, { left: 8, top: 692 }],
    ["bottom right", { left: 900, top: 700 }, { left: 892, top: 692 }],
  ])("%s snaps to the corner", (_name, position, expected) => {
    expect(snapToViewportCorner({ ...position, width: 100, height: 100 }, viewport)).toEqual(expected);
  });

  it("centreOfTheViewportIsLeftWhereItIs", () => {
    expect(snapToViewportCorner({ left: 400, top: 400, width: 100, height: 100 }, viewport)).toEqual({
      left: 400,
      top: 400,
    });
  });
});

describe("CpuHotspots runtime frames", () => {
  const MIXED: RenderedProfile = {
    svg: "<svg></svg>",
    hotspots: [
      { function: "UNMANAGED_CODE_TIME", self_count: 400, self_pct: 90.9, severity: "critical" },
      { function: "compute", self_count: 40, self_pct: 9.1, severity: "low" },
    ],
    application_hotspots: [
      { function: "compute", self_count: 40, self_pct: 100.0, severity: "critical" },
    ],
    runtime_frames: [
      { language: "dotnet", frames: ["UNMANAGED_CODE_TIME", "CPU_TIME"] },
      { language: "java", frames: ["java."] },
    ],
  };

  it("appOnlyToggled_ranksWithoutRuntimeFrames", async () => {
    respondWithProfile(MIXED);
    render(<CpuProfileView componentId="svc" isFullScreen={false} />);
    await screen.findByTestId("cpu-hotspots");
    expect(screen.getByTestId("cpu-hotspot-UNMANAGED_CODE_TIME")).toBeDefined();

    fireEvent.click(screen.getByTestId("cpu-hide-runtime-frames"));

    expect(screen.queryByTestId("cpu-hotspot-UNMANAGED_CODE_TIME")).toBeNull();
    expect(screen.getByTestId("cpu-hotspot-compute").textContent).toContain("100");
  });

  it("appOnlyToggledOff_bringsTheRuntimeFramesBack", async () => {
    respondWithProfile(MIXED);
    render(<CpuProfileView componentId="svc" isFullScreen={false} />);
    await screen.findByTestId("cpu-hotspots");

    fireEvent.click(screen.getByTestId("cpu-hide-runtime-frames"));
    fireEvent.click(screen.getByTestId("cpu-hide-runtime-frames"));

    expect(screen.getByTestId("cpu-hotspot-UNMANAGED_CODE_TIME")).toBeDefined();
  });

  it("infoToggled_namesTheHiddenFramesPerLanguage", async () => {
    respondWithProfile(MIXED);
    render(<CpuProfileView componentId="svc" isFullScreen={false} />);
    await screen.findByTestId("cpu-hotspots");

    fireEvent.click(screen.getByTestId("cpu-runtime-frames-info-toggle"));

    const info = screen.getByTestId("cpu-runtime-frames-info");
    expect(info.textContent).toContain("dotnet");
    expect(info.textContent).toContain("UNMANAGED_CODE_TIME");
    expect(info.textContent).toContain("java");
  });

  it("appOnlyToggled_leavesTheFlamegraphAlone", async () => {
    respondWithProfile(MIXED);
    render(<CpuProfileView componentId="svc" isFullScreen={false} />);
    const frame = await screen.findByTestId("cpu-flamegraph");
    const before = frame.getAttribute("srcDoc") ?? frame.getAttribute("srcdoc");

    fireEvent.click(screen.getByTestId("cpu-hide-runtime-frames"));

    const after = frame.getAttribute("srcDoc") ?? frame.getAttribute("srcdoc");
    expect(after).toBe(before);
  });
});

describe("CpuHotspots layout", () => {
  it("hotspotTableFixesTheColumnsSoSelfAndShareStayVisible", async () => {
    respondWithProfile(SAMPLE_PROFILE);
    render(<CpuProfileView componentId="svc" isFullScreen={false} />);

    const table = await screen.findByTestId("cpu-hotspots");

    expect(table.className).toContain("table-fixed");
    expect(table.className).toContain("w-full");
    const headers = table.querySelectorAll("th");
    expect(headers[1].textContent).toBe("Self");
    expect(headers[2].textContent).toBe("Share");
    const functionCell = table.querySelector("tbody td");
    expect(functionCell?.className).toContain("truncate");
  });
});
