import { describe, expect, it } from "vitest";

import { arenaClasses, formatDetail, nodeClasses } from "./presentation";
import type { Fault } from "./types";

describe("nodeClasses", () => {
  it.each([
    ["not_started", "neutral"],
    ["starting", "amber"],
    ["readiness_check", "amber"],
    ["started", "emerald"],
    ["stopping", "orange"],
    ["stopped", "neutral"],
    ["faulted", "red"],
    ["some_unknown_state", "neutral"],
  ])("state %s returns classes containing %s", (state: string, expectedColor: string) => {
    expect(nodeClasses(state)).toContain(expectedColor);
  });

  it("started state includes a pulse animation class", () => {
    expect(nodeClasses("started")).toContain("animate-pulse-soft");
  });

  it("faulted state includes a pulse animation class", () => {
    expect(nodeClasses("faulted")).toContain("animate-pulse");
  });
});

describe("arenaClasses", () => {
  it.each([
    ["arena_open", "emerald"],
    ["arena_faulted", "red"],
    ["arena_closed", "neutral"],
    ["arena_closing", "orange"],
    ["components_stopping", "orange"],
    ["dependencies_stopped", "orange"],
    ["arena_teardown", "orange"],
    ["arena_starting", "amber"],
  ])("state %s returns classes containing %s", (state: string, expectedColor: string) => {
    expect(arenaClasses(state)).toContain(expectedColor);
  });
});

describe("formatDetail", () => {
  it("no faults includes none in the faults section", () => {
    const detail = formatDetail("postgres", "dependency", "started", [], 0);

    expect(detail).toContain("id: postgres");
    expect(detail).toContain("kind: dependency");
    expect(detail).toContain("state: started");
    expect(detail).toContain("children: 0");
    expect(detail).toContain("faults:\nnone");
  });

  it("with faults lists each fault on its own line", () => {
    const faults: Fault[] = [
      { id: "f1", subject: "dependency", message: "boom", at: "2026-01-01T00:00:00.000Z", faults: [] },
      { id: "f2", subject: "component", message: "bang", at: "2026-01-01T00:00:01.000Z", faults: [] },
    ];

    const detail = formatDetail("api", "component", "faulted", faults, 2);

    expect(detail).toContain("f1 (dependency) @ 2026-01-01T00:00:00.000Z: boom");
    expect(detail).toContain("f2 (component) @ 2026-01-01T00:00:01.000Z: bang");
  });
});
