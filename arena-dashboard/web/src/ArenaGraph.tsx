import { useEffect, useRef } from "react";
import {
  Background,
  BackgroundVariant,
  Handle,
  MarkerType,
  Panel,
  Position,
  ReactFlow,
  useEdgesState,
  useNodesInitialized,
  useNodesState,
  useReactFlow,
  type Edge,
  type Node,
  type NodeProps,
} from "@xyflow/react";

import { ArenaEdge, type ArenaEdgeData } from "./ArenaEdge";
import { dagreLayout, estimateNodeWidth } from "./dagreLayout";
import { computeLayout, type LayoutNode } from "./layout";
import { profilerLabel } from "./cpuProfile";
import { CpuProfileButton } from "./CpuProfilePanel";
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

function ArenaNode({ data }: NodeProps) {
  const node = data as unknown as LayoutNode;
  const select = useDashboardStore((s) => s.select);
  const profiler = useDashboardStore((s) => s.cpuProfiles.get(node.label));
  const hasFaults = node.faults.length > 0;
  const isPulsing = hasFaults || isActiveState(node.state);

  return (
    <div
      style={{
        width: estimateNodeWidth(
          node.label,
          profiler ? profilerLabel(profiler.state, profiler.reportReady) : undefined,
        ),
        ...pulseStyle(isPulsing, hasFaults),
      }}
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
      {profiler !== undefined && (
        <CpuProfileButton componentId={node.label} profiler={profiler} variant="node" />
      )}
    </div>
  );
}

const nodeTypes = { arena: ArenaNode };

const GRAPH_CONTROL_CLASSES =
  "cursor-pointer rounded-md border border-slate-600 bg-[#0e1330] px-2 py-1.5 text-xs uppercase tracking-wide text-slate-300 transition-colors hover:bg-slate-700/60";

function GraphControls() {
  const { fitView, zoomIn, zoomOut } = useReactFlow();

  return (
    <div className="flex items-center gap-1">
      <span
        data-testid="zoom-hint-graph"
        className="mr-1 whitespace-nowrap rounded-sm border border-slate-700 bg-[#0e1330]/80 px-1.5 py-0.5 text-[9px] uppercase tracking-wide text-slate-400"
      >
        Ctrl + scroll to zoom
      </span>
      <button
        type="button"
        data-testid="zoom-out-graph"
        aria-label="Zoom out"
        onClick={() => void zoomOut()}
        className={GRAPH_CONTROL_CLASSES}
      >
        -
      </button>
      <button
        type="button"
        data-testid="zoom-in-graph"
        aria-label="Zoom in"
        onClick={() => void zoomIn()}
        className={GRAPH_CONTROL_CLASSES}
      >
        +
      </button>
      <button
        type="button"
        data-testid="recenter-graph"
        onClick={() => void fitView()}
        className={GRAPH_CONTROL_CLASSES}
      >
        Recenter
      </button>
    </div>
  );
}

function FitViewOnNodeSetChange({ nodeIds }: { nodeIds: string }) {
  const { fitView } = useReactFlow();
  const nodesInitialized = useNodesInitialized();

  useEffect(() => {
    if (!nodesInitialized) return;
    void fitView();
  }, [fitView, nodeIds, nodesInitialized]);

  return null;
}

export function ArenaGraph({ arenaId }: { arenaId: string }) {
  const snapshot = useDashboardStore((s) => s.arenas.get(arenaId)?.snapshot ?? null);
  const select = useDashboardStore((s) => s.select);

  const canvasRef = useRef<HTMLDivElement | null>(null);
  const measured = useRef(new Map<string, { width: number; height: number }>());
  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (canvas === null) return;
    const onWheel = (event: WheelEvent) => {
      if (event.ctrlKey || event.metaKey) event.preventDefault();
    };
    canvas.addEventListener("wheel", onWheel, { passive: false });
    return () => canvas.removeEventListener("wheel", onWheel);
  }, []);

  const applyNodeChanges = (changes: Parameters<typeof onNodesChange>[0]) => {
    changes.forEach((change) => {
      if (change.type === "dimensions" && change.dimensions !== undefined) {
        measured.current.set(change.id, change.dimensions);
      }
    });
    onNodesChange(changes.filter((change) => change.type !== "remove"));
  };
  const applyEdgeChanges = (changes: Parameters<typeof onEdgesChange>[0]) =>
    onEdgesChange(changes.filter((change) => change.type !== "remove"));

  useEffect(() => {
    if (!snapshot) return;
    const layout = computeLayout(snapshot);
    const positions = dagreLayout(layout.nodes, layout.edges);

    setNodes((current) => {
      const existing = new Map(current.map((n) => [n.id, n]));
      return layout.nodes.map((n) => {
        const prev = existing.get(n.id);
        if (prev) {
          return {
            ...prev,
            measured: prev.measured ?? measured.current.get(n.id),
            data: n as unknown as Record<string, unknown>,
          };
        }
        return {
          id: n.id,
          type: "arena",
          position: positions.get(n.id) ?? { x: 0, y: 0 },
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
    <div
      ref={canvasRef}
      data-testid="arena-canvas"
      className="h-[480px] w-full overflow-hidden rounded-md bg-[#0e1330]"
    >
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={applyNodeChanges}
        onEdgesChange={applyEdgeChanges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        fitView
        zoomOnScroll={false}
        preventScrolling={false}
        zoomActivationKeyCode={["Control", "Meta"]}
        proOptions={{ hideAttribution: true }}
      >
        <FitViewOnNodeSetChange nodeIds={nodes.map((n) => n.id).join("|")} />
        <Background
          variant={BackgroundVariant.Lines}
          color={GRID_LINE_COLOR}
          gap={24}
          lineWidth={GRID_LINE_WIDTH}
        />
        <Panel position="top-left" className="flex w-[calc(100%-30px)] items-start justify-between gap-3">
          <div className="min-w-0">
          <button
            type="button"
            data-testid={`arena-state-${arenaId}`}
            title={snapshot ? `${snapshot.id} · state: ${snapshot.state}` : undefined}
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
            className={`block max-w-full cursor-pointer truncate rounded-md border px-3 py-1.5 text-left text-xs uppercase tracking-wide ${
              snapshot ? arenaClasses(snapshot.state) : "border-slate-700 text-slate-500"
            }`}
          >
            {snapshot ? `${snapshot.id} · state: ${snapshot.state}` : "waiting for events..."}
          </button>
          </div>
          <div className="shrink-0">
            <GraphControls />
          </div>
        </Panel>
      </ReactFlow>
    </div>
  );
}
