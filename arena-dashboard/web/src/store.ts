import { create } from "zustand";

import type { ArenaState, Subject } from "./types";

export type SubjectKind = "dependency" | "component";

export interface LogEntry {
  id: number;
  text: string;
  isFault: boolean;
}

export interface Selected {
  title: string;
  detail: string;
}

interface SubjectSnapshot {
  id: string;
  kind: SubjectKind;
  state: string;
}

interface ArenaEntry {
  snapshot: ArenaState;
  log: LogEntry[];
  lastArenaState: string | null;
  lastSubjectStates: Map<string, SubjectSnapshot>;
  nextLogId: number;
}

function flatten(list: Subject[], kind: SubjectKind, out: Map<string, SubjectSnapshot>): void {
  list.forEach((s) => {
    out.set(`${kind}:${s.id}`, { id: s.id, kind, state: s.state });
    flatten(s.children, kind, out);
  });
}

function nextEntry(prev: ArenaEntry | undefined, next: ArenaState): ArenaEntry {
  const subjectStates = new Map<string, SubjectSnapshot>();
  flatten(next.dependencies, "dependency", subjectStates);
  flatten(next.components, "component", subjectStates);

  const lastSubjectStates = prev?.lastSubjectStates ?? new Map();
  const lastArenaState = prev?.lastArenaState ?? null;
  let id = prev?.nextLogId ?? 0;
  const entries: LogEntry[] = [];

  for (const [key, nextSubject] of subjectStates) {
    const prevSubject = lastSubjectStates.get(key);
    if (!prevSubject || prevSubject.state !== nextSubject.state) {
      entries.push({
        id: id++,
        text: `[${next.at}] ${nextSubject.kind} ${nextSubject.id}: ${prevSubject ? prevSubject.state : "(new)"} -> ${nextSubject.state}`,
        isFault: nextSubject.state === "faulted",
      });
    }
  }
  if (next.state !== lastArenaState) {
    entries.push({
      id: id++,
      text: `[${next.at}] arena ${next.id}: ${lastArenaState ?? "(new)"} -> ${next.state}`,
      isFault: next.state === "arena_faulted",
    });
  }
  if (entries.length === 0) {
    entries.push({ id: id++, text: `[${next.at}] ${next.id}: no change`, isFault: false });
  }

  return {
    snapshot: next,
    lastArenaState: next.state,
    lastSubjectStates: subjectStates,
    log: [...(prev?.log ?? []), ...entries],
    nextLogId: id,
  };
}

interface DashboardStore {
  arenas: Map<string, ArenaEntry>;
  selected: Selected | null;
  ingest: (next: ArenaState) => void;
  select: (selected: Selected) => void;
  clearSelected: () => void;
}

export const useDashboardStore = create<DashboardStore>((set, get) => ({
  arenas: new Map(),
  selected: null,

  ingest: (next) => {
    const { arenas } = get();
    const updated = new Map(arenas);
    updated.set(next.id, nextEntry(arenas.get(next.id), next));
    set({ arenas: updated });
  },

  select: (selected) => set({ selected }),
  clearSelected: () => set({ selected: null }),
}));
