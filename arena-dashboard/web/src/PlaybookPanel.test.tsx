import "./testSetup";

import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { PlaybookPanel } from "./PlaybookPanel";
import { useDashboardStore } from "./store";
import type { ArenaState, Fault } from "./types";

function lastDetail() {
  const details = useDashboardStore.getState().details;
  return details.length === 0 ? null : details[details.length - 1];
}


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
  runs = 1,
) {
  useDashboardStore
    .getState()
    .ingestPlaybook(arenaId, { at, playbookId, state, kind, playbookType, runs, faults });
}

beforeEach(() => {
  useDashboardStore.getState().ingest(arenaSnapshot("arena_open"));
});

afterEach(() => {
  useDashboardStore.getState().clearArenas();
});

describe("PlaybookPanel", () => {
  it("arenaWithNoPlaybooksSaysNoneAreRegistered", () => {
    render(<PlaybookPanel arenaId={ARENA} />);

    expect(screen.getByTestId(`no-playbooks-${ARENA}`).textContent).toBe(
      "no playbooks registered",
    );
  });

  it("registeredPlaybookRendersGreyAndStatic", () => {
    report(ARENA, "reset-orders", "registered");

    render(<PlaybookPanel arenaId={ARENA} />);

    const row = screen.getByTestId("playbook-reset-orders");
    expect(row.textContent).toContain("registered");
    expect(row.className).not.toContain("emerald");
    expect(row.className).not.toContain("animate-pulse");
  });

  it("rowNamesHowManyTimesItHasRun", () => {
    report(ARENA, "reset-orders", "running", [], "scoped", 1, "managed", 1);
    report(ARENA, "apply-schema", "stopped", [], "startup", 2, "unmanaged", 4);

    render(<PlaybookPanel arenaId={ARENA} />);

    expect(screen.getByTestId("playbook-reset-orders").textContent).toContain("1 run");
    expect(screen.getByTestId("playbook-apply-schema").textContent).toContain("4 runs");
  });

  it("neverRunPlaybookNamesZeroRuns", () => {
    report(ARENA, "reset-orders", "registered", [], "scoped", 1, "managed", 0);

    render(<PlaybookPanel arenaId={ARENA} />);

    expect(screen.getByTestId("playbook-reset-orders").textContent).toContain("0 runs");
  });

  it("runningPlaybookRendersLive", () => {
    report(ARENA, "reset-orders", "running");

    render(<PlaybookPanel arenaId={ARENA} />);

    const row = screen.getByTestId("playbook-reset-orders");
    expect(row.className).toContain("emerald");
    expect(row.className).toContain("animate-pulse");
  });

  it("reportedPlaybooksRenderARowEachWithItsState", () => {
    report(ARENA, "reset-orders", "running");
    report(ARENA, "apply-schema", "stopped");

    render(<PlaybookPanel arenaId={ARENA} />);

    expect(screen.getByTestId("playbook-reset-orders").textContent).toContain("running");
    expect(screen.getByTestId("playbook-apply-schema").textContent).toContain("stopped");
  });

  it("playbooksAreListedInNameOrder", () => {
    report(ARENA, "reset-orders", "running");
    report(ARENA, "apply-schema", "running");

    render(<PlaybookPanel arenaId={ARENA} />);

    const rows = screen.getAllByRole("button").map((row) => row.getAttribute("data-testid"));
    expect(rows).toEqual(["playbook-apply-schema", "playbook-reset-orders"]);
  });

  it("laterStateForTheSamePlaybookReplacesTheEarlierOne", () => {
    report(ARENA, "reset-orders", "running", [], "scoped", 1);
    report(ARENA, "reset-orders", "stopped", [], "scoped", 2);

    render(<PlaybookPanel arenaId={ARENA} />);

    expect(screen.getAllByRole("button")).toHaveLength(1);
    expect(screen.getByTestId("playbook-reset-orders").textContent).toContain("stopped");
  });

  it("faultedPlaybookIsMarked", () => {
    report(ARENA, "reset-orders", "faulted", [fault("seed data rejected")]);

    render(<PlaybookPanel arenaId={ARENA} />);

    expect(screen.getByTestId("playbook-reset-orders").className).toContain("rose");
  });

  it("faultedStateWithNoFaultDetailIsStillMarked", () => {
    report(ARENA, "reset-orders", "faulted");

    render(<PlaybookPanel arenaId={ARENA} />);

    expect(screen.getByTestId("playbook-reset-orders").className).toContain("rose");
  });

  it("rowClickDetailNamesTheTypeKindAndRunCount", () => {
    report(ARENA, "apply-schema", "stopped", [], "startup", 1, "unmanaged", 3);
    render(<PlaybookPanel arenaId={ARENA} />);

    fireEvent.click(screen.getByTestId("playbook-apply-schema"));

    const detail = lastDetail()?.detail ?? "";
    expect(detail).toContain("kind: playbook");
    expect(detail).toContain("type: unmanaged startup");
    expect(detail).toContain("state: stopped");
    expect(detail).toContain("runs: 3");
  });

  it("rowClickOpensTheDetailDialogWithItsFaults", () => {
    report(ARENA, "reset-orders", "faulted", [fault("seed data rejected")]);
    render(<PlaybookPanel arenaId={ARENA} />);

    fireEvent.click(screen.getByTestId("playbook-reset-orders"));

    const selected = lastDetail();
    expect(selected?.title).toBe("reset-orders");
    expect(selected?.detail).toContain("kind: playbook");
    expect(selected?.detail).toContain("seed data rejected");
  });

  it("rowNamesItsPlaybookType", () => {
    report(ARENA, "seed-reference-data", "running", [], "scoped", 1, "unmanaged");

    render(<PlaybookPanel arenaId={ARENA} />);

    expect(screen.getByTestId("playbook-seed-reference-data").textContent).toContain(
      "unmanaged",
    );
  });

  it("startupPlaybookRowNamesItsKind", () => {
    report(ARENA, "apply-schema", "running", [], "startup");

    render(<PlaybookPanel arenaId={ARENA} />);

    expect(screen.getByTestId("playbook-apply-schema").textContent).toContain("startup");
  });

  it("anotherArenasPlaybooksAreNotShown", () => {
    useDashboardStore.getState().ingest({ ...arenaSnapshot("arena_open"), id: "arena-catalog" });
    report("arena-catalog", "reset-orders", "running");

    render(<PlaybookPanel arenaId={ARENA} />);

    expect(screen.queryByTestId("playbook-reset-orders")).toBeNull();
    expect(screen.getByTestId(`no-playbooks-${ARENA}`)).not.toBeNull();
  });

  it("replayedArenaShowsTheRowsAsOfTheReplayedMoment", () => {
    const store = useDashboardStore.getState();
    store.ingest(arenaSnapshot("arena_starting", "2026-10-18T00:00:01.000Z"));
    report(ARENA, "reset-orders", "running", [], "scoped", Date.parse("2026-10-18T00:00:02.000Z"));
    store.ingest(arenaSnapshot("arena_closed", "2026-10-18T00:00:03.000Z"));
    report(ARENA, "seed-reference", "running", [], "scoped", Date.parse("2026-10-18T00:00:04.000Z"));

    useDashboardStore.getState().scrubArena(ARENA, 2);
    render(<PlaybookPanel arenaId={ARENA} />);

    expect(screen.getByTestId("playbook-reset-orders").textContent).toContain("running");
    expect(screen.queryByTestId("playbook-seed-reference")).toBeNull();
  });
});
