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
  // Not present in a real Arena snapshot today - Arena's ComponentState has
  // no field recording which dependencies a component uses. Optional so the
  // dashboard renders real arenas with no fake links at all; only present
  // when a producer (e.g. the demo script) chooses to declare it.
  dependency_ids?: string[];
}

export interface ArenaState {
  id: string;
  state: string;
  at: string;
  dependencies: Subject[];
  components: Subject[];
  faults: Fault[];
}
