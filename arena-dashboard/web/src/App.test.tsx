import "./testSetup";

import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { App, fullScreenProfileComponentId } from "./App";
import { useDashboardStore } from "./store";
import type { ArenaState } from "./types";

const SLIDER_SETTLE_MS = 400;

class FakeEventSource {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSED = 2;
  static instances: FakeEventSource[] = [];

  readyState = FakeEventSource.CONNECTING;
  onopen: (() => void) | null = null;
  onerror: (() => void) | null = null;
  onmessage: ((evt: { data: string }) => void) | null = null;
  private listeners = new Map<string, Set<() => void>>();

  constructor(public url: string) {
    FakeEventSource.instances.push(this);
  }

  addEventListener(type: string, listener: () => void): void {
    const forType = this.listeners.get(type) ?? new Set();
    forType.add(listener);
    this.listeners.set(type, forType);
  }

  removeEventListener(type: string, listener: () => void): void {
    this.listeners.get(type)?.delete(listener);
  }

  dispatchEvent(type: string): void {
    this.listeners.get(type)?.forEach((listener) => listener());
  }

  close(): void {
    this.readyState = FakeEventSource.CLOSED;
  }
}

function latestEventSource(): FakeEventSource {
  const instance = FakeEventSource.instances.at(-1);
  if (!instance) throw new Error("no EventSource was constructed");
  return instance;
}

function arenaState(overrides: Partial<ArenaState> = {}): ArenaState {
  return {
    id: "arena-1",
    state: "arena_open",
    at: "2026-01-01T00:00:00.000Z",
    dependencies: [],
    components: [],
    faults: [],
    ...overrides,
  };
}

beforeEach(() => {
  FakeEventSource.instances = [];
  (globalThis as unknown as { EventSource: typeof FakeEventSource }).EventSource = FakeEventSource;
  useDashboardStore.setState({
    arenas: new Map(),
    selected: null,
    cpuProfiles: new Map(),
    profileView: null,
  });
  window.history.replaceState(null, "", "/");
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(null, { status: 204 })));
});

afterEach(() => {
  FakeEventSource.instances = [];
  vi.unstubAllGlobals();
});

