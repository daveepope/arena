export interface Fault {
  id: string;
  subject: string;
  message: string;
  at: string;
  faults: Fault[];
}

export interface Subject {
  id: string;
  state: string;
  faults: Fault[];
  children: Subject[];
  component_ids?: string[];
}

export interface ArenaState {
  id: string;
  state: string;
  at: string;
  dependencies: Subject[];
  components: Subject[];
  faults: Fault[];
}

export type Severity = "low" | "medium" | "high" | "critical";

export interface Hotspot {
  function: string;
  self_count: number;
  self_pct: number;
  severity: Severity;
}

export interface RenderedProfile {
  svg: string;
  hotspots: Hotspot[];
}

export interface CpuProfilerState {
  state: string;
  reportReady: boolean;
}

export interface ScopedPlaybookState {
  state: string;
  kind: string;
  faults: Fault[];
}

export interface ScopedPlaybookEntry {
  at: number;
  playbookId: string;
  state: string;
  kind: string;
  faults: Fault[];
}
