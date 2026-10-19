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

export interface RuntimeFrames {
  language: string;
  frames: string[];
}

export interface RenderedProfile {
  svg: string;
  hotspots: Hotspot[];
  application_hotspots?: Hotspot[];
  runtime_frames?: RuntimeFrames[];
}

export interface CpuProfilerState {
  state: string;
  reportReady: boolean;
}

export interface PlaybookRowState {
  state: string;
  kind: string;
  playbookType: string;
  runs: number;
  faults: Fault[];
}

export interface PlaybookEntry {
  at: number;
  playbookId: string;
  state: string;
  kind: string;
  playbookType: string;
  runs: number;
  faults: Fault[];
}
