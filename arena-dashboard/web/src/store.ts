import { create } from "zustand";

import { isArenaRestart } from "./presentation";
import type { ArenaState, CpuProfilerState, PlaybookEntry, PlaybookRowState, Subject } from "./types";

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

export interface ProfileView {
  componentId: string;
  position: { x: number; y: number };
  isPinned: boolean;
}

export interface DetailPanel extends Selected {
  key: number;
  isPinned: boolean;
}

export interface ProfilePanel extends ProfileView {
  key: number;
}

export interface HistoryEntry {
  at: number;
  snapshot?: ArenaState;
  playbook?: PlaybookEntry;
}

interface DiffState {
  log: LogEntry[];
  lastArenaState: string | null;
  lastSubjectStates: Map<string, SubjectSnapshot>;
  nextLogId: number;
  playbooks: Map<string, PlaybookRowState>;
}

interface ArenaEntry extends DiffState {
  snapshot: ArenaState;
  history: HistoryEntry[];
  isReplaying: boolean;
  replayIndex: number | null;
}

const MAX_HISTORY = 1000;

let panelKey = 0;

function nextPanelKey(): number {
  panelKey += 1;
  return panelKey;
}
const REPLAY_MIN_DELAY_MS = 150;
const REPLAY_MAX_DELAY_MS = 1500;

export function lastRecordedSnapshot(history: HistoryEntry[] | undefined): ArenaState | undefined {
  if (history === undefined) return undefined;
  for (let i = history.length - 1; i >= 0; i--) {
    const snapshot = history[i].snapshot;
    if (snapshot !== undefined) return snapshot;
  }
  return undefined;
}

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
    playbooks: prev?.playbooks ?? new Map(),
  };
}

function withPlaybook(
  playbooks: Map<string, PlaybookRowState>,
  reported: PlaybookEntry,
): Map<string, PlaybookRowState> {
  return new Map(playbooks).set(reported.playbookId, {
    state: reported.state,
    kind: reported.kind,
    playbookType: reported.playbookType,
    runs: reported.runs,
    faults: reported.faults,
  });
}

export function foldUpTo(
  history: HistoryEntry[],
  index: number,
): DiffState & { snapshot?: ArenaState } {
  let acc: (DiffState & { snapshot?: ArenaState }) | undefined;
  for (let i = 0; i <= index && i < history.length; i++) {
    const entry = history[i];
    if (entry.snapshot !== undefined) {
      acc = diffEntry(acc, entry.snapshot);
    } else if (entry.playbook !== undefined) {
      acc = {
        log: acc?.log ?? [],
        lastArenaState: acc?.lastArenaState ?? null,
        lastSubjectStates: acc?.lastSubjectStates ?? new Map(),
        nextLogId: acc?.nextLogId ?? 0,
        snapshot: acc?.snapshot,
        playbooks: withPlaybook(acc?.playbooks ?? new Map(), entry.playbook),
      };
    }
  }
  return (
    acc ?? {
      log: [],
      lastArenaState: null,
      lastSubjectStates: new Map(),
      nextLogId: 0,
      playbooks: new Map(),
    }
  );
}

interface DashboardStore {
  arenas: Map<string, ArenaEntry>;
  details: DetailPanel[];
  cpuProfiles: Map<string, CpuProfilerState>;
  profiles: ProfilePanel[];
  ingest: (next: ArenaState) => void;
  ingestHistory: (snapshots: ArenaState[], playbooks?: PlaybookEntry[]) => void;
  select: (selected: Selected) => void;
  clearSelected: () => void;
  closeDetail: (key: number) => void;
  toggleDetailPin: (key: number) => void;
  clearArenas: () => void;
  ingestCpuProfile: (
    componentId: string,
    state: string,
    reportReady: boolean,
    reason?: string,
  ) => void;
  ingestPlaybook: (arenaId: string, reported: PlaybookEntry) => void;
  openProfileView: (componentId: string, position: { x: number; y: number }) => void;
  closeProfileView: (key: number) => void;
  toggleProfileViewPin: (key: number) => void;
  replayArena: (arenaId: string) => void;
  stopReplay: (arenaId: string) => void;
  scrubArena: (arenaId: string, index: number) => void;
}

