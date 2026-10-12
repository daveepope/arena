import type { Fault } from "./types";

export type NodeKind = "dependency" | "component";

function kindBorderClass(kind: NodeKind): string {
  return kind === "dependency" ? "border-sky-400" : "border-fuchsia-400";
}

export function isActiveState(state: string): boolean {
  return state === "starting" || state === "readiness_check" || state === "started" || state === "stopping";
}

function fillClasses(state: string): string {
  switch (state) {
    case "starting":
    case "readiness_check":
      return "bg-amber-500/60 text-amber-50";
    case "started":
      return "bg-emerald-500/60 text-emerald-50";
    case "stopping":
      return "bg-orange-500/60 text-orange-50";
    case "stopped":
      return "bg-neutral-700/60 text-neutral-300";
    case "not_started":
    default:
      return "bg-neutral-800/60 text-neutral-400";
  }
}

export function nodeClasses(kind: NodeKind, state: string, hasFaults = false): string {
  const border = kindBorderClass(kind);
  if (hasFaults) {
    return `${border} bg-red-600/30 text-red-100 animate-pulse`;
  }
  const fill = fillClasses(state);
  const pulse = isActiveState(state) ? "animate-pulse-soft" : "";
  return `${border} ${fill} ${pulse}`.trim();
}

export function arenaClasses(state: string): string {
  switch (state) {
    case "arena_open":
      return "border-emerald-400 bg-emerald-500/60 text-emerald-50 animate-pulse-soft";
    case "arena_faulted":
      return "border-red-500 bg-red-600/30 text-red-100 animate-pulse";
    case "arena_closed":
      return "border-neutral-500 bg-neutral-800/60 text-neutral-300";
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
