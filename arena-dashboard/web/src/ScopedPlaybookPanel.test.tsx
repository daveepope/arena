import "./testSetup";

import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { ScopedPlaybookPanel } from "./ScopedPlaybookPanel";
import { useDashboardStore } from "./store";
import type { ArenaState, Fault } from "./types";

const ARENA = "arena-orders";

function arenaSnapshot(state: string, at = "2026-10-18T00:00:00.000Z"): ArenaState {
  return {
    id: ARENA,
    state,
    at,
    dependencies: [],
    components: [],
    faults: [],
  };
}

function fault(message: string): Fault {
  return {
    id: "reset-orders",
    subject: "playbook",
    message,
    at: "2026-10-18T00:00:00.000Z",
    faults: [],
  };
}

function report(
  arenaId: string,
  playbookId: string,
  state: string,
  faults: Fault[] = [],
  kind = "scoped",
  at = 1,
  playbookType = "managed",
) {
  useDashboardStore
    .getState()
    .ingestScopedPlaybook(arenaId, { at, playbookId, state, kind, playbookType, faults });
}

beforeEach(() => {
  useDashboardStore.getState().ingest(arenaSnapshot("arena_open"));
});

afterEach(() => {
  useDashboardStore.getState().clearArenas();
});

describe("ScopedPlaybookPanel", () => {
  it("arenaWithNoPlaybooksSaysNoneAreRegistered", () => {
    render(<ScopedPlaybookPanel arenaId={ARENA} />);

    expect(screen.getByTestId(`no-playbooks-${ARENA}`).textContent).toBe(
      "no playbooks registered",
    );
  });

  it("reportedPlaybooksRenderARowEachWithItsState", () => {
    report(ARENA, "reset-orders", "started");
    report(ARENA, "apply-schema", "stopped");

    render(<ScopedPlaybookPanel arenaId={ARENA} />);

    expect(screen.getByTestId("scoped-playbook-reset-orders").textContent).toContain("started");
    expect(screen.getByTestId("scoped-playbook-apply-schema").textContent).toContain("stopped");
  });

  it("playbooksAreListedInNameOrder", () => {
    report(ARENA, "reset-orders", "started");
    report(ARENA, "apply-schema", "started");

    render(<ScopedPlaybookPanel arenaId={ARENA} />);

    const rows = screen.getAllByRole("button").map((row) => row.getAttribute("data-testid"));
    expect(rows).toEqual(["scoped-playbook-apply-schema", "scoped-playbook-reset-orders"]);
  });

  it("laterStateForTheSamePlaybookReplacesTheEarlierOne", () => {
    report(ARENA, "reset-orders", "started", [], "scoped", 1);
    report(ARENA, "reset-orders", "stopped", [], "scoped", 2);

    render(<ScopedPlaybookPanel arenaId={ARENA} />);

    expect(screen.getAllByRole("button")).toHaveLength(1);
    expect(screen.getByTestId("scoped-playbook-reset-orders").textContent).toContain("stopped");
  });

  it("faultedPlaybookIsMarked", () => {
    report(ARENA, "reset-orders", "faulted", [fault("seed data rejected")]);

    render(<ScopedPlaybookPanel arenaId={ARENA} />);

    expect(screen.getByTestId("scoped-playbook-reset-orders").className).toContain("rose");
  });

  it("faultedStateWithNoFaultDetailIsStillMarked", () => {
    report(ARENA, "reset-orders", "faulted");

    render(<ScopedPlaybookPanel arenaId={ARENA} />);

    expect(screen.getByTestId("scoped-playbook-reset-orders").className).toContain("rose");
  });

  it("rowClickOpensTheDetailDialogWithItsFaults", () => {
    report(ARENA, "reset-orders", "faulted", [fault("seed data rejected")]);
    render(<ScopedPlaybookPanel arenaId={ARENA} />);

    fireEvent.click(screen.getByTestId("scoped-playbook-reset-orders"));

    const selected = useDashboardStore.getState().selected;
    expect(selected?.title).toBe("reset-orders");
    expect(selected?.detail).toContain("kind: playbook");
    expect(selected?.detail).toContain("seed data rejected");
  });

  it("rowNamesItsPlaybookType", () => {
    report(ARENA, "seed-reference-data", "started", [], "scoped", 1, "unmanaged");

    render(<ScopedPlaybookPanel arenaId={ARENA} />);

    expect(screen.getByTestId("scoped-playbook-seed-reference-data").textContent).toContain(
      "unmanaged",
    );
  });

  it("startupPlaybookRowNamesItsKind", () => {
    report(ARENA, "apply-schema", "started", [], "startup");

    render(<ScopedPlaybookPanel arenaId={ARENA} />);

    expect(screen.getByTestId("scoped-playbook-apply-schema").textContent).toContain("startup");
  });

  it("anotherArenasPlaybooksAreNotShown", () => {
    useDashboardStore.getState().ingest({ ...arenaSnapshot("arena_open"), id: "arena-catalog" });
    report("arena-catalog", "reset-orders", "started");

    render(<ScopedPlaybookPanel arenaId={ARENA} />);

    expect(screen.queryByTestId("scoped-playbook-reset-orders")).toBeNull();
    expect(screen.getByTestId(`no-playbooks-${ARENA}`)).not.toBeNull();
  });

  it("replayedArenaShowsTheRowsAsOfTheReplayedMoment", () => {
    const store = useDashboardStore.getState();
    store.ingest(arenaSnapshot("arena_starting", "2026-10-18T00:00:01.000Z"));
    report(ARENA, "reset-orders", "started", [], "scoped", Date.parse("2026-10-18T00:00:02.000Z"));
    store.ingest(arenaSnapshot("arena_closed", "2026-10-18T00:00:03.000Z"));
    report(ARENA, "seed-reference", "started", [], "scoped", Date.parse("2026-10-18T00:00:04.000Z"));

    useDashboardStore.getState().scrubArena(ARENA, 2);
    render(<ScopedPlaybookPanel arenaId={ARENA} />);

    expect(screen.getByTestId("scoped-playbook-reset-orders").textContent).toContain("started");
    expect(screen.queryByTestId("scoped-playbook-seed-reference")).toBeNull();
  });
});
