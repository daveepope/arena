import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";

import { ArenaGraph } from "./ArenaGraph";
import { useDashboardStore } from "./store";
import type { ArenaState } from "./types";

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
  useDashboardStore.setState({ arenas: new Map(), selected: null });
});

describe("ArenaGraph", () => {
  it("shows a waiting placeholder before any snapshot arrives", () => {
    render(<ArenaGraph arenaId="arena-1" />);

    expect(screen.getByText("waiting for events...")).toBeDefined();
  });

  it("renders a node for each visible subject once a snapshot is ingested", () => {
    render(<ArenaGraph arenaId="arena-1" />);

    act(() => {
      useDashboardStore.getState().ingest(
        arenaState({
          dependencies: [{ id: "postgres", state: "started", faults: [], children: [] }],
          components: [{ id: "api", state: "started", faults: [], children: [] }],
        })
      );
    });

    expect(screen.getByText("postgres")).toBeDefined();
    expect(screen.getByText("api")).toBeDefined();
  });

  it("clicking a node sets the store selection to that subject's detail", () => {
    render(<ArenaGraph arenaId="arena-1" />);

    act(() => {
      useDashboardStore.getState().ingest(
        arenaState({ dependencies: [{ id: "postgres", state: "started", faults: [], children: [] }] })
      );
    });

    fireEvent.click(screen.getByText("postgres"));

    expect(useDashboardStore.getState().selected?.title).toBe("postgres");
  });

  it("clicking the arena panel button sets the store selection to the arena's own detail", () => {
    render(<ArenaGraph arenaId="arena-1" />);

    act(() => {
      useDashboardStore.getState().ingest(arenaState());
    });

    fireEvent.click(screen.getByText(/arena-1.*state: arena_open/));

    expect(useDashboardStore.getState().selected?.title).toContain("arena-1");
  });

  it("not_started subjects are not visible as rendered node text", () => {
    render(<ArenaGraph arenaId="arena-1" />);

    act(() => {
      useDashboardStore.getState().ingest(
        arenaState({ dependencies: [{ id: "oracle", state: "not_started", faults: [], children: [] }] })
      );
    });

    const node = screen.getByText("oracle").closest(".pointer-events-none");
    expect(node).not.toBeNull();
  });
});
