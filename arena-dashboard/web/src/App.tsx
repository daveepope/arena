import { useEffect, useMemo, useRef, useState } from "react";

import { ArenaGraph } from "./ArenaGraph";
import { useDashboardStore } from "./store";
import type { ArenaState } from "./types";

type ConnectionStatus = "connecting" | "connected" | "error";

const EMPTY_LOG: never[] = [];
const SLIDER_TWEEN_MS = 280;

function useSmoothSliderValue(target: number, durationMs: number): [number, (value: number) => void] {
  const [display, setDisplay] = useState(target);
  const displayRef = useRef(target);
  const frameRef = useRef<number | null>(null);

  useEffect(() => {
    if (frameRef.current !== null) cancelAnimationFrame(frameRef.current);
    const start = displayRef.current;
    const delta = target - start;
    if (Math.abs(delta) < 0.001) {
      displayRef.current = target;
      setDisplay(target);
      return;
    }
    const startTime = performance.now();
    const step = (now: number) => {
      const t = Math.min(1, (now - startTime) / durationMs);
      const eased = 1 - Math.pow(1 - t, 3);
      const value = start + delta * eased;
      displayRef.current = value;
      setDisplay(value);
      frameRef.current = t < 1 ? requestAnimationFrame(step) : null;
    };
    frameRef.current = requestAnimationFrame(step);
    return () => {
      if (frameRef.current !== null) cancelAnimationFrame(frameRef.current);
    };
  }, [target, durationMs]);

  const setImmediate = (value: number) => {
    if (frameRef.current !== null) cancelAnimationFrame(frameRef.current);
    displayRef.current = value;
    setDisplay(value);
  };

  return [display, setImmediate];
}

const TERMINAL_ARENA_STATES = new Set(["arena_closed", "arena_faulted"]);

function ArenaPanel({ arenaId }: { arenaId: string }) {
  const log = useDashboardStore((s) => s.arenas.get(arenaId)?.log ?? EMPTY_LOG);
  const historyLength = useDashboardStore((s) => s.arenas.get(arenaId)?.history.length ?? 0);
  const latestArenaState = useDashboardStore((s) => {
    const history = s.arenas.get(arenaId)?.history;
    return history && history.length > 0 ? history[history.length - 1].snapshot.state : null;
  });
  const replayIndex = useDashboardStore((s) => s.arenas.get(arenaId)?.replayIndex ?? null);
  const isReplaying = useDashboardStore((s) => s.arenas.get(arenaId)?.isReplaying ?? false);
  const replayArena = useDashboardStore((s) => s.replayArena);
  const stopReplay = useDashboardStore((s) => s.stopReplay);
  const scrubArena = useDashboardStore((s) => s.scrubArena);

  const isTerminal = latestArenaState !== null && TERMINAL_ARENA_STATES.has(latestArenaState);
  const sliderMax = Math.max(historyLength - 1, 0);
  const sliderValue = replayIndex ?? sliderMax;
  const [displaySliderValue, setDisplaySliderValueImmediate] = useSmoothSliderValue(sliderValue, SLIDER_TWEEN_MS);

  return (
    <div className="w-full rounded-lg border border-slate-800 bg-[#121735] p-3">
      <div className="mb-2 flex items-center justify-between gap-3">
        <h2 className="text-sm font-semibold text-slate-300">{arenaId}</h2>
        {isTerminal && (
          <div className="flex flex-1 items-center gap-2">
            <span className="whitespace-nowrap text-xs uppercase tracking-wide text-slate-500">Timeline</span>
            <input
              type="range"
              min={0}
              max={sliderMax}
              value={displaySliderValue}
              onChange={(e) => {
                const value = Number(e.target.value);
                setDisplaySliderValueImmediate(value);
                scrubArena(arenaId, Math.round(value));
              }}
              className="h-1 flex-1 cursor-pointer accent-sky-400"
            />
          </div>
        )}
        <button
          type="button"
          disabled={historyLength === 0}
          onClick={() => {
            if (isReplaying) {
              stopReplay(arenaId);
            } else {
              setDisplaySliderValueImmediate(0);
              replayArena(arenaId);
            }
          }}
          className="cursor-pointer rounded-md border border-slate-600 bg-[#0e1330] px-2 py-1 text-xs uppercase tracking-wide text-slate-300 transition-colors hover:bg-slate-700/60 disabled:cursor-not-allowed disabled:opacity-40"
        >
          {isReplaying ? "Stop replay" : "Replay"}
        </button>
      </div>
      <ArenaGraph arenaId={arenaId} />
      <div className="mt-3 h-32 overflow-y-auto rounded-md border border-slate-800 bg-[#0e1330] p-2 font-mono text-xs text-slate-400">
        {log.map((entry) => (
          <div key={entry.id} className={entry.isFault ? "text-rose-400" : undefined}>
            {entry.text}
          </div>
        ))}
      </div>
    </div>
  );
}

const STATE_LEGEND: { label: string; dotClass: string }[] = [
  { label: "not started / stopped", dotClass: "bg-neutral-500" },
  { label: "starting", dotClass: "bg-amber-400" },
  { label: "started", dotClass: "bg-emerald-400" },
  { label: "stopping", dotClass: "bg-orange-400" },
  { label: "faulted", dotClass: "bg-red-500" },
];

const KIND_LEGEND: { label: string; dotClass: string }[] = [
  { label: "dependency", dotClass: "bg-sky-400" },
  { label: "component", dotClass: "bg-fuchsia-400" },
];

