import "./testSetup";

import { describe, expect, it } from "vitest";

import { arenaClasses, formatDetail, nodeClasses } from "./presentation";
import type { Fault } from "./types";

describe("nodeClasses", () => {
  it.each([
    ["not_started", "slate"],
    ["starting", "amber"],
    ["readiness_check", "amber"],
    ["started", "emerald"],
    ["stopping", "orange"],
    ["stopped", "slate"],
    ["some_unknown_state", "slate"],
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

  it("recorded faults return rose classes with a pulse animation", () => {
    const classes = nodeClasses("dependency", "stopped", true);
    expect(classes).toContain("rose");
    expect(classes).toContain("animate-pulse");
  });

  it("dependency kind uses a square shape", () => {
    expect(nodeClasses("dependency", "started")).toContain("rounded-md");
  });

  it("component kind uses a pill shape", () => {
    expect(nodeClasses("component", "started")).toContain("rounded-full");
  });

  it("dependency and component shapes differ for the same state", () => {
    const dependency = nodeClasses("dependency", "faulted", true);
    const component = nodeClasses("component", "faulted", true);
    expect(dependency).toContain("rounded-md");
    expect(component).toContain("rounded-full");
  });

  it("dependency kind uses a sky border regardless of state", () => {
    expect(nodeClasses("dependency", "started")).toContain("border-sky-400");
    expect(nodeClasses("dependency", "not_started")).toContain("border-sky-400");
  });

  it("component kind uses a fuchsia border regardless of state", () => {
    expect(nodeClasses("component", "started")).toContain("border-fuchsia-400");
    expect(nodeClasses("component", "not_started")).toContain("border-fuchsia-400");
  });

  it("kind border is the only border class so state fill cannot override it", () => {
    const classes = nodeClasses("dependency", "started").split(" ").filter((c) => c.startsWith("border-"));
    expect(classes).toEqual(["border-sky-400"]);
  });

  it("stopped state with recorded faults still returns rose classes", () => {
    expect(nodeClasses("dependency", "stopped", true)).toContain("rose");
  });

  it("started state with recorded faults still returns rose classes", () => {
    expect(nodeClasses("dependency", "started", true)).toContain("rose");
  });

  it("stopped state with no faults returns slate classes", () => {
    expect(nodeClasses("dependency", "stopped", false)).toContain("slate");
  });
});

describe("arenaClasses", () => {
  it.each([
    ["arena_open", "emerald"],
    ["arena_faulted", "rose"],
    ["arena_closed", "slate"],
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
