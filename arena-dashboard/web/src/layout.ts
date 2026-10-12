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
  componentIds: string[];
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
      componentIds: subject.component_ids ?? [],
      depth,
    });

    if (parentId) {
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

export function computeLayout(state: ArenaState): Layout {
  const nodes: LayoutNode[] = [];
  const edges: LayoutEdge[] = [];

  walk(state.dependencies, 0, null, "dependency", "root", nodes, edges);
  walk(state.components, 0, null, "component", "root", nodes, edges);

  const compByRawId = new Map(nodes.filter((n) => n.kind === "component").map((n) => [n.label, n]));
  const deps = nodes.filter((n) => n.kind === "dependency");

  deps.forEach((dep) => {
    dep.componentIds.forEach((rawId) => {
      const comp = compByRawId.get(rawId);
      if (!comp) return;
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
