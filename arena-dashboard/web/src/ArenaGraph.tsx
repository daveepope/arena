import { useEffect } from "react";
import {
  Background,
  BackgroundVariant,
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
import { dagreLayout, estimateNodeWidth } from "./dagreLayout";
import { computeLayout, type LayoutNode } from "./layout";
import { isProfileAvailable, profilerLabel } from "./cpuProfile";
import { arenaClasses, formatDetail, isActiveState, nodeClasses } from "./presentation";
import { useDashboardStore } from "./store";

const edgeTypes = { arena: ArenaEdge };

const HIDDEN_HANDLE_STYLE = { opacity: 0, width: 1, height: 1, border: "none" };

const GRID_LINE_COLOR = "rgba(226, 232, 240, 0.1)";
const GRID_LINE_WIDTH = 0.5;

const PULSE_DURATION_MS = 2000;
const PULSE_SOFT_DURATION_MS = 2400;

function syncedAnimationDelay(durationMs: number): string {
  return `-${Date.now() % durationMs}ms`;
}

function pulseStyle(isPulsing: boolean, isStrongPulse: boolean): Record<string, unknown> {
  if (!isPulsing) return {};
  return { animationDelay: syncedAnimationDelay(isStrongPulse ? PULSE_DURATION_MS : PULSE_SOFT_DURATION_MS) };
}

function ProfilerLabel({ componentId, state }: { componentId: string; state: string }) {
  const openProfileView = useDashboardStore((s) => s.openProfileView);
  const isAvailable = isProfileAvailable(state);

  return (
    <button
      type="button"
      data-testid={`profiler-label-${componentId}`}
      disabled={!isAvailable}
      onClick={(e) => {
        e.stopPropagation();
        openProfileView(componentId, { x: e.clientX, y: e.clientY });
      }}
      className={`mt-0.5 rounded-sm border px-1.5 py-0.5 text-[9px] uppercase tracking-wide ${
        isAvailable
          ? "cursor-pointer border-teal-500 bg-teal-950/60 text-teal-300"
          : "cursor-default border-slate-600 bg-slate-900/60 text-slate-400"
      }`}
    >
      {profilerLabel(state)}
    </button>
  );
}

function ArenaNode({ data }: NodeProps) {
  const node = data as unknown as LayoutNode;
  const select = useDashboardStore((s) => s.select);
  const profilerState = useDashboardStore((s) => s.cpuProfiles.get(node.label));
  const hasFaults = node.faults.length > 0;
  const isPulsing = hasFaults || isActiveState(node.state);

  return (
    <div
      style={{ width: estimateNodeWidth(node.label), ...pulseStyle(isPulsing, hasFaults) }}
      className={`flex min-h-[70px] cursor-pointer flex-col items-center justify-center gap-0.5 break-words border-2 px-3 py-2 text-center text-[11px] transition-all duration-300 ${
        node.visible ? "scale-100 opacity-100" : "pointer-events-none scale-[0.3] opacity-0"
      } ${nodeClasses(node.kind, node.state, hasFaults)}`}
      onClick={(e) =>
        select({
          title: node.label,
          detail: formatDetail(node.label, node.kind, node.state, node.faults, node.childCount),
          position: { x: e.clientX, y: e.clientY },
        })
      }
    >
      <Handle type="source" position={Position.Top} style={HIDDEN_HANDLE_STYLE} />
      <Handle type="target" position={Position.Bottom} style={HIDDEN_HANDLE_STYLE} />
      <span>{node.label}</span>
      <span className="text-[9px] uppercase tracking-wide opacity-70">{node.kind}</span>
      <span className="text-[9px] uppercase tracking-wide opacity-60">{node.state}</span>
      {profilerState !== undefined && (
        <ProfilerLabel componentId={node.label} state={profilerState} />
      )}
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
    <div className="h-[480px] w-full overflow-hidden rounded-md bg-[#0e1330]">
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
        <Background
          variant={BackgroundVariant.Lines}
          color={GRID_LINE_COLOR}
          gap={24}
          lineWidth={GRID_LINE_WIDTH}
        />
        <Panel position="top-left">
          <button
            type="button"
            onClick={(e) =>
              snapshot &&
              select({
                title: `${snapshot.id} · state: ${snapshot.state}`,
                detail: formatDetail(
                  snapshot.id,
                  "arena",
                  snapshot.state,
                  snapshot.faults,
                  snapshot.dependencies.length + snapshot.components.length
                ),
                position: { x: e.clientX, y: e.clientY },
              })
            }
            style={snapshot ? pulseStyle(true, snapshot.state === "arena_faulted") : {}}
            className={`cursor-pointer rounded-md border px-3 py-1.5 text-xs uppercase tracking-wide ${
              snapshot ? arenaClasses(snapshot.state) : "border-slate-700 text-slate-500"
            }`}
          >
            {snapshot ? `${snapshot.id} · state: ${snapshot.state}` : "waiting for events..."}
          </button>
        </Panel>
      </ReactFlow>
    </div>
  );
}
