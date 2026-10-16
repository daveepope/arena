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
  position: { x: number; y: number };
}

interface SubjectSnapshot {
  id: string;
  kind: SubjectKind;
  state: string;
}

export interface HistoryEntry {
  at: number;
  snapshot: ArenaState;
}

interface DiffState {
  log: LogEntry[];
  lastArenaState: string | null;
  lastSubjectStates: Map<string, SubjectSnapshot>;
  nextLogId: number;
}

interface ArenaEntry extends DiffState {
  snapshot: ArenaState;
  history: HistoryEntry[];
  isReplaying: boolean;
  replayIndex: number | null;
}

const MAX_HISTORY = 1000;
const REPLAY_MIN_DELAY_MS = 150;
const REPLAY_MAX_DELAY_MS = 1500;

function flatten(list: Subject[], kind: SubjectKind, out: Map<string, SubjectSnapshot>): void {
  list.forEach((s) => {
    out.set(`${kind}:${s.id}`, { id: s.id, kind, state: s.state });
    flatten(s.children, kind, out);
  });
}

function diffEntry(prev: DiffState | undefined, next: ArenaState): DiffState & { snapshot: ArenaState } {
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

function diffUpTo(history: HistoryEntry[], index: number): DiffState & { snapshot: ArenaState } {
  let acc: DiffState | undefined;
  let result: (DiffState & { snapshot: ArenaState }) | undefined;
  for (let i = 0; i <= index; i++) {
    result = diffEntry(acc, history[i].snapshot);
    acc = result;
  }
  return result as DiffState & { snapshot: ArenaState };
}

interface DashboardStore {
  arenas: Map<string, ArenaEntry>;
  selected: Selected | null;
  ingest: (next: ArenaState) => void;
  ingestHistory: (snapshots: ArenaState[]) => void;
  select: (selected: Selected) => void;
  clearSelected: () => void;
  clearArenas: () => void;
  replayArena: (arenaId: string) => void;
  stopReplay: (arenaId: string) => void;
  scrubArena: (arenaId: string, index: number) => void;
}

export const useDashboardStore = create<DashboardStore>((set, get) => ({
  arenas: new Map(),
  selected: null,

  ingest: (next) => {
    const { arenas } = get();
    const prev = arenas.get(next.id);
    const lastRecorded = prev?.history[prev.history.length - 1]?.snapshot;
    const isDuplicateOfLastRecorded =
      lastRecorded !== undefined && JSON.stringify(lastRecorded) === JSON.stringify(next);
    const history = isDuplicateOfLastRecorded
      ? prev?.history ?? []
      : [...(prev?.history ?? []), { at: Date.now(), snapshot: next }].slice(-MAX_HISTORY);
    const updated = new Map(arenas);
    if (prev && prev.replayIndex !== null) {
      updated.set(next.id, { ...prev, history });
    } else {
      updated.set(next.id, { ...diffEntry(prev, next), history, isReplaying: false, replayIndex: null });
    }
    set({ arenas: updated });
  },

  ingestHistory: (snapshots) => {
    if (snapshots.length === 0) return;
    const arenaId = snapshots[0].id;
    const { arenas } = get();
    const prev = arenas.get(arenaId);
    const history = snapshots.map((snapshot) => ({ at: Date.parse(snapshot.at), snapshot }));
    const newerLocally = (prev?.history ?? []).filter((entry) => entry.snapshot.at > snapshots[snapshots.length - 1].at);
    const merged = [...history, ...newerLocally].slice(-MAX_HISTORY);
    if (prev && prev.history.length >= merged.length) return;

    const updated = new Map(arenas);
    if (prev && (prev.isReplaying || prev.replayIndex !== null)) {
      updated.set(arenaId, { ...prev, history: merged });
    } else {
      updated.set(arenaId, {
        ...diffUpTo(merged, merged.length - 1),
        history: merged,
        isReplaying: false,
        replayIndex: null,
      });
    }
    set({ arenas: updated });
  },

  select: (selected) => set({ selected }),
  clearSelected: () => set({ selected: null }),
  clearArenas: () => set({ arenas: new Map(), selected: null }),

  replayArena: (arenaId) => {
    const entry = get().arenas.get(arenaId);
    if (!entry || entry.history.length === 0 || entry.isReplaying) return;
    const history = entry.history;

    const playStep = (index: number) => {
      const current = get().arenas.get(arenaId);
      if (!current || !current.isReplaying) return;

      const isLast = index >= history.length - 1;
      const arenas = new Map(get().arenas);
      arenas.set(arenaId, {
        ...diffUpTo(history, index),
        history: current.history,
        isReplaying: !isLast,
        replayIndex: isLast ? null : index,
      });
      set({ arenas });

      if (isLast) return;
      const delay = Math.min(
        REPLAY_MAX_DELAY_MS,
        Math.max(REPLAY_MIN_DELAY_MS, history[index + 1].at - history[index].at)
      );
      setTimeout(() => playStep(index + 1), delay);
    };

    const resetBase: ArenaEntry = {
      snapshot: entry.snapshot,
      log: [],
      history,
      isReplaying: true,
      replayIndex: 0,
      lastArenaState: null,
      lastSubjectStates: new Map(),
      nextLogId: 0,
    };
    set({ arenas: new Map(get().arenas).set(arenaId, resetBase) });
    setTimeout(() => playStep(0), REPLAY_MIN_DELAY_MS);
  },

  stopReplay: (arenaId) => {
    const { arenas } = get();
    const entry = arenas.get(arenaId);
    if (!entry || !entry.isReplaying) return;
    const updated = new Map(arenas);
    updated.set(arenaId, { ...entry, isReplaying: false });
    set({ arenas: updated });
  },

  scrubArena: (arenaId, index) => {
    const { arenas } = get();
    const entry = arenas.get(arenaId);
    if (!entry || entry.history.length === 0) return;

    const clamped = Math.max(0, Math.min(index, entry.history.length - 1));
    const atEnd = clamped === entry.history.length - 1;
    const updated = new Map(arenas);
    updated.set(arenaId, {
      ...diffUpTo(entry.history, clamped),
      history: entry.history,
      isReplaying: false,
      replayIndex: atEnd ? null : clamped,
    });
    set({ arenas: updated });
  },
}));