describe("App", () => {
  it("renders the dashboard heading and starts in connecting status", () => {
    render(<App />);

    expect(screen.getByText("Arena Topology Dashboard")).toBeDefined();
    expect(screen.getByText("connecting")).toBeDefined();
  });

  it("renders a legend explaining state and kind colors", () => {
    render(<App />);

    expect(screen.getByText("started")).toBeDefined();
    expect(screen.getByText("faulted")).toBeDefined();
    expect(screen.getByText("dependency")).toBeDefined();
    expect(screen.getByText("component")).toBeDefined();
  });

  it("shows connected once the event source opens", () => {
    render(<App />);

    act(() => {
      latestEventSource().onopen?.();
    });

    expect(screen.getByText("connected")).toBeDefined();
  });

  it("shows disconnected after missing one heartbeat, even without an error event", () => {
    vi.useFakeTimers();
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onopen?.();
    });
    expect(screen.getByText("connected")).toBeDefined();

    act(() => {
      vi.advanceTimersByTime(1600);
    });

    expect(screen.getByTestId("disconnected-notice")).toBeDefined();
    expect(screen.queryByText("Clear server state")).toBeNull();
    expect(screen.queryByText("Stop server")).toBeNull();

    vi.useRealTimers();
  });

  it("returns to connected as soon as a heartbeat arrives again", () => {
    vi.useFakeTimers();
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onopen?.();
      vi.advanceTimersByTime(1600);
    });
    expect(screen.getByTestId("disconnected-notice")).toBeDefined();

    act(() => {
      source.dispatchEvent("heartbeat");
    });

    expect(screen.getByText("connected")).toBeDefined();

    vi.useRealTimers();
  });

  it("repeated heartbeats within the interval keep the connection marked as connected", () => {
    vi.useFakeTimers();
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onopen?.();
    });

    act(() => {
      vi.advanceTimersByTime(400);
      source.dispatchEvent("heartbeat");
      vi.advanceTimersByTime(400);
      source.dispatchEvent("heartbeat");
      vi.advanceTimersByTime(400);
    });

    expect(screen.getByText("connected")).toBeDefined();

    vi.useRealTimers();
  });

  it("clear server state and stop server buttons appear once connected", () => {
    render(<App />);

    act(() => {
      latestEventSource().onopen?.();
    });

    expect(screen.getByText("Clear server state")).toBeDefined();
    expect(screen.getByText("Stop server")).toBeDefined();
  });

  it("appEventSourceErrorsWhileOpenReportsErrorState", () => {
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.readyState = FakeEventSource.CLOSED;
      source.onerror?.();
    });

    expect(screen.getByText("error")).toBeDefined();
  });

  it("appHeartbeatLapsesReportsDisconnectedState", () => {
    vi.useFakeTimers();
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onopen?.();
    });
    act(() => {
      vi.advanceTimersByTime(1600);
    });

    expect(screen.getByTestId("disconnected-notice")).toBeDefined();
    vi.useRealTimers();
  });

  it("renders an arena panel once a state document is ingested", () => {
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onmessage?.({ data: JSON.stringify(arenaState()) });
    });

    expect(screen.getAllByText("arena-1").length).toBeGreaterThan(0);
  });

  it("new log entries scroll the event pane to the latest line", () => {
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_starting" })) });
    });

    const pane = screen.getByTestId("event-log-arena-1");
    Object.defineProperty(pane, "scrollHeight", { value: 640, configurable: true });
    pane.scrollTop = 0;

    act(() => {
      source.onmessage?.({
        data: JSON.stringify(
          arenaState({
            state: "arena_open",
            dependencies: [{ id: "postgres", state: "started", faults: [], children: [] }],
          }),
        ),
      });
    });

    expect(pane.scrollTop).toBe(640);
  });

  it("arena panel labels the name as an arena", () => {
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onmessage?.({ data: JSON.stringify(arenaState()) });
    });

    const heading = screen.getByRole("heading", { level: 2 });
    expect(heading.textContent).toContain("arena");
    expect(heading.textContent).toContain("arena-1");
  });

  it("does not render the timeline slider before any state has been captured", () => {
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onopen?.();
    });

    expect(screen.queryByRole("slider")).toBeNull();
  });

  it("does not render the timeline slider while the arena is still open", () => {
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_starting" })) });
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_open" })) });
    });

    expect(screen.queryByRole("slider")).toBeNull();
  });

  it("history fetched on mount restores a scrubbable timeline", async () => {
    const history = [
      {
        id: "arena-1",
        states: [
          arenaState({ state: "arena_starting", at: "2026-01-01T00:00:00.000Z" }),
          arenaState({ state: "arena_open", at: "2026-01-01T00:00:01.000Z" }),
          arenaState({ state: "arena_closed", at: "2026-01-01T00:00:02.000Z" }),
        ],
      },
    ];
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(new Response(JSON.stringify(history), { status: 200 })),
    );

    render(<App />);
    await act(async () => {});

    expect(fetch).toHaveBeenCalledWith("/history");
    const slider = screen.getByRole("slider") as HTMLInputElement;
    expect(slider.max).toBe("2");
    expect(screen.getByText("Replay")).toBeDefined();
  });

  it("does not render the replay button while the arena is still open", () => {
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_starting" })) });
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_open" })) });
    });

    expect(screen.queryByText("Replay")).toBeNull();
  });

  it("renders the replay button alongside the timeline slider on a terminal arena", () => {
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_starting" })) });
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_closed" })) });
    });

    expect(screen.getByRole("slider")).toBeDefined();
    expect(screen.getByText("Replay")).toBeDefined();
  });

  it("renders a timeline slider once the arena reaches a terminal state", () => {
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_starting" })) });
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_open" })) });
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_closed" })) });
    });

    const slider = screen.getByRole("slider") as HTMLInputElement;
    expect(slider.max).toBe("2");
    expect(slider.value).toBe("2");
  });

  it("dragging the slider scrubs the arena display back to an earlier recorded state", () => {
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_starting" })) });
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_open" })) });
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_closed" })) });
    });
    expect(screen.getByText(/state: arena_closed/)).toBeDefined();

    act(() => {
      fireEvent.change(screen.getByRole("slider"), { target: { value: "0" } });
    });

    expect(screen.getByText(/state: arena_starting/)).toBeDefined();
  });

  it("replayArena advances the store's replay index through to completion", () => {
    vi.useFakeTimers();
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_starting" })) });
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_open" })) });
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_closed" })) });
    });

    act(() => {
      screen.getByText("Replay").click();
    });
    expect(useDashboardStore.getState().arenas.get("arena-1")?.replayIndex).toBe(0);

    act(() => {
      vi.advanceTimersByTime(10_000);
    });

    const entry = useDashboardStore.getState().arenas.get("arena-1");
    expect(entry?.isReplaying).toBe(false);
    expect(entry?.replayIndex).toBeNull();
    expect(screen.getByText(/state: arena_closed/)).toBeDefined();

    vi.useRealTimers();
  });

  it("sliding back to the last history index returns the arena to live tracking", () => {
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_starting" })) });
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_open" })) });
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_closed" })) });
    });

    act(() => {
      fireEvent.change(screen.getByRole("slider"), { target: { value: "0" } });
    });
    expect(screen.getByText(/state: arena_starting/)).toBeDefined();

    act(() => {
      fireEvent.change(screen.getByRole("slider"), { target: { value: "2" } });
    });

    expect(screen.getByText(/state: arena_closed/)).toBeDefined();
  });

  it("dragging the slider holds a fractional position and scrubs to the nearest index", () => {
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_starting" })) });
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_open" })) });
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_closed" })) });
    });

    const slider = screen.getByRole("slider") as HTMLInputElement;
    expect(slider.step).toBe("any");
    expect(slider.className).toContain("timeline-slider");

    act(() => {
      fireEvent.pointerDown(slider);
      fireEvent.change(slider, { target: { value: "1.4" } });
    });

    expect(slider.value).toBe("1.4");
    expect(screen.getByText(/state: arena_open/)).toBeDefined();
  });

  it("releasing the slider settles it onto the nearest index", () => {
    vi.useFakeTimers();
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_starting" })) });
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_open" })) });
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_closed" })) });
    });

    const slider = screen.getByRole("slider") as HTMLInputElement;

    act(() => {
      fireEvent.pointerDown(slider);
      fireEvent.change(slider, { target: { value: "1.4" } });
    });
    expect(slider.value).toBe("1.4");

    act(() => {
      vi.advanceTimersByTime(SLIDER_SETTLE_MS);
    });
    expect(slider.value).toBe("1.4");

    act(() => {
      fireEvent.pointerUp(slider);
    });
    act(() => {
      vi.advanceTimersByTime(SLIDER_SETTLE_MS);
    });

    expect(slider.value).toBe("1");
    expect(screen.getByText(/state: arena_open/)).toBeDefined();

    vi.useRealTimers();
  });

  it("stop server click asks for confirmation before shutting down", () => {
    render(<App />);
    const source = latestEventSource();
    act(() => {
      source.onopen?.();
    });

    act(() => {
      screen.getByText("Stop server").click();
    });

    expect(screen.getByRole("dialog")).toBeDefined();
    expect(screen.getByText("Stop the dashboard server?")).toBeDefined();
    expect(fetch).not.toHaveBeenCalledWith("/shutdown", { method: "POST" });
  });

  it("confirming the dialog stops the server", async () => {
    render(<App />);
    const source = latestEventSource();
    act(() => {
      source.onopen?.();
    });

    act(() => {
      screen.getByText("Stop server").click();
    });
    await act(async () => {
      within(screen.getByRole("dialog")).getByText("Stop server").click();
    });

    expect(fetch).toHaveBeenCalledWith("/shutdown", { method: "POST" });
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("cancelling the dialog leaves the server running", () => {
    render(<App />);
    const source = latestEventSource();
    act(() => {
      source.onopen?.();
    });

    act(() => {
      screen.getByText("Stop server").click();
    });
    act(() => {
      screen.getByText("Cancel").click();
    });

    expect(screen.queryByRole("dialog")).toBeNull();
    expect(fetch).not.toHaveBeenCalledWith("/shutdown", { method: "POST" });
  });

  it("escape key dismisses the confirmation dialog", () => {
    render(<App />);
    const source = latestEventSource();
    act(() => {
      source.onopen?.();
    });

    act(() => {
      screen.getByText("Stop server").click();
    });
    act(() => {
      fireEvent.keyDown(window, { key: "Escape" });
    });

    expect(screen.queryByRole("dialog")).toBeNull();
    expect(fetch).not.toHaveBeenCalledWith("/shutdown", { method: "POST" });
  });

  it("selecting a subject renders the detail popover near the click position, not centered", () => {
    render(<App />);

    act(() => {
      useDashboardStore.getState().select({ title: "postgres", detail: "id: postgres", position: { x: 50, y: 60 } });
    });

    const heading = screen.getByText("postgres");
    const popover = heading.parentElement as HTMLElement;
    expect(popover.style.left).toBe("50px");
    expect(popover.style.top).toBe("60px");
  });

  it("ignores a malformed message instead of crashing", () => {
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onopen?.();
      source.onmessage?.({ data: "not json" });
    });

    expect(screen.getByText("Connected - no arenas yet")).toBeDefined();
  });

  it("shows a connecting placeholder before the event source opens", () => {
    render(<App />);

    expect(screen.getByText("Connecting...")).toBeDefined();
  });

  it("clicking clear server state posts to the clear endpoint", async () => {
    render(<App />);
    act(() => {
      latestEventSource().onopen?.();
    });

    await act(async () => {
      screen.getByText("Clear server state").click();
    });

    expect(fetch).toHaveBeenCalledWith("/clear", { method: "POST" });
  });

  it("clicking stop server posts to the shutdown endpoint once confirmed", async () => {
    render(<App />);
    act(() => {
      latestEventSource().onopen?.();
    });

    act(() => {
      screen.getByText("Stop server").click();
    });
    await act(async () => {
      within(screen.getByRole("dialog")).getByText("Stop server").click();
    });

    expect(fetch).toHaveBeenCalledWith("/shutdown", { method: "POST" });
  });

  it("clicking clear server state shows feedback once the request resolves", async () => {
    render(<App />);
    act(() => {
      latestEventSource().onopen?.();
    });

    await act(async () => {
      screen.getByText("Clear server state").click();
    });

    expect(screen.getByText("Cleared")).toBeDefined();
  });

  it("confirmed stop server shows feedback once the request resolves", async () => {
    render(<App />);
    act(() => {
      latestEventSource().onopen?.();
    });

    act(() => {
      screen.getByText("Stop server").click();
    });
    await act(async () => {
      within(screen.getByRole("dialog")).getByText("Stop server").click();
    });

    expect(screen.getByText("Server stopped")).toBeDefined();
  });

  it("clear server state and stop server buttons are hidden while not connected", () => {
    render(<App />);

    expect(screen.queryByText("Clear server state")).toBeNull();
    expect(screen.queryByText("Stop server")).toBeNull();
  });

  it("a clear message from the server empties the arena map", () => {
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onmessage?.({ data: JSON.stringify(arenaState()) });
    });
    expect(screen.getAllByText("arena-1").length).toBeGreaterThan(0);

    act(() => {
      source.onmessage?.({ data: JSON.stringify({ type: "clear" }) });
    });

    expect(useDashboardStore.getState().arenas.size).toBe(0);
  });

  it.each([
    ["cancelling the drag", "pointerCancel"],
    ["blurring the slider", "blur"],
  ])("%s settles it onto the nearest index", (_label, endEvent) => {
    vi.useFakeTimers();
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_starting" })) });
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_open" })) });
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_closed" })) });
    });

    const slider = screen.getByRole("slider") as HTMLInputElement;

    act(() => {
      fireEvent.pointerDown(slider);
      fireEvent.change(slider, { target: { value: "1.4" } });
    });
    expect(slider.value).toBe("1.4");

    act(() => {
      if (endEvent === "pointerCancel") fireEvent.pointerCancel(slider);
      else fireEvent.blur(slider);
    });
    act(() => {
      vi.advanceTimersByTime(SLIDER_SETTLE_MS);
    });

    expect(slider.value).toBe("1");

    vi.useRealTimers();
  });

  it("clicking stop replay during a replay halts it at the current index", () => {
    vi.useFakeTimers();
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_starting" })) });
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_open" })) });
      source.onmessage?.({ data: JSON.stringify(arenaState({ state: "arena_closed" })) });
    });

    act(() => {
      screen.getByText("Replay").click();
    });
    expect(useDashboardStore.getState().arenas.get("arena-1")?.isReplaying).toBe(true);

    act(() => {
      screen.getByText("Stop replay").click();
    });

    expect(useDashboardStore.getState().arenas.get("arena-1")?.isReplaying).toBe(false);
    expect(screen.getByText("Replay")).toBeDefined();

    vi.useRealTimers();
  });

  it("clicking inside the detail popover leaves it open", () => {
    render(<App />);

    act(() => {
      useDashboardStore.getState().select({ title: "postgres", detail: "id: postgres", position: { x: 50, y: 60 } });
    });
    const popover = screen.getByText("postgres").parentElement as HTMLElement;

    act(() => {
      popover.click();
    });

    expect(useDashboardStore.getState().selected).not.toBeNull();
  });

  it("clicking the backdrop dismisses the detail popover", () => {
    render(<App />);

    act(() => {
      useDashboardStore.getState().select({ title: "postgres", detail: "id: postgres", position: { x: 50, y: 60 } });
    });
    const backdrop = screen.getByText("postgres").parentElement?.previousElementSibling as HTMLElement;

    act(() => {
      backdrop.click();
    });

    expect(useDashboardStore.getState().selected).toBeNull();
  });

  it("a rejected shutdown request reports the server as stopped", async () => {
    render(<App />);
    act(() => {
      latestEventSource().onopen?.();
    });
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("connection reset")));

    act(() => {
      screen.getByText("Stop server").click();
    });
    await act(async () => {
      within(screen.getByRole("dialog")).getByText("Stop server").click();
    });

    const feedback = screen.getByText("Server stopped");
    expect(feedback.className).toContain("text-teal-300");
    expect(feedback.className).not.toContain("text-rose-400");
  });

  it("a rejected clear request shows a clear failure", async () => {
    render(<App />);
    act(() => {
      latestEventSource().onopen?.();
    });
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("network down")));

    await act(async () => {
      screen.getByText("Clear server state").click();
    });

    expect(screen.getByText("Failed to clear")).toBeDefined();
  });
});

