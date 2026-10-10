import type { Fault } from "./types";

export function nodeClasses(state: string): string {
  switch (state) {
    case "not_started":
      return "border-neutral-600 bg-neutral-900 text-neutral-400";
    case "starting":
    case "readiness_check":
      return "border-amber-400 bg-amber-950/40 text-amber-100";
    case "started":
      return "border-emerald-500 bg-emerald-950/40 text-emerald-100 animate-pulse-soft";
    case "stopping":
      return "border-orange-400 bg-orange-950/40 text-orange-100";
    case "stopped":
      return "border-neutral-500 bg-neutral-800 text-neutral-300";
    case "faulted":
      return "border-red-500 bg-red-950/50 text-red-100 animate-pulse";
    default:
      return "border-neutral-600 bg-neutral-900 text-neutral-400";
  }
}

export function arenaClasses(state: string): string {
  switch (state) {
    case "arena_open":
      return "border-emerald-500 bg-emerald-950/40 text-emerald-100";
    case "arena_faulted":
      return "border-red-500 bg-red-950/50 text-red-100 animate-pulse";
    case "arena_closed":
      return "border-neutral-500 bg-neutral-800 text-neutral-300";
    case "arena_closing":
    case "components_stopping":
    case "components_stopped":
    case "dependencies_stopping":
    case "dependencies_stopped":
    case "arena_teardown":
      return "border-orange-400 bg-orange-950/40 text-orange-100";
    default:
      return "border-amber-400 bg-amber-950/40 text-amber-100";
  }
}

function formatFaults(faults: Fault[]): string {
  if (faults.length === 0) return "none";
  return faults.map((f) => `  ${f.id} (${f.subject}) @ ${f.at}: ${f.message}`).join("\n");
}

export function formatDetail(id: string, kind: string, state: string, faults: Fault[], childCount: number): string {
  return [
    `id: ${id}`,
    `kind: ${kind}`,
    `state: ${state}`,
    `children: ${childCount}`,
    `faults:\n${formatFaults(faults)}`,
  ].join("\n");
}
