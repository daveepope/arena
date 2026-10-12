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
    ["some_unknown_state", "neutral"],
  ])("state %s returns classes containing %s", (state: string, expectedColor: string) => {
    expect(nodeClasses("dependency", state)).toContain(expectedColor);
  });

  it("started state includes a pulse animation class", () => {
    expect(nodeClasses("dependency", "started")).toContain("animate-pulse-soft");
  });

  it("starting state includes a pulse animation class", () => {
    expect(nodeClasses("dependency", "starting")).toContain("animate-pulse-soft");
  });

  it("not_started state has no pulse animation class", () => {
    expect(nodeClasses("dependency", "not_started")).not.toContain("animate-pulse");
  });

  it("recorded faults return red classes with a pulse animation", () => {
    const classes = nodeClasses("dependency", "stopped", true);
    expect(classes).toContain("red");
    expect(classes).toContain("animate-pulse");
  });

  it("dependency kind uses a sky border regardless of state", () => {
    expect(nodeClasses("dependency", "started")).toContain("border-sky-400");
  });

  it("component kind uses a fuchsia border regardless of state", () => {
    expect(nodeClasses("component", "started")).toContain("border-fuchsia-400");
  });

  it("dependency and component borders differ for the same state", () => {
    const dependency = nodeClasses("dependency", "faulted", true);
    const component = nodeClasses("component", "faulted", true);
    expect(dependency).toContain("border-sky-400");
    expect(component).toContain("border-fuchsia-400");
  });

  it("stopped state with recorded faults still returns red classes", () => {
    expect(nodeClasses("dependency", "stopped", true)).toContain("red");
  });

  it("started state with recorded faults still returns red classes", () => {
    expect(nodeClasses("dependency", "started", true)).toContain("red");
  });

  it("stopped state with no faults returns neutral classes", () => {
    expect(nodeClasses("dependency", "stopped", false)).toContain("neutral");
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
    expect(detail).toContain("faults: none");
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
