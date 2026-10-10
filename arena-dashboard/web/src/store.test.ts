import { beforeEach, describe, expect, it } from "vitest";

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

describe("useDashboardStore ingest", () => {
  it("first snapshot for a new arena logs new transitions", () => {
    useDashboardStore.getState().ingest(
      arenaState({ dependencies: [{ id: "postgres", state: "started", faults: [], children: [] }] })
    );

    const entry = useDashboardStore.getState().arenas.get("arena-1");
    expect(entry).toBeDefined();
    expect(entry?.log.some((e) => e.text.includes("(new) -> started"))).toBe(true);
    expect(entry?.log.some((e) => e.text.includes("arena arena-1: (new) -> arena_open"))).toBe(true);
  });

  it("unchanged snapshot logs no change", () => {
    const snapshot = arenaState();
    useDashboardStore.getState().ingest(snapshot);
    const logLengthAfterFirst = useDashboardStore.getState().arenas.get("arena-1")?.log.length ?? 0;

    useDashboardStore.getState().ingest(snapshot);

    const entry = useDashboardStore.getState().arenas.get("arena-1");
    expect(entry?.log.length).toBe(logLengthAfterFirst + 1);
    expect(entry?.log.at(-1)?.text).toContain("no change");
  });

  it("subject state change appends a transition entry and keeps prior log", () => {
    useDashboardStore.getState().ingest(
      arenaState({ dependencies: [{ id: "postgres", state: "starting", faults: [], children: [] }] })
    );
    const firstLogLength = useDashboardStore.getState().arenas.get("arena-1")?.log.length ?? 0;

    useDashboardStore.getState().ingest(
      arenaState({ dependencies: [{ id: "postgres", state: "started", faults: [], children: [] }] })
    );

    const entry = useDashboardStore.getState().arenas.get("arena-1");
    expect(entry?.log.length).toBeGreaterThan(firstLogLength);
    expect(entry?.log.at(-1)?.text).toContain("starting -> started");
  });

  it("subject transitioning to faulted marks the log entry as a fault", () => {
    useDashboardStore.getState().ingest(
      arenaState({ components: [{ id: "api", state: "started", faults: [], children: [] }] })
    );

    useDashboardStore.getState().ingest(
      arenaState({ components: [{ id: "api", state: "faulted", faults: [], children: [] }] })
    );

    const entry = useDashboardStore.getState().arenas.get("arena-1");
    const faultEntry = entry?.log.find((e) => e.text.includes("api") && e.text.includes("faulted"));
    expect(faultEntry?.isFault).toBe(true);
  });

  it("two different arena ids are tracked independently", () => {
    useDashboardStore.getState().ingest(arenaState({ id: "arena-1" }));
    useDashboardStore.getState().ingest(arenaState({ id: "arena-2" }));

    expect(useDashboardStore.getState().arenas.size).toBe(2);
    expect(useDashboardStore.getState().arenas.has("arena-1")).toBe(true);
    expect(useDashboardStore.getState().arenas.has("arena-2")).toBe(true);
  });
});

describe("useDashboardStore selection", () => {
  it("select sets the selected detail", () => {
    useDashboardStore.getState().select({ title: "postgres", detail: "id: postgres" });

    expect(useDashboardStore.getState().selected).toEqual({ title: "postgres", detail: "id: postgres" });
  });

  it("clearSelected resets selection to null", () => {
    useDashboardStore.getState().select({ title: "postgres", detail: "id: postgres" });

    useDashboardStore.getState().clearSelected();

    expect(useDashboardStore.getState().selected).toBeNull();
  });
});
