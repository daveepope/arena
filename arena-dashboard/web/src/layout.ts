import type { ArenaState, Fault, Subject } from "./types";

export type SubjectKind = "dependency" | "component";

export interface LayoutNode {
  id: string;
  label: string;
  kind: SubjectKind;
  state: string;
  faults: Fault[];
  childCount: number;
  visible: boolean;
  dependencyIds: string[];
  depth: number;
}

export interface LayoutEdge {
  id: string;
  source: string;
  target: string;
  faulted: boolean;
  decorative: boolean;
  visible: boolean;
}

export interface Layout {
  nodes: LayoutNode[];
  edges: LayoutEdge[];
}

function isVisible(state: string): boolean {
  return state !== "not_started";
}

function walk(
  list: Subject[],
  depth: number,
  parentId: string | null,
  kind: SubjectKind,
  pathPrefix: string,
  nodes: LayoutNode[],
  edges: LayoutEdge[]
): void {
  list.forEach((subject) => {
    const id = `${pathPrefix}>${kind}:${subject.id}`;
    const visible = isVisible(subject.state);

    nodes.push({
      id,
      label: subject.id,
      kind,
      state: subject.state,
      faults: subject.faults,
      childCount: subject.children.length,
      visible,
      dependencyIds: subject.dependency_ids ?? [],
      depth,
    });

    // Edge points from the nested child toward its structural parent: the
    // child must be ready first, the parent waits on it - same "tip = the
    // thing that waits" convention as the dependency mesh below.
    if (depth > 0 && parentId) {
      edges.push({
        id: `edge:${id}`,
        source: id,
        target: parentId,
        faulted: subject.state === "faulted",
        decorative: false,
        visible,
      });
    }

    if (subject.children.length > 0) {
      walk(subject.children, depth + 1, id, kind, id, nodes, edges);
    }
  });
}

// Arena's lifecycle snapshot has no field recording which dependencies a
// given component actually uses - ComponentState only has id/state/faults/
// children. A real arena never sends dependency_ids, so no link is drawn at
// all for it: that's more honest than guessing. A link is only drawn when a
// subject explicitly declares dependency_ids (e.g. the demo script, for
// illustration) - resolved by raw dependency id, tip at the component that
// declared it (the component waits on its declared dependencies).
//
// Only top-level (depth 0) dependencies are resolvable targets: a nested
// dependency (e.g. http-dep-2 under postgres) already has its own real
// structural edge to its parent, so a component-level link must stop at
// that parent rather than skip past it to reach the nested one directly.
//
// Positions are not computed here - ArenaGraph lays the tree out with
// dagre (bottom-to-top: dependencies/children at the bottom, the
// components/parents that wait on them above).
export function computeLayout(state: ArenaState): Layout {
  const nodes: LayoutNode[] = [];
  const edges: LayoutEdge[] = [];

  walk(state.dependencies, 0, null, "dependency", "root", nodes, edges);
  walk(state.components, 0, null, "component", "root", nodes, edges);

  const depByRawId = new Map(
    nodes.filter((n) => n.kind === "dependency" && n.depth === 0).map((n) => [n.label, n])
  );
  const comps = nodes.filter((n) => n.kind === "component");

  comps.forEach((comp) => {
    comp.dependencyIds.forEach((rawId) => {
      const dep = depByRawId.get(rawId);
      if (!dep) return;
      edges.push({
        id: `link:${dep.id}>${comp.id}`,
        source: dep.id,
        target: comp.id,
        faulted: false,
        decorative: true,
        visible: comp.visible && dep.visible,
      });
    });
  });

  return { nodes, edges };
}
