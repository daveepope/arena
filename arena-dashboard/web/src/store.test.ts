import "./testSetup";

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useDashboardStore } from "./store";
import type { ArenaState, ScopedPlaybookEntry } from "./types";

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
  useDashboardStore.setState({
    arenas: new Map(),
    selected: null,
    cpuProfiles: new Map(),
    profileView: null,
  });
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
    expect(history?.[0].snapshot?.state).toBe("arena_starting");
    expect(history?.[1].snapshot?.state).toBe("arena_open");
    expect(typeof history?.[0].at).toBe("number");
  });

  it("ingesting the same snapshot again (e.g. replayed on an SSE reconnect) does not duplicate history", () => {
    useDashboardStore.getState().ingest(arenaState({ state: "arena_starting" }));
    useDashboardStore.getState().ingest(arenaState({ state: "arena_open" }));
    useDashboardStore.getState().ingest(arenaState({ state: "arena_open" }));
    useDashboardStore.getState().ingest(arenaState({ state: "arena_open" }));

    const history = useDashboardStore.getState().arenas.get("arena-1")?.history;
    expect(history?.length).toBe(2);
    expect(history?.map((h) => h.snapshot?.state)).toEqual(["arena_starting", "arena_open"]);
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

describe("useDashboardStore ingestHistory", () => {
  it("server history rebuilds the full timeline and event log", () => {
    useDashboardStore.getState().ingestHistory([
      arenaState({ state: "arena_starting", at: "2026-01-01T00:00:00.000Z" }),
      arenaState({
        state: "arena_open",
        at: "2026-01-01T00:00:01.000Z",
        dependencies: [{ id: "postgres", state: "started", faults: [], children: [] }],
      }),
      arenaState({ state: "arena_closed", at: "2026-01-01T00:00:02.000Z" }),
    ]);

    const entry = useDashboardStore.getState().arenas.get("arena-1");
    expect(entry?.history.length).toBe(3);
    expect(entry?.snapshot.state).toBe("arena_closed");
    expect(entry?.log.some((e) => e.text.includes("(new) -> arena_starting"))).toBe(true);
    expect(entry?.log.some((e) => e.text.includes("arena_open -> arena_closed"))).toBe(true);
  });

  it("history arriving after a live snapshot keeps the newer live entry", () => {
    useDashboardStore.getState().ingest(arenaState({ state: "arena_closed", at: "2026-01-01T00:00:09.000Z" }));
    useDashboardStore.getState().ingestHistory([
      arenaState({ state: "arena_starting", at: "2026-01-01T00:00:00.000Z" }),
      arenaState({ state: "arena_open", at: "2026-01-01T00:00:01.000Z" }),
    ]);

    const entry = useDashboardStore.getState().arenas.get("arena-1");
    expect(entry?.history.length).toBe(3);
    expect(entry?.history[2].snapshot?.state).toBe("arena_closed");
  });

  it("history no longer than what is already recorded is ignored", () => {
    useDashboardStore.getState().ingestHistory([
      arenaState({ state: "arena_starting", at: "2026-01-01T00:00:00.000Z" }),
      arenaState({ state: "arena_closed", at: "2026-01-01T00:00:02.000Z" }),
    ]);
    useDashboardStore.getState().ingestHistory([
      arenaState({ state: "arena_starting", at: "2026-01-01T00:00:00.000Z" }),
    ]);

    expect(useDashboardStore.getState().arenas.get("arena-1")?.history.length).toBe(2);
  });

  it("scrubbing works on a timeline restored from server history", () => {
    useDashboardStore.getState().ingestHistory([
      arenaState({ state: "arena_starting", at: "2026-01-01T00:00:00.000Z" }),
      arenaState({ state: "arena_open", at: "2026-01-01T00:00:01.000Z" }),
      arenaState({ state: "arena_closed", at: "2026-01-01T00:00:02.000Z" }),
    ]);

    useDashboardStore.getState().scrubArena("arena-1", 0);

    const entry = useDashboardStore.getState().arenas.get("arena-1");
    expect(entry?.replayIndex).toBe(0);
    expect(entry?.snapshot.state).toBe("arena_starting");
  });

  it("empty history is ignored", () => {
    useDashboardStore.getState().ingestHistory([]);

    expect(useDashboardStore.getState().arenas.size).toBe(0);
  });

  it("longer history arriving while scrubbed extends the timeline without moving the display", () => {
    useDashboardStore.getState().ingestHistory([
      arenaState({ state: "arena_starting", at: "2026-01-01T00:00:00.000Z" }),
      arenaState({ state: "arena_open", at: "2026-01-01T00:00:01.000Z" }),
    ]);
    useDashboardStore.getState().scrubArena("arena-1", 0);

    useDashboardStore.getState().ingestHistory([
      arenaState({ state: "arena_starting", at: "2026-01-01T00:00:00.000Z" }),
      arenaState({ state: "arena_open", at: "2026-01-01T00:00:01.000Z" }),
      arenaState({ state: "arena_closed", at: "2026-01-01T00:00:02.000Z" }),
    ]);

    const entry = useDashboardStore.getState().arenas.get("arena-1");
    expect(entry?.history.length).toBe(3);
    expect(entry?.replayIndex).toBe(0);
    expect(entry?.snapshot.state).toBe("arena_starting");
  });
});

describe("useDashboardStore cpu profiles", () => {
  it("ingested profile state is keyed by component id", () => {
    useDashboardStore.getState().ingestCpuProfile("svc", "started", false);
    useDashboardStore.getState().ingestCpuProfile("worker", "stopped", true);

    expect(useDashboardStore.getState().cpuProfiles.get("svc")).toEqual({ state: "started", reportReady: false });
    expect(useDashboardStore.getState().cpuProfiles.get("worker")).toEqual({ state: "stopped", reportReady: true });
  });

  it("later state for the same component replaces the earlier one", () => {
    useDashboardStore.getState().ingestCpuProfile("svc", "started", false);
    useDashboardStore.getState().ingestCpuProfile("svc", "stopped", true);

    expect(useDashboardStore.getState().cpuProfiles.get("svc")).toEqual({ state: "stopped", reportReady: true });
    expect(useDashboardStore.getState().cpuProfiles.size).toBe(1);
  });

  it("clearing arenas discards the profiles and the open view", () => {
    useDashboardStore.getState().ingestCpuProfile("svc", "stopped", true);
    useDashboardStore.getState().openProfileView("svc", { x: 1, y: 2 });

    useDashboardStore.getState().clearArenas();

    expect(useDashboardStore.getState().cpuProfiles.size).toBe(0);
    expect(useDashboardStore.getState().profileView).toBeNull();
  });
});

describe("useDashboardStore profile view", () => {
  it("opening a view records the component and position unpinned", () => {
    useDashboardStore.getState().openProfileView("svc", { x: 4, y: 8 });

    expect(useDashboardStore.getState().profileView).toEqual({
      componentId: "svc",
      position: { x: 4, y: 8 },
      isPinned: false,
    });
  });

  it("closing a view clears it", () => {
    useDashboardStore.getState().openProfileView("svc", { x: 4, y: 8 });

    useDashboardStore.getState().closeProfileView();

    expect(useDashboardStore.getState().profileView).toBeNull();
  });

  it("toggling the pin flips it both ways", () => {
    useDashboardStore.getState().openProfileView("svc", { x: 4, y: 8 });

    useDashboardStore.getState().toggleProfileViewPin();
    expect(useDashboardStore.getState().profileView?.isPinned).toBe(true);

    useDashboardStore.getState().toggleProfileViewPin();
    expect(useDashboardStore.getState().profileView?.isPinned).toBe(false);
  });

  it("toggling the pin with no view open does nothing", () => {
    useDashboardStore.getState().toggleProfileViewPin();

    expect(useDashboardStore.getState().profileView).toBeNull();
  });
});

describe("useDashboardStore arena restarts", () => {
  it("sameArenaStartingAgainReplacesTheFinishedRun", () => {
    const store = useDashboardStore.getState();
    store.ingest(arenaState({ id: "arena-1", state: "arena_starting" }));
    store.ingest(arenaState({ id: "arena-1", state: "arena_open" }));
    store.ingest(arenaState({ id: "arena-1", state: "arena_closed" }));
    const afterFirstRun = useDashboardStore.getState().arenas.get("arena-1");
    expect(afterFirstRun?.history).toHaveLength(3);

    useDashboardStore.getState().ingest(arenaState({ id: "arena-1", state: "arena_starting" }));

    const entry = useDashboardStore.getState().arenas.get("arena-1");
    expect(entry?.history).toHaveLength(1);
    expect(entry?.history[0].snapshot?.state).toBe("arena_starting");
    expect(entry?.snapshot.state).toBe("arena_starting");
  });

  it("arenaStillRunningKeepsItsHistory", () => {
    const store = useDashboardStore.getState();
    store.ingest(arenaState({ id: "arena-2", state: "arena_starting" }));
    store.ingest(arenaState({ id: "arena-2", state: "arena_open" }));

    useDashboardStore.getState().ingest(arenaState({ id: "arena-2", state: "arena_starting" }));

    expect(useDashboardStore.getState().arenas.get("arena-2")?.history).toHaveLength(3);
  });
});

describe("useDashboardStore replay", () => {
  it("replayWalksEveryRecordedStateAndReturnsToLive", () => {
    vi.useFakeTimers();
    const store = useDashboardStore.getState();
    store.ingest(arenaState({ id: "arena-replay", state: "arena_starting" }));
    store.ingest(arenaState({ id: "arena-replay", state: "arena_open" }));
    store.ingest(arenaState({ id: "arena-replay", state: "arena_closed" }));

    useDashboardStore.getState().replayArena("arena-replay");
    expect(useDashboardStore.getState().arenas.get("arena-replay")?.isReplaying).toBe(true);

    vi.advanceTimersByTime(5_000);

    const entry = useDashboardStore.getState().arenas.get("arena-replay");
    expect(entry?.isReplaying).toBe(false);
    expect(entry?.replayIndex).toBeNull();
    expect(entry?.snapshot.state).toBe("arena_closed");
    vi.useRealTimers();
  });
});

function playbookEntry(
  playbookId: string,
  state: string,
  at = 1,
  kind = "scoped",
): ScopedPlaybookEntry {
  return { at, playbookId, state, kind, faults: [] };
}

describe("useDashboardStore playbooks", () => {
  function openArena(arenaId: string, at = "2026-10-18T00:00:00.000Z") {
    useDashboardStore
      .getState()
      .ingest(arenaState({ id: arenaId, state: "arena_open", at }));
  }

  function playbooksOf(arenaId: string) {
    return useDashboardStore.getState().arenas.get(arenaId)?.playbooks;
  }

  it("reportedPlaybooksAreHeldAgainstTheirOwnArena", () => {
    openArena("arena-a");
    openArena("arena-b");

    useDashboardStore.getState().ingestScopedPlaybook("arena-a", playbookEntry("reset", "started"));
    useDashboardStore.getState().ingestScopedPlaybook("arena-b", playbookEntry("reset", "stopped"));

    expect(playbooksOf("arena-a")?.get("reset")?.state).toBe("started");
    expect(playbooksOf("arena-b")?.get("reset")?.state).toBe("stopped");
  });

  it("reportForAnArenaThatIsNotOnScreenIsDropped", () => {
    useDashboardStore.getState().ingestScopedPlaybook("arena-unknown", playbookEntry("reset", "started"));

    expect(useDashboardStore.getState().arenas.get("arena-unknown")).toBeUndefined();
  });

  it("restartingAnArenaForgetsItsPreviousPlaybooks", () => {
    const store = useDashboardStore.getState();
    store.ingest(arenaState({ id: "arena-restart", state: "arena_starting" }));
    store.ingest(arenaState({ id: "arena-restart", state: "arena_closed" }));
    store.ingestScopedPlaybook("arena-restart", playbookEntry("reset", "faulted"));

    useDashboardStore.getState().ingest(arenaState({ id: "arena-restart", state: "arena_starting" }));

    expect(playbooksOf("arena-restart")?.size).toBe(0);
  });

  it("clearingArenasForgetsEveryPlaybook", () => {
    openArena("arena-a");
    useDashboardStore.getState().ingestScopedPlaybook("arena-a", playbookEntry("reset", "started"));

    useDashboardStore.getState().clearArenas();

    expect(useDashboardStore.getState().arenas.size).toBe(0);
  });

  it("everyReportedStateJoinsTheSameTimelineAsTheSnapshots", () => {
    openArena("arena-a", "2026-10-18T00:00:00.000Z");
    const store = useDashboardStore.getState();
    store.ingestScopedPlaybook("arena-a", playbookEntry("reset", "starting", 10));
    store.ingestScopedPlaybook("arena-a", playbookEntry("reset", "started", 20));
    store.ingestScopedPlaybook("arena-a", playbookEntry("reset", "stopped", 30));

    const history = useDashboardStore.getState().arenas.get("arena-a")?.history ?? [];
    expect(history.map((entry) => entry.playbook?.state ?? entry.snapshot?.state)).toEqual([
      "arena_open",
      "starting",
      "started",
      "stopped",
    ]);
  });

  it("sameReportArrivingTwiceIsRecordedOnce", () => {
    openArena("arena-a");
    const store = useDashboardStore.getState();
    store.ingestScopedPlaybook("arena-a", playbookEntry("reset", "started", 20));

    store.ingestScopedPlaybook("arena-a", playbookEntry("reset", "started", 20));

    expect(useDashboardStore.getState().arenas.get("arena-a")?.history).toHaveLength(2);
  });

  it("loadedHistoryRebuildsBothTheSnapshotAndThePlaybooks", () => {
    useDashboardStore.getState().ingestHistory(
      [
        arenaState({ id: "arena-a", state: "arena_starting", at: "2026-10-18T00:00:01.000Z" }),
        arenaState({ id: "arena-a", state: "arena_open", at: "2026-10-18T00:00:03.000Z" }),
      ],
      [
        playbookEntry("reset", "started", Date.parse("2026-10-18T00:00:02.000Z")),
        playbookEntry("reset", "starting", Date.parse("2026-10-18T00:00:01.500Z")),
      ],
    );

    const entry = useDashboardStore.getState().arenas.get("arena-a");
    expect(entry?.snapshot.state).toBe("arena_open");
    expect(entry?.playbooks.get("reset")?.state).toBe("started");
    expect(entry?.history).toHaveLength(4);
  });

  it("scrubbingBackBeforeAPlaybookReportDropsThatRow", () => {
    useDashboardStore.getState().ingestHistory(
      [
        arenaState({ id: "arena-a", state: "arena_starting", at: "2026-10-18T00:00:01.000Z" }),
        arenaState({ id: "arena-a", state: "arena_open", at: "2026-10-18T00:00:04.000Z" }),
      ],
      [
        playbookEntry("reset", "starting", Date.parse("2026-10-18T00:00:02.000Z")),
        playbookEntry("seed", "started", Date.parse("2026-10-18T00:00:03.000Z")),
      ],
    );

    useDashboardStore.getState().scrubArena("arena-a", 1);

    const entry = useDashboardStore.getState().arenas.get("arena-a");
    expect(entry?.playbooks.get("reset")?.state).toBe("starting");
    expect(entry?.playbooks.get("seed")).toBeUndefined();
    expect(entry?.snapshot.state).toBe("arena_starting");
  });

  it("scrubbingBackToTheStartShowsNoPlaybooks", () => {
    useDashboardStore.getState().ingestHistory(
      [
        arenaState({ id: "arena-a", state: "arena_starting", at: "2026-10-18T00:00:01.000Z" }),
        arenaState({ id: "arena-a", state: "arena_open", at: "2026-10-18T00:00:03.000Z" }),
      ],
      [playbookEntry("reset", "starting", Date.parse("2026-10-18T00:00:02.000Z"))],
    );

    useDashboardStore.getState().scrubArena("arena-a", 0);

    expect(useDashboardStore.getState().arenas.get("arena-a")?.playbooks.size).toBe(0);
  });

  it("aReportArrivingDuringAReplayIsRecordedWithoutDisturbingTheReplayedRows", () => {
    useDashboardStore.getState().ingestHistory(
      [
        arenaState({ id: "arena-a", state: "arena_starting", at: "2026-10-18T00:00:01.000Z" }),
        arenaState({ id: "arena-a", state: "arena_open", at: "2026-10-18T00:00:03.000Z" }),
      ],
      [playbookEntry("reset", "starting", Date.parse("2026-10-18T00:00:02.000Z"))],
    );
    useDashboardStore.getState().scrubArena("arena-a", 0);

    useDashboardStore
      .getState()
      .ingestScopedPlaybook("arena-a", playbookEntry("seed", "started", Date.parse("2026-10-18T00:00:04.000Z")));

    const entry = useDashboardStore.getState().arenas.get("arena-a");
    expect(entry?.playbooks.size).toBe(0);
    expect(entry?.history).toHaveLength(4);
  });
});