function LegendItem({ label, dotClass }: { label: string; dotClass: string }) {
  return (
    <span className="flex items-center gap-1 text-xs text-slate-400">
      <span className={`h-2.5 w-2.5 rounded-full ${dotClass}`} />
      {label}
    </span>
  );
}

function Legend() {
  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
      {STATE_LEGEND.map((item) => (
        <LegendItem key={item.label} {...item} />
      ))}
      <span className="h-3 w-px bg-slate-700" />
      {KIND_LEGEND.map((item) => (
        <LegendItem key={item.label} {...item} />
      ))}
    </div>
  );
}

const DETAIL_PANEL_WIDTH = 320;
const DETAIL_PANEL_MAX_HEIGHT = 360;
const VIEWPORT_MARGIN = 8;

function clampPosition(x: number, y: number): { left: number; top: number } {
  const maxLeft = window.innerWidth - DETAIL_PANEL_WIDTH - VIEWPORT_MARGIN;
  const maxTop = window.innerHeight - DETAIL_PANEL_MAX_HEIGHT - VIEWPORT_MARGIN;
  return {
    left: Math.min(Math.max(x, VIEWPORT_MARGIN), Math.max(maxLeft, VIEWPORT_MARGIN)),
    top: Math.min(Math.max(y, VIEWPORT_MARGIN), Math.max(maxTop, VIEWPORT_MARGIN)),
  };
}

function SelectedDetailPopover({
  selected,
  onClose,
}: {
  selected: { title: string; detail: string; position: { x: number; y: number } };
  onClose: () => void;
}) {
  const { left, top } = clampPosition(selected.position.x, selected.position.y);

  return (
    <>
      <div className="fixed inset-0" onClick={onClose} />
      <div
        style={{ left, top, width: DETAIL_PANEL_WIDTH, maxHeight: DETAIL_PANEL_MAX_HEIGHT }}
        className="fixed overflow-y-auto rounded-md border border-slate-700 bg-[#121735] p-4 text-slate-200 shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="mb-2 text-sm font-semibold">{selected.title}</h3>
        <pre className="whitespace-pre-wrap text-xs text-slate-400">{selected.detail}</pre>
        <button
          type="button"
          onClick={onClose}
          className="mt-3 cursor-pointer rounded-md border border-slate-700 px-3 py-1 text-xs"
        >
          Close
        </button>
      </div>
    </>
  );
}

export function App() {
  const ingest = useDashboardStore((s) => s.ingest);
  const arenas = useDashboardStore((s) => s.arenas);
  const arenaIds = useMemo(() => Array.from(arenas.keys()), [arenas]);
  const selected = useDashboardStore((s) => s.selected);
  const clearSelected = useDashboardStore((s) => s.clearSelected);
  const clearArenas = useDashboardStore((s) => s.clearArenas);
  const [status, setStatus] = useState<ConnectionStatus>("connecting");

  useEffect(() => {
    const source = new EventSource("/events");
    source.onopen = () => setStatus("connected");
    source.onerror = () => setStatus(source.readyState === EventSource.CONNECTING ? "connecting" : "error");
    source.onmessage = (evt) => {
      try {
        const data = JSON.parse(evt.data);
        if (data && data.type === "clear") {
          clearArenas();
          return;
        }
        ingest(data as ArenaState);
      } catch {}
    };
    return () => source.close();
  }, [ingest, clearArenas]);

  const handleClearServerState = () => {
    void fetch("/clear", { method: "POST" });
  };

  return (
    <div className="w-full bg-[#0e1330] text-slate-200">
      <div className="fixed inset-x-0 top-0 z-40 flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-slate-800 bg-[#0e1330] px-3 py-3 pr-40">
        <div className="flex items-center gap-2">
          <h1 className="text-base font-semibold text-slate-200">Arena Topology Dashboard</h1>
          <span
            className={`h-2 w-2 rounded-full ${
              status === "connected" ? "bg-teal-400" : status === "error" ? "bg-rose-500" : "bg-amber-400"
            }`}
            title={`SSE: ${status}`}
          />
          <span className="text-xs text-slate-500">{status}</span>
        </div>
        <Legend />
      </div>
      <button
        type="button"
        onClick={handleClearServerState}
        className="fixed right-4 top-3 z-50 cursor-pointer rounded-md border border-slate-600 bg-[#121735] px-3 py-1.5 text-xs font-medium uppercase tracking-wide text-slate-300 transition-colors hover:bg-slate-700/60"
      >
        Clear server state
      </button>

      <div className="px-3 pb-4 pt-24">
        {status === "error" && (
          <div className="mb-4 rounded-md border border-rose-900 bg-rose-950/40 p-4 text-sm text-rose-300">
            Lost connection to the dashboard server's /events stream. Is the server still running?
          </div>
        )}

        {status !== "error" && arenaIds.length === 0 && (
          <div className="mb-4 rounded-lg border border-dashed border-slate-800 p-10 text-center">
            <div className="text-sm font-medium text-slate-300">
              {status === "connected" ? "Connected - no arenas yet" : "Connecting..."}
            </div>
            <div className="mt-1 text-xs text-slate-500">Arenas will appear here once one starts</div>
          </div>
        )}

        <div className="flex w-full flex-col gap-4">
          {arenaIds.map((id) => (
            <ArenaPanel key={id} arenaId={id} />
          ))}
        </div>
      </div>

      {selected && <SelectedDetailPopover selected={selected} onClose={clearSelected} />}
    </div>
  );
}
