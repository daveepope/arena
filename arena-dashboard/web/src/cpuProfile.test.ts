import "./testSetup";

import { describe, expect, it } from "vitest";

import {
  dimUnmatchedFrames,
  flamegraphDocument,
  flamegraphFrames,
  formatSelfPct,
  frameFunctionName,
  highlightFrames,
  isProfileAvailable,
  matchesFilter,
  isProfileFailed,
  profilerFailureReason,
  profilerLabel,
  severityClasses,
  visibleHotspots,
} from "./cpuProfile";
import type { Hotspot, Severity } from "./types";

function hotspot(overrides: Partial<Hotspot> = {}): Hotspot {
  return { function: "compute", self_count: 42, self_pct: 84.0, severity: "critical", ...overrides };
}

function flamegraphRoot(functions: string[]): HTMLElement {
  const root = document.createElement("div");
  root.innerHTML = `<svg><svg id="frames">${functions
    .map(
      (func) => `<g><title>${func} (42 samples, 84.00%)</title><rect width="10" height="10"></rect></g>`,
    )
    .join("")}</svg></svg>`;
  return root;
}

describe("profilerLabel", () => {
  it.each([
    ["starting", false, "cpu profiler starting"],
    ["started", false, "cpu profiler started"],
    ["stopping", false, "cpu profiler stopping"],
    ["stopped", false, "cpu profiler stopped"],
    ["stopped", true, "cpu profile report"],
    ["faulted", false, "cpu profiler faulted"],
  ])("%s with report %s returns %s", (state, reportReady, expected) => {
    expect(profilerLabel(state, reportReady)).toBe(expected);
  });

  it("unknown state returns the state prefixed", () => {
    expect(profilerLabel("not_started", false)).toBe("cpu profiler not_started");
  });
});

describe("isProfileAvailable", () => {
  it.each([
    ["stopped", true, true],
    ["stopped", false, false],
    ["started", true, false],
    [undefined, true, false],
  ])("%s with report %s returns %s", (state, reportReady, expected) => {
    expect(isProfileAvailable(state, reportReady)).toBe(expected);
  });
});

describe("isProfileFailed", () => {
  it.each([
    ["faulted", true],
    ["stopped", false],
    [undefined, false],
  ])("%s returns %s", (state, expected) => {
    expect(isProfileFailed(state)).toBe(expected);
  });
});

describe("profilerFailureReason", () => {
  it("faulted with a reason returns that reason", () => {
    expect(profilerFailureReason("faulted", "py-spy recorded no samples")).toBe(
      "py-spy recorded no samples",
    );
  });

  it("faulted without a reason returns a stand-in", () => {
    expect(profilerFailureReason("faulted", undefined)).toBe(
      "the profiler reported no reason for the failure",
    );
  });

  it.each([
    ["stopped", "ignored"],
    [undefined, undefined],
  ])("%s returns null", (state, reason) => {
    expect(profilerFailureReason(state, reason)).toBeNull();
  });
});

describe("matchesFilter", () => {
  it("empty pattern matches everything", () => {
    expect(matchesFilter("compute", "   ")).toBe(true);
  });

  it("regex pattern matches case insensitively", () => {
    expect(matchesFilter("Compute", "^comp")).toBe(true);
  });

  it("regex pattern that does not match returns false", () => {
    expect(matchesFilter("compute", "^idle$")).toBe(false);
  });

  it("invalid regex falls back to substring matching", () => {
    expect(matchesFilter("compute(", "compute(")).toBe(true);
    expect(matchesFilter("idle", "compute(")).toBe(false);
  });
});

describe("frameFunctionName", () => {
  it("title with sample counts returns the function alone", () => {
    expect(frameFunctionName("main;handler (42 samples, 84.00%)")).toBe("main;handler");
  });

  it("title without sample counts returns it unchanged", () => {
    expect(frameFunctionName("all")).toBe("all");
  });

  it("missing title returns an empty name", () => {
    expect(frameFunctionName("")).toBe("");
  });
});

