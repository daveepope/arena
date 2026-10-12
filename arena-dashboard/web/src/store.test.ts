import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

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
    useDashboardStore.getState().select({ title: "postgres", detail: "id: postgres", position: { x: 10, y: 20 } });

    expect(useDashboardStore.getState().selected).toEqual({
      title: "postgres",
      detail: "id: postgres",
      position: { x: 10, y: 20 },
    });
  });

  it("clearSelected resets selection to null", () => {
    useDashboardStore.getState().select({ title: "postgres", detail: "id: postgres", position: { x: 10, y: 20 } });

    useDashboardStore.getState().clearSelected();

    expect(useDashboardStore.getState().selected).toBeNull();
  });

  it("clearArenas empties the arena map and the current selection", () => {
    useDashboardStore.getState().ingest(arenaState());
    useDashboardStore.getState().select({ title: "postgres", detail: "id: postgres", position: { x: 10, y: 20 } });

    useDashboardStore.getState().clearArenas();

    expect(useDashboardStore.getState().arenas.size).toBe(0);
    expect(useDashboardStore.getState().selected).toBeNull();
  });
});

describe("useDashboardStore replay", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("ingest records every snapshot into history with a timestamp", () => {
    useDashboardStore.getState().ingest(arenaState({ state: "arena_starting" }));
    useDashboardStore.getState().ingest(arenaState({ state: "arena_open" }));

    const history = useDashboardStore.getState().arenas.get("arena-1")?.history;
    expect(history?.length).toBe(2);
    expect(history?.[0].snapshot.state).toBe("arena_starting");
    expect(history?.[1].snapshot.state).toBe("arena_open");
    expect(typeof history?.[0].at).toBe("number");
  });

  it("replayArena replays recorded snapshots back through the same log, ending not replaying", () => {
    useDashboardStore.getState().ingest(arenaState({ state: "arena_starting" }));
    useDashboardStore.getState().ingest(arenaState({ state: "arena_open" }));
    const originalLogLength = useDashboardStore.getState().arenas.get("arena-1")?.log.length ?? 0;

    useDashboardStore.getState().replayArena("arena-1");
    expect(useDashboardStore.getState().arenas.get("arena-1")?.isReplaying).toBe(true);

    vi.runAllTimers();

    const entry = useDashboardStore.getState().arenas.get("arena-1");
    expect(entry?.isReplaying).toBe(false);
    expect(entry?.log.length).toBe(originalLogLength);
    expect(entry?.snapshot.state).toBe("arena_open");
  });

  it("stopReplay halts an in-progress replay", () => {
    useDashboardStore.getState().ingest(arenaState({ state: "arena_starting" }));
    useDashboardStore.getState().ingest(arenaState({ state: "arena_open" }));

    useDashboardStore.getState().replayArena("arena-1");
    useDashboardStore.getState().stopReplay("arena-1");

    expect(useDashboardStore.getState().arenas.get("arena-1")?.isReplaying).toBe(false);
  });

  it("ingest during replay still records history but leaves the replay display untouched", () => {
    useDashboardStore.getState().ingest(arenaState({ state: "arena_starting" }));
    useDashboardStore.getState().ingest(arenaState({ state: "arena_open" }));
    useDashboardStore.getState().replayArena("arena-1");

    useDashboardStore.getState().ingest(arenaState({ state: "arena_closed" }));

    const entry = useDashboardStore.getState().arenas.get("arena-1");
    expect(entry?.history.length).toBe(3);
    expect(entry?.isReplaying).toBe(true);
  });

  it("scrubArena jumps the display to an earlier point in history", () => {
    useDashboardStore.getState().ingest(arenaState({ state: "arena_starting" }));
    useDashboardStore.getState().ingest(arenaState({ state: "arena_open" }));
    useDashboardStore.getState().ingest(arenaState({ state: "arena_closed" }));

    useDashboardStore.getState().scrubArena("arena-1", 0);

    const entry = useDashboardStore.getState().arenas.get("arena-1");
    expect(entry?.snapshot.state).toBe("arena_starting");
    expect(entry?.replayIndex).toBe(0);
    expect(entry?.isReplaying).toBe(false);
  });

  it("scrubArena at the last index returns to live (replayIndex null)", () => {
    useDashboardStore.getState().ingest(arenaState({ state: "arena_starting" }));
    useDashboardStore.getState().ingest(arenaState({ state: "arena_open" }));
    useDashboardStore.getState().scrubArena("arena-1", 0);

    useDashboardStore.getState().scrubArena("arena-1", 1);

    const entry = useDashboardStore.getState().arenas.get("arena-1");
    expect(entry?.snapshot.state).toBe("arena_open");
    expect(entry?.replayIndex).toBeNull();
  });

  it("scrubArena clamps an out-of-range index to the available history", () => {
    useDashboardStore.getState().ingest(arenaState({ state: "arena_starting" }));
    useDashboardStore.getState().ingest(arenaState({ state: "arena_open" }));

    useDashboardStore.getState().scrubArena("arena-1", 99);

    const entry = useDashboardStore.getState().arenas.get("arena-1");
    expect(entry?.snapshot.state).toBe("arena_open");
    expect(entry?.replayIndex).toBeNull();
  });

  it("scrubArena stops an in-progress auto-replay", () => {
    useDashboardStore.getState().ingest(arenaState({ state: "arena_starting" }));
    useDashboardStore.getState().ingest(arenaState({ state: "arena_open" }));
    useDashboardStore.getState().replayArena("arena-1");

    useDashboardStore.getState().scrubArena("arena-1", 0);

    const entry = useDashboardStore.getState().arenas.get("arena-1");
    expect(entry?.isReplaying).toBe(false);
    expect(entry?.snapshot.state).toBe("arena_starting");
  });

  it("ingest while scrubbed to a past point does not move the display", () => {
    useDashboardStore.getState().ingest(arenaState({ state: "arena_starting" }));
    useDashboardStore.getState().ingest(arenaState({ state: "arena_open" }));
    useDashboardStore.getState().scrubArena("arena-1", 0);

    useDashboardStore.getState().ingest(arenaState({ state: "arena_closed" }));

    const entry = useDashboardStore.getState().arenas.get("arena-1");
    expect(entry?.snapshot.state).toBe("arena_starting");
    expect(entry?.history.length).toBe(3);
  });
});
