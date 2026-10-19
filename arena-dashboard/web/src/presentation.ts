import type { Fault } from "./types";

export type NodeKind = "dependency" | "component";

function kindBorderClass(kind: NodeKind): string {
  return kind === "dependency" ? "border-sky-400" : "border-fuchsia-400";
}

function kindShapeClass(kind: NodeKind): string {
  return kind === "dependency" ? "rounded-md" : "rounded-full";
}

export function isActiveState(state: string): boolean {
  return (
    state === "starting" ||
    state === "readiness_check" ||
    state === "started" ||
    state === "running" ||
    state === "stopping"
  );
}

export function fillClasses(state: string): string {
  switch (state) {
    case "starting":
    case "readiness_check":
      return "bg-amber-500/60 text-amber-50";
    case "started":
    case "running":
      return "bg-emerald-500/60 text-emerald-50";
    case "stopping":
      return "bg-orange-500/60 text-orange-50";
    case "stopped":
      return "bg-slate-700/60 text-slate-200";
    case "not_started":
    default:
      return "bg-slate-800/60 text-slate-300";
  }
}

export function nodeClasses(kind: NodeKind, state: string, hasFaults = false): string {
  const border = kindBorderClass(kind);
  const shape = kindShapeClass(kind);
  if (hasFaults) {
    return `${border} ${shape} bg-rose-600/30 text-rose-100 animate-pulse`;
  }
  const fill = fillClasses(state);
  const pulse = isActiveState(state) ? "animate-pulse-soft" : "";
  return `${border} ${shape} ${fill} ${pulse}`.trim();
}

export function arenaClasses(state: string): string {
  switch (state) {
    case "arena_open":
      return "border-emerald-400 bg-emerald-500/60 text-emerald-50 animate-pulse-soft";
    case "arena_faulted":
      return "border-rose-500 bg-rose-600/30 text-rose-100 animate-pulse";
    case "arena_closed":
      return "border-slate-500 bg-slate-800/60 text-slate-200";
    case "arena_closing":
    case "components_stopping":
    case "components_stopped":
    case "dependencies_stopping":
    case "dependencies_stopped":
    case "arena_teardown":
      return "border-orange-400 bg-orange-500/25 text-orange-100 animate-pulse-soft";
    default:
      return "border-amber-400 bg-amber-500/20 text-amber-100 animate-pulse-soft";
  }
}

function formatFaults(faults: Fault[]): string {
  if (faults.length === 0) return "faults: none";
  const lines = faults.map((f) => `  ${f.id} (${f.subject}) @ ${f.at}: ${f.message}`).join("\n");
  return `faults:\n${lines}`;
}

export function formatDetail(id: string, kind: string, state: string, faults: Fault[], childCount: number): string {
  return [`id: ${id}`, `kind: ${kind}`, `state: ${state}`, `children: ${childCount}`, formatFaults(faults)].join(
    "\n"
  );
}

export function formatPlaybookDetail(
  id: string,
  playbookType: string,
  kind: string,
  state: string,
  runs: number,
  faults: Fault[]
): string {
  return [
    `id: ${id}`,
    `kind: playbook`,
    `type: ${playbookType} ${kind}`,
    `state: ${state}`,
    `runs: ${runs}`,
    formatFaults(faults),
  ].join("\n");
}

export const TERMINAL_ARENA_STATES = new Set(["arena_closed", "arena_faulted"]);
export const INITIAL_ARENA_STATE = "arena_starting";

export function isArenaRestart(previousState: string | undefined, nextState: string): boolean {
  return (
    previousState !== undefined &&
    TERMINAL_ARENA_STATES.has(previousState) &&
    nextState === INITIAL_ARENA_STATE
  );
}