export const useDashboardStore = create<DashboardStore>((set, get) => ({
  arenas: new Map(),
  details: [],
  cpuProfiles: new Map(),
  profiles: [],

  ingest: (next) => {
    const { arenas } = get();
    const prevEntry = arenas.get(next.id);
    const prevLastRecorded = lastRecordedSnapshot(prevEntry?.history);
    const isRestart = isArenaRestart(prevLastRecorded?.state, next.state);
    const prev = isRestart ? undefined : prevEntry;
    const lastRecorded = isRestart ? undefined : prevLastRecorded;
    const isDuplicateOfLastRecorded =
      lastRecorded !== undefined && JSON.stringify(lastRecorded) === JSON.stringify(next);
    const history = isDuplicateOfLastRecorded
      ? prev?.history ?? []
      : [...(prev?.history ?? []), { at: Date.parse(next.at), snapshot: next }].slice(-MAX_HISTORY);
    const updated = new Map(arenas);
    if (prev && prev.replayIndex !== null) {
      updated.set(next.id, { ...prev, history });
    } else {
      updated.set(next.id, { ...diffEntry(prev, next), history, isReplaying: false, replayIndex: null });
    }
    set({ arenas: updated });
  },

  ingestHistory: (snapshots, playbooks = []) => {
    if (snapshots.length === 0) return;
    const arenaId = snapshots[0].id;
    const { arenas } = get();
    const prev = arenas.get(arenaId);
    const recorded: HistoryEntry[] = [
      ...snapshots.map((snapshot) => ({ at: Date.parse(snapshot.at), snapshot })),
      ...playbooks.map((playbook) => ({ at: playbook.at, playbook })),
    ].sort((left, right) => left.at - right.at);
    const lastRecordedAt = recorded[recorded.length - 1].at;
    const newerLocally = (prev?.history ?? []).filter((entry) => entry.at > lastRecordedAt);
    const merged = [...recorded, ...newerLocally].slice(-MAX_HISTORY);
    if (prev && prev.history.length >= merged.length) return;

    const folded = foldUpTo(merged, merged.length - 1);
    const updated = new Map(arenas);
    if (prev && (prev.isReplaying || prev.replayIndex !== null)) {
      updated.set(arenaId, { ...prev, history: merged });
    } else if (folded.snapshot !== undefined) {
      updated.set(arenaId, {
        ...folded,
        snapshot: folded.snapshot,
        history: merged,
        isReplaying: false,
        replayIndex: null,
      });
    }
    set({ arenas: updated });
  },

  select: (selected) =>
    set({
      details: [
        ...get().details.filter((panel) => panel.isPinned),
        { ...selected, key: nextPanelKey(), isPinned: false },
      ],
    }),
  clearSelected: () => set({ details: get().details.filter((panel) => panel.isPinned) }),
  closeDetail: (key) => set({ details: get().details.filter((panel) => panel.key !== key) }),
  toggleDetailPin: (key) =>
    set({
      details: get().details.map((panel) =>
        panel.key === key ? { ...panel, isPinned: !panel.isPinned } : panel,
      ),
    }),
  clearArenas: () =>
    set({
      arenas: new Map(),
      details: [],
      cpuProfiles: new Map(),
      profiles: [],
    }),

  ingestPlaybook: (arenaId, reported) => {
    const { arenas } = get();
    const prev = arenas.get(arenaId);
    if (prev === undefined) return;
    const alreadyRecorded = prev.history.some(
      (entry) =>
        entry.playbook !== undefined &&
        entry.playbook.playbookId === reported.playbookId &&
        entry.playbook.state === reported.state &&
        entry.playbook.runs === reported.runs &&
        entry.at === reported.at,
    );
    const history = alreadyRecorded
      ? prev.history
      : [...prev.history, { at: reported.at, playbook: reported }].slice(-MAX_HISTORY);
    const updated = new Map(arenas);
    updated.set(arenaId, {
      ...prev,
      history,
      playbooks:
        prev.replayIndex === null ? withPlaybook(prev.playbooks, reported) : prev.playbooks,
    });
    set({ arenas: updated });
  },

  ingestCpuProfile: (componentId, state, reportReady, reason) =>
    set({
      cpuProfiles: new Map(get().cpuProfiles).set(componentId, { state, reportReady, reason }),
    }),

  openProfileView: (componentId, position) =>
    set({
      profiles: [
        ...get().profiles.filter((panel) => panel.isPinned),
        { componentId, position, key: nextPanelKey(), isPinned: false },
      ],
    }),

  closeProfileView: (key) =>
    set({ profiles: get().profiles.filter((panel) => panel.key !== key) }),

  toggleProfileViewPin: (key) =>
    set({
      profiles: get().profiles.map((panel) =>
        panel.key === key ? { ...panel, isPinned: !panel.isPinned } : panel,
      ),
    }),

  replayArena: (arenaId) => {
    const entry = get().arenas.get(arenaId);
    if (!entry || entry.history.length === 0 || entry.isReplaying) return;
    const history = entry.history;

    const playStep = (index: number) => {
      const current = get().arenas.get(arenaId);
      if (!current || !current.isReplaying) return;

      const isLast = index >= history.length - 1;
      const folded = foldUpTo(history, index);
      const arenas = new Map(get().arenas);
      arenas.set(arenaId, {
        ...folded,
        snapshot: folded.snapshot ?? current.snapshot,
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
      playbooks: new Map(),
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
    const folded = foldUpTo(entry.history, clamped);
    const updated = new Map(arenas);
    updated.set(arenaId, {
      ...folded,
      snapshot: folded.snapshot ?? entry.snapshot,
      history: entry.history,
      isReplaying: false,
      replayIndex: atEnd ? null : clamped,
    });
    set({ arenas: updated });
  },
}));
