import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "./App";
import { useDashboardStore } from "./store";
import type { ArenaState } from "./types";

class FakeEventSource {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSED = 2;
  static instances: FakeEventSource[] = [];

  readyState = FakeEventSource.CONNECTING;
  onopen: (() => void) | null = null;
  onerror: (() => void) | null = null;
  onmessage: ((evt: { data: string }) => void) | null = null;

  constructor(public url: string) {
    FakeEventSource.instances.push(this);
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
  useDashboardStore.setState({ arenas: new Map(), selected: null });
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

  it("shows an error message when the event source errors while open", () => {
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.readyState = FakeEventSource.CLOSED;
      source.onerror?.();
    });

    expect(screen.getByText("error")).toBeDefined();
    expect(screen.getByText(/Lost connection/)).toBeDefined();
  });

  it("renders an arena panel once a state document is ingested", () => {
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onmessage?.({ data: JSON.stringify(arenaState()) });
    });

    expect(screen.getAllByText("arena-1").length).toBeGreaterThan(0);
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
      vi.runAllTimers();
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

    await act(async () => {
      screen.getByText("Clear server state").click();
    });

    expect(fetch).toHaveBeenCalledWith("/clear", { method: "POST" });
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
});