describe("fullScreenProfileComponentId", () => {
  it.each([
    ["?profile=svc", "svc"],
    ["?profile=svc%20one", "svc one"],
    ["?profile=", null],
    ["", null],
    ["?other=svc", null],
  ])("%s resolves to %s", (search, expected) => {
    expect(fullScreenProfileComponentId(search)).toBe(expected);
  });
});

describe("App cpu profile events", () => {
  it("cpu profile event records the profiler state for that component", () => {
    render(<App />);

    act(() => {
      latestEventSource().onmessage?.({
        data: JSON.stringify({
          type: "cpu_profile",
          component: "svc",
          state: "stopped",
          report: true,
        }),
      });
    });

    expect(useDashboardStore.getState().cpuProfiles.get("svc")).toEqual({
      state: "stopped",
      reportReady: true,
    });
  });

  it("appCpuProfileEventWithoutAReportRecordsTheStateOnly", () => {
    render(<App />);

    act(() => {
      latestEventSource().onmessage?.({
        data: JSON.stringify({ type: "cpu_profile", component: "svc", state: "stopped" }),
      });
    });

    expect(useDashboardStore.getState().cpuProfiles.get("svc")).toEqual({
      state: "stopped",
      reportReady: false,
    });
  });

  it("cpu profile event is not treated as an arena snapshot", () => {
    render(<App />);

    act(() => {
      latestEventSource().onmessage?.({
        data: JSON.stringify({ type: "cpu_profile", component: "svc", state: "started" }),
      });
    });

    expect(useDashboardStore.getState().arenas.size).toBe(0);
  });

  it("clear event discards recorded profiler state", () => {
    render(<App />);

    act(() => {
      latestEventSource().onmessage?.({
        data: JSON.stringify({ type: "cpu_profile", component: "svc", state: "stopped" }),
      });
      latestEventSource().onmessage?.({ data: JSON.stringify({ type: "clear" }) });
    });

    expect(useDashboardStore.getState().cpuProfiles.size).toBe(0);
  });
});

describe("App full screen profile", () => {
  it("profile query parameter renders only the profile view", () => {
    window.history.replaceState(null, "", "/?profile=svc");
    vi.stubGlobal("fetch", vi.fn().mockReturnValue(new Promise(() => {})));

    render(<App />);

    expect(screen.getByText("cpu profile · svc")).toBeDefined();
    expect(screen.queryByText("Arena Topology Dashboard")).toBeNull();
  });

  it("appDisconnectedWithArenasInMemoryShowsNoNotice", () => {
    vi.useFakeTimers();
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onopen?.();
      source.onmessage?.({ data: JSON.stringify(arenaState()) });
    });
    act(() => {
      vi.advanceTimersByTime(1600);
    });

    expect(screen.getByText("disconnected")).toBeDefined();
    expect(screen.queryByTestId("disconnected-notice")).toBeNull();
    vi.useRealTimers();
  });
});
