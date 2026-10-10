import { useEffect } from "react";
import {
  Background,
  Handle,
  MarkerType,
  Panel,
  Position,
  ReactFlow,
  useEdgesState,
  useNodesState,
  type Edge,
  type Node,
  type NodeProps,
} from "@xyflow/react";

import { ArenaEdge, type ArenaEdgeData } from "./ArenaEdge";
import { dagreLayout } from "./dagreLayout";
import { computeLayout, type LayoutNode } from "./layout";
import { arenaClasses, formatDetail, nodeClasses } from "./presentation";
import { useDashboardStore } from "./store";

const edgeTypes = { arena: ArenaEdge };

const HIDDEN_HANDLE_STYLE = { opacity: 0, width: 1, height: 1, border: "none" };

function ArenaNode({ data }: NodeProps) {
  const node = data as unknown as LayoutNode;
  const select = useDashboardStore((s) => s.select);

  return (
    <div
      className={`flex h-20 w-20 cursor-pointer flex-col items-center justify-center rounded-full border-2 text-center text-[11px] transition-all duration-300 ${
        node.visible ? "scale-100 opacity-100" : "pointer-events-none scale-[0.3] opacity-0"
      } ${nodeClasses(node.state)}`}
      onClick={() =>
        select({
          title: node.label,
          detail: formatDetail(node.label, node.kind, node.state, node.faults, node.childCount),
        })
      }
    >
      <Handle type="source" position={Position.Top} style={HIDDEN_HANDLE_STYLE} />
      <Handle type="target" position={Position.Bottom} style={HIDDEN_HANDLE_STYLE} />
      <span>{node.label}</span>
      <span className="text-[9px] uppercase tracking-wide opacity-70">{node.kind}</span>
      <span className="text-[9px] uppercase tracking-wide opacity-60">{node.state}</span>
    </div>
  );
}

const nodeTypes = { arena: ArenaNode };

export function ArenaGraph({ arenaId }: { arenaId: string }) {
  const snapshot = useDashboardStore((s) => s.arenas.get(arenaId)?.snapshot ?? null);
  const select = useDashboardStore((s) => s.select);

  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);

  useEffect(() => {
    if (!snapshot) return;
    const layout = computeLayout(snapshot);
    const positions = dagreLayout(layout.nodes, layout.edges);

    setNodes((current) => {
      const existing = new Map(current.map((n) => [n.id, n]));
      return layout.nodes.map((n) => {
        const prev = existing.get(n.id);
        return {
          id: n.id,
          type: "arena",
          position: prev?.position ?? positions.get(n.id) ?? { x: 0, y: 0 },
          data: n as unknown as Record<string, unknown>,
        };
      });
    });

    setEdges(
      layout.edges.map((e) => ({
        id: e.id,
        source: e.source,
        target: e.target,
        type: "arena",
        data: { faulted: e.faulted, decorative: e.decorative, visible: e.visible } satisfies ArenaEdgeData,
        markerEnd: { type: MarkerType.ArrowClosed, color: e.faulted ? "#e74c3c" : "#9aa0a6" },
      }))
    );
  }, [snapshot, setNodes, setEdges]);

  return (
    <div className="h-[480px] w-full overflow-hidden rounded-md bg-neutral-950">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        fitView
        proOptions={{ hideAttribution: true }}
      >
        <Background color="#2a2a2a" gap={24} />
        <Panel position="top-left">
          <button
            type="button"
            onClick={() =>
              snapshot &&
              select({
                title: `${snapshot.id} — state: ${snapshot.state}`,
                detail: formatDetail(
                  snapshot.id,
                  "arena",
                  snapshot.state,
                  snapshot.faults,
                  snapshot.dependencies.length + snapshot.components.length
                ),
              })
            }
            className={`rounded-md border px-3 py-1.5 text-xs uppercase tracking-wide ${
              snapshot ? arenaClasses(snapshot.state) : "border-neutral-700 text-neutral-500"
            }`}
          >
            {snapshot ? `${snapshot.id} — state: ${snapshot.state}` : "waiting for events..."}
          </button>
        </Panel>
      </ReactFlow>
    </div>
  );
}