describe("severityClasses", () => {
  it.each<[Severity, string]>([
    ["critical", "rose"],
    ["high", "orange"],
    ["medium", "amber"],
    ["low", "slate"],
  ])("%s is coloured with %s", (severity, expectedColour) => {
    expect(severityClasses(severity)).toContain(expectedColour);
  });

  it("unknown severity falls back to the low styling", () => {
    expect(severityClasses("unknown" as Severity)).toBe(severityClasses("low"));
  });
});

describe("formatSelfPct", () => {
  it("rounds to one decimal place", () => {
    expect(formatSelfPct(84.049)).toBe("84.0%");
  });
});

describe("visibleHotspots", () => {
  it("filter keeps only matching functions", () => {
    const rows = [hotspot({ function: "compute" }), hotspot({ function: "idle" })];

    expect(visibleHotspots(rows, "idle").map((h) => h.function)).toEqual(["idle"]);
  });

  it("empty filter keeps every function", () => {
    const rows = [hotspot({ function: "compute" }), hotspot({ function: "idle" })];

    expect(visibleHotspots(rows, "")).toHaveLength(2);
  });
});

describe("flamegraphFrames", () => {
  it("svg with frames returns each function name", () => {
    const root = flamegraphRoot(["compute", "idle"]);

    expect(flamegraphFrames(root).map((f) => f.func)).toEqual(["compute", "idle"]);
  });

  it("frame without a title returns an empty name", () => {
    const root = document.createElement("div");
    root.innerHTML = `<svg><svg id="frames"><g><rect></rect></g></svg></svg>`;

    expect(flamegraphFrames(root).map((f) => f.func)).toEqual([""]);
  });
});

describe("dimUnmatchedFrames", () => {
  it("filter dims the frames that do not match", () => {
    const root = flamegraphRoot(["compute", "idle"]);

    const matched = dimUnmatchedFrames(root, "compute");

    const frames = flamegraphFrames(root);
    expect(matched).toBe(1);
    expect(frames[0].group.style.opacity).toBe("1");
    expect(frames[1].group.style.opacity).toBe("0.15");
  });

  it("cleared filter restores every frame", () => {
    const root = flamegraphRoot(["compute", "idle"]);
    dimUnmatchedFrames(root, "compute");

    const matched = dimUnmatchedFrames(root, "");

    expect(matched).toBe(2);
    flamegraphFrames(root).forEach((frame) => expect(frame.group.style.opacity).toBe("1"));
  });
});

describe("highlightFrames", () => {
  it("hovered function outlines only its own frames", () => {
    const root = flamegraphRoot(["compute", "idle", "compute"]);

    const highlighted = highlightFrames(root, "compute");

    const rects = Array.from(root.querySelectorAll<SVGRectElement>("rect"));
    expect(highlighted).toBe(2);
    expect(rects[0].style.stroke).not.toBe("");
    expect(rects[1].style.stroke).toBe("");
  });

  it("no hovered function clears every outline", () => {
    const root = flamegraphRoot(["compute"]);
    highlightFrames(root, "compute");

    const highlighted = highlightFrames(root, null);

    expect(highlighted).toBe(0);
    expect(root.querySelector<SVGRectElement>("rect")!.style.stroke).toBe("");
  });

  it("frame without a rect is skipped", () => {
    const root = document.createElement("div");
    root.innerHTML = `<svg><svg id="frames"><g><title>compute (1 samples, 1.00%)</title></g></svg></svg>`;

    expect(highlightFrames(root, "compute")).toBe(0);
  });
});

describe("flamegraphDocument", () => {
  it("svg is wrapped in a document with the flamegraph background", () => {
    const wrapped = flamegraphDocument("<svg id='graph'></svg>");

    expect(wrapped).toContain("<!doctype html>");
    expect(wrapped).toContain("<svg id='graph'></svg>");
    expect(wrapped).toContain("#1e1e1e");
  });
});
