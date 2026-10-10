import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

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
});

afterEach(() => {
  FakeEventSource.instances = [];
});

describe("App", () => {
  it("renders the dashboard heading and starts in connecting status", () => {
    render(<App />);

    expect(screen.getByText("Arena Dashboard")).toBeDefined();
    expect(screen.getByText("connecting")).toBeDefined();
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

    expect(screen.getByText("arena-1")).toBeDefined();
  });

  it("ignores a malformed message instead of crashing", () => {
    render(<App />);
    const source = latestEventSource();

    act(() => {
      source.onmessage?.({ data: "not json" });
    });

    expect(screen.getByText("Connected - no arenas yet")).toBeDefined();
  });
});
