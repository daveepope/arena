import { useEffect, useMemo, useRef, useState } from "react";

import { ArenaGraph } from "./ArenaGraph";
import { PlaybookPanel } from "./PlaybookPanel";
import { CpuProfileButton, CpuProfilePopover, CpuProfileView } from "./CpuProfilePanel";
import { FloatingPanel } from "./FloatingPanel";
import { lastRecordedSnapshot, useDashboardStore } from "./store";
import type { DetailPanel } from "./store";
import { TERMINAL_ARENA_STATES } from "./presentation";
import type { ArenaState, Fault, PlaybookEntry } from "./types";

type ConnectionStatus = "connecting" | "connected" | "disconnected" | "error";

function isDisconnected(status: ConnectionStatus): boolean {
  return status === "error" || status === "disconnected";
}

const HEARTBEAT_INTERVAL_MS = 500;
const HEARTBEAT_TIMEOUT_MS = HEARTBEAT_INTERVAL_MS * 2;

const EMPTY_LOG: never[] = [];
const SLIDER_TWEEN_MS = 280;

function useSmoothSliderValue(
  target: number,
  durationMs: number,
  tweenEnabled: boolean,
): [number, (value: number) => void] {
  const [display, setDisplay] = useState(target);
  const displayRef = useRef(target);
  const frameRef = useRef<number | null>(null);

  useEffect(() => {
    if (!tweenEnabled) return;
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
  }, [target, durationMs, tweenEnabled]);

  const setImmediate = (value: number) => {
    if (frameRef.current !== null) cancelAnimationFrame(frameRef.current);
    displayRef.current = value;
    setDisplay(value);
  };

  return [display, setImmediate];
}


interface RecordedPlaybook {
  playbook: string;
  state: string;
  kind: string;
  playbook_type: string;
  runs: number;
  at: string;
  faults?: Fault[];
}

function recordedPlaybookEntry(recorded: RecordedPlaybook): PlaybookEntry {
  return {
    at: Date.parse(recorded.at),
    playbookId: recorded.playbook,
    state: recorded.state,
    kind: recorded.kind,
    playbookType: recorded.playbook_type,
    runs: recorded.runs,
    faults: recorded.faults ?? [],
  };
}

function ArenaPanel({ arenaId, isStale }: { arenaId: string; isStale: boolean }) {
  const log = useDashboardStore((s) => s.arenas.get(arenaId)?.log ?? EMPTY_LOG);
  const historyLength = useDashboardStore((s) => s.arenas.get(arenaId)?.history.length ?? 0);
  const latestArenaState = useDashboardStore(
    (s) => lastRecordedSnapshot(s.arenas.get(arenaId)?.history)?.state ?? null,
  );
  const replayIndex = useDashboardStore((s) => s.arenas.get(arenaId)?.replayIndex ?? null);
  const isReplaying = useDashboardStore((s) => s.arenas.get(arenaId)?.isReplaying ?? false);
  const replayArena = useDashboardStore((s) => s.replayArena);
  const stopReplay = useDashboardStore((s) => s.stopReplay);
  const scrubArena = useDashboardStore((s) => s.scrubArena);

  const logRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const pane = logRef.current;
    if (pane) pane.scrollTop = pane.scrollHeight;
  }, [log]);

  const isTerminal = latestArenaState !== null && TERMINAL_ARENA_STATES.has(latestArenaState);
  const showTimeline = isTerminal || isStale;
  const sliderMax = Math.max(historyLength - 1, 0);
  const sliderValue = replayIndex ?? sliderMax;
  const [isScrubbing, setIsScrubbing] = useState(false);
  const [displaySliderValue, setDisplaySliderValueImmediate] = useSmoothSliderValue(
    sliderValue,
    SLIDER_TWEEN_MS,
    !isScrubbing,
  );

  return (
    <div
      data-testid={`arena-panel-${arenaId}`}
      className="w-full rounded-lg border border-slate-800 bg-[#121735] p-3"
    >
      <div className="mb-2 flex items-center justify-between gap-3">
        <h2 className="flex items-center gap-2 text-sm font-semibold text-slate-300">
          <span className="rounded-sm border border-slate-700 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-slate-400">
            arena
          </span>
          {arenaId}
        </h2>
        {showTimeline && (
          <>
            <div className="flex flex-1 items-center gap-2">
              <span className="whitespace-nowrap text-xs uppercase tracking-wide text-slate-500">Timeline</span>
              <input
                type="range"
                min={0}
                max={sliderMax}
                step="any"
                value={displaySliderValue}
                onChange={(e) => {
                  const value = Number(e.target.value);
                  setDisplaySliderValueImmediate(value);
                  scrubArena(arenaId, Math.round(value));
                }}
                onPointerDown={() => setIsScrubbing(true)}
                onPointerUp={() => setIsScrubbing(false)}
                onPointerCancel={() => setIsScrubbing(false)}
                onBlur={() => setIsScrubbing(false)}
                className="timeline-slider flex-1 cursor-pointer"
              />
            </div>
            <button
              type="button"
              onClick={() => {
                if (isReplaying) {
                  stopReplay(arenaId);
                } else {
                  setDisplaySliderValueImmediate(0);
                  replayArena(arenaId);
                }
              }}
              className="cursor-pointer rounded-md border border-slate-600 bg-[#0e1330] px-2 py-1 text-xs uppercase tracking-wide text-slate-300 transition-colors hover:bg-slate-700/60"
            >
              {isReplaying ? "Stop replay" : "Replay"}
            </button>
          </>
        )}
      </div>
      {isStale && (
        <div
          data-testid={`stale-notice-${arenaId}`}
          className="mb-2 rounded-md border border-amber-700/60 bg-amber-950/30 px-2 py-1 text-[11px] text-amber-300"
        >
          disconnected from server, this is the last known state and may not be accurate
        </div>
      )}
      <div className="flex gap-3">
        <PlaybookPanel arenaId={arenaId} />
        <div className="min-w-0 flex-1">
          <ArenaGraph arenaId={arenaId} />
        </div>
      </div>
      <div
        ref={logRef}
        data-testid={`event-log-${arenaId}`}
        className="mt-3 h-32 overflow-y-auto rounded-md border border-slate-800 bg-[#0e1330] p-2 font-mono text-xs text-slate-400"
      >
        {log.map((entry) => (
          <div key={entry.id} className={entry.isFault ? "text-rose-400" : undefined}>
            {entry.text}
          </div>
        ))}
      </div>
    </div>
  );
}

interface LegendEntry {
  label: string;
  dotClass: string;
  shapeClass: string;
}

const STATE_LEGEND: LegendEntry[] = [
  { label: "not_started / registered", dotClass: "bg-slate-500", shapeClass: "rounded-full" },
  { label: "starting / readiness_check", dotClass: "bg-amber-400", shapeClass: "rounded-full" },
  { label: "started / running", dotClass: "bg-emerald-400", shapeClass: "rounded-full" },
  { label: "stopping", dotClass: "bg-orange-400", shapeClass: "rounded-full" },
  { label: "stopped", dotClass: "bg-slate-700", shapeClass: "rounded-full" },
  { label: "faulted", dotClass: "bg-rose-500", shapeClass: "rounded-full" },
];

const SUBJECT_LEGEND: LegendEntry[] = [
  { label: "dependency", dotClass: "bg-sky-400", shapeClass: "rounded-sm" },
  { label: "component", dotClass: "bg-fuchsia-400", shapeClass: "rounded-full" },
];

function LegendItem({ label, dotClass, shapeClass }: LegendEntry) {
  return (
    <span className="flex items-center gap-1 text-xs text-slate-400">
      <span className={`h-2.5 w-2.5 ${shapeClass} ${dotClass}`} />
      {label}
    </span>
  );
}

function LegendGroup({ title, entries }: { title: string; entries: LegendEntry[] }) {
  return (
    <div className="flex items-center gap-2 rounded-md border border-slate-800 px-2 py-1">
      <span className="text-[10px] uppercase tracking-wide text-slate-500">{title}</span>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
        {entries.map((item) => (
          <LegendItem key={item.label} {...item} />
        ))}
      </div>
    </div>
  );
}

function Legend() {
  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-2" data-testid="legend">
      <LegendGroup title="state" entries={STATE_LEGEND} />
      <LegendGroup title="subject" entries={SUBJECT_LEGEND} />
    </div>
  );
}

function ServerStatus({ status }: { status: ConnectionStatus }) {
  return (
    <div
      data-testid="server-status"
      className="flex items-center gap-2 rounded-md border border-slate-800 bg-[#121735] px-2 py-1"
    >
      <span className="text-[10px] uppercase tracking-wide text-slate-500">server</span>
      <span
        className={`h-2 w-2 rounded-full ${
          status === "connected"
            ? "bg-emerald-400"
            : isDisconnected(status)
              ? "bg-rose-500"
              : "bg-amber-400"
        }`}
        title={`SSE: ${status}`}
      />
      <span className="text-xs text-slate-300">{status}</span>
    </div>
  );
}

const DETAIL_PANEL_WIDTH = 320;
const DETAIL_PANEL_MAX_HEIGHT = 360;

function SelectedDetailPopover({
  panel,
  onTogglePin,
  onClose,
}: {
  panel: DetailPanel;
  onTogglePin: () => void;
  onClose: () => void;
}) {
  const profiler = useDashboardStore((s) => s.cpuProfiles.get(panel.title));

  return (
    <FloatingPanel
      testId="detail-panel"
      handleId="detail"
      title={panel.title}
      position={panel.position}
      size={{ width: DETAIL_PANEL_WIDTH, height: DETAIL_PANEL_MAX_HEIGHT }}
      minSize={{ width: 240, height: 160 }}
      isPinned={panel.isPinned}
      onTogglePin={onTogglePin}
      onClose={onClose}
    >
      <pre className="whitespace-pre-wrap text-xs text-slate-400">{panel.detail}</pre>
      {profiler !== undefined && (
        <div className="mt-3">
          <CpuProfileButton componentId={panel.title} profiler={profiler} variant="detail" />
        </div>
      )}
    </FloatingPanel>
  );
}

function StopServerConfirmation({
  onConfirm,
  onCancel,
}: {
  onConfirm: () => void;
  onCancel: () => void;
}) {
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onCancel]);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      onClick={onCancel}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="stop-server-title"
        className="w-full max-w-sm rounded-md border border-slate-700 bg-[#121735] p-4 text-slate-200 shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 id="stop-server-title" className="text-sm font-semibold">
          Stop the dashboard server?
        </h3>
        <p className="mt-2 text-xs text-slate-400">
          Are you sure you want to stop the server? Live arena events will stop arriving until it is
          started again.
        </p>
        <div className="mt-4 flex justify-end gap-2">
          <button
            type="button"
            onClick={onCancel}
            className="cursor-pointer rounded-md border border-slate-600 bg-[#0e1330] px-3 py-1.5 text-xs font-medium uppercase tracking-wide text-slate-300 transition-colors hover:bg-slate-700/60"
          >
            Cancel
          </button>
          <button
            type="button"
            autoFocus
            onClick={onConfirm}
            className="cursor-pointer rounded-md border border-rose-800 bg-rose-950/40 px-3 py-1.5 text-xs font-medium uppercase tracking-wide text-rose-300 transition-colors hover:bg-rose-900/40"
          >
            Stop server
          </button>
        </div>
      </div>
    </div>
  );
}

export function fullScreenProfileComponentId(search: string): string | null {
  const value = new URLSearchParams(search).get("profile");
  return value !== null && value !== "" ? value : null;
}

export function App() {
  const ingest = useDashboardStore((s) => s.ingest);
  const ingestCpuProfile = useDashboardStore((s) => s.ingestCpuProfile);
  const ingestPlaybook = useDashboardStore((s) => s.ingestPlaybook);
  const ingestHistory = useDashboardStore((s) => s.ingestHistory);
  const arenas = useDashboardStore((s) => s.arenas);
  const arenaIds = useMemo(() => Array.from(arenas.keys()), [arenas]);
  const details = useDashboardStore((s) => s.details);
  const clearSelected = useDashboardStore((s) => s.clearSelected);
  const closeDetail = useDashboardStore((s) => s.closeDetail);
  const toggleDetailPin = useDashboardStore((s) => s.toggleDetailPin);
  const clearArenas = useDashboardStore((s) => s.clearArenas);
  const [status, setStatus] = useState<ConnectionStatus>("connecting");
  const lastHeartbeatAtRef = useRef<number | null>(null);

  useEffect(() => {
    let abandoned = false;
    fetch("/history")
      .then((response) => (response.ok ? response.json() : []))
      .then((arenas: { id: string; states: ArenaState[]; playbooks?: RecordedPlaybook[] }[]) => {
        if (abandoned || !Array.isArray(arenas)) return;
        arenas.forEach((arena) =>
          ingestHistory(arena.states, (arena.playbooks ?? []).map(recordedPlaybookEntry)),
        );
      })
      .catch(() => {});
    fetch("/profiles")
      .then((response) => (response.ok ? response.json() : []))
      .then((held: { component: string; state: string; report: boolean }[]) => {
        if (abandoned || !Array.isArray(held)) return;
        held.forEach((entry) => ingestCpuProfile(entry.component, entry.state, entry.report));
      })
      .catch(() => {});
    return () => {
      abandoned = true;
    };
  }, [ingestHistory, ingestCpuProfile]);

  useEffect(() => {
    const source = new EventSource("/events");
    const markAlive = () => {
      lastHeartbeatAtRef.current = Date.now();
      setStatus("connected");
    };
    source.onopen = markAlive;
    source.addEventListener("heartbeat", markAlive);
    source.onerror = () => {
      setStatus((prev) => {
        if (prev === "disconnected") return prev;
        return source.readyState === EventSource.CONNECTING ? "connecting" : "error";
      });
    };
    source.onmessage = (evt) => {
      lastHeartbeatAtRef.current = Date.now();
      try {
        const data = JSON.parse(evt.data);
        if (data && data.type === "clear") {
          clearArenas();
          return;
        }
        if (data && data.type === "cpu_profile") {
          ingestCpuProfile(data.component, data.state, data.report === true);
          return;
        }
        if (data && data.type === "playbook") {
          ingestPlaybook(data.arena, {
            at: Date.parse(data.at),
            playbookId: data.playbook,
            state: data.state,
            kind: data.kind,
            playbookType: data.playbook_type,
            runs: data.runs,
            faults: Array.isArray(data.faults) ? data.faults : [],
          });
          return;
        }
        if (data && typeof data.type === "string") {
          return;
        }
        ingest(data as ArenaState);
      } catch {}
    };

    const heartbeatCheck = setInterval(() => {
      const lastHeartbeatAt = lastHeartbeatAtRef.current;
      if (lastHeartbeatAt !== null && Date.now() - lastHeartbeatAt > HEARTBEAT_TIMEOUT_MS) {
        setStatus("disconnected");
      }
    }, HEARTBEAT_INTERVAL_MS);

    return () => {
      clearInterval(heartbeatCheck);
      source.close();
    };
  }, [ingest, clearArenas, ingestCpuProfile, ingestPlaybook]);

  const fullScreenProfile = useMemo(() => fullScreenProfileComponentId(window.location.search), []);
  const [actionFeedback, setActionFeedback] = useState<{ text: string; isError: boolean } | null>(null);
  const [isStopConfirmOpen, setIsStopConfirmOpen] = useState(false);

  const showActionFeedback = (text: string, isError: boolean) => {
    setActionFeedback({ text, isError });
    setTimeout(() => setActionFeedback(null), 2500);
  };

  const handleClearServerState = async () => {
    setActionFeedback({ text: "Clearing...", isError: false });
    try {
      const response = await fetch("/clear", { method: "POST" });
      showActionFeedback(response.ok ? "Cleared" : "Failed to clear", !response.ok);
    } catch {
      showActionFeedback("Failed to clear", true);
    }
  };

  const handleStopServer = async () => {
    setIsStopConfirmOpen(false);
    setActionFeedback({ text: "Stopping...", isError: false });
    try {
      const response = await fetch("/shutdown", { method: "POST" });
      showActionFeedback(response.ok ? "Server stopped" : "Failed to stop server", !response.ok);
    } catch {
      showActionFeedback("Server stopped", false);
    }
  };

  if (fullScreenProfile !== null) {
    return (
      <div className="flex h-screen w-full flex-col gap-2 bg-[#0e1330] p-3 text-slate-200">
        <h1 className="text-base font-semibold">cpu profile · {fullScreenProfile}</h1>
        <CpuProfileView componentId={fullScreenProfile} isFullScreen />
      </div>
    );
  }

  return (
    <div className="w-full bg-[#0e1330] text-slate-200">
      <div
        data-testid="dashboard-header"
        className="sticky top-0 z-40 flex flex-col gap-2 border-b border-slate-800 bg-[#0e1330] px-3 py-3"
      >
        <div className="flex items-start gap-4">
          <div className="flex min-w-0 flex-wrap items-center gap-x-4 gap-y-2">
            <h1 className="text-base font-semibold text-slate-200">Arena Topology Dashboard</h1>
            <ServerStatus status={status} />
          </div>
          <div data-testid="server-actions" className="ml-auto flex shrink-0 items-center gap-2">
        {actionFeedback && (
          <span className={`text-xs ${actionFeedback.isError ? "text-rose-400" : "text-teal-300"}`}>
            {actionFeedback.text}
          </span>
        )}
        {status === "connected" && (
          <>
            <button
              type="button"
              onClick={handleClearServerState}
              className="cursor-pointer rounded-md border border-slate-600 bg-[#121735] px-3 py-1.5 text-xs font-medium uppercase tracking-wide text-slate-300 transition-colors hover:bg-slate-700/60"
            >
              Clear server state
            </button>
            <button
              type="button"
              onClick={() => setIsStopConfirmOpen(true)}
              className="cursor-pointer rounded-md border border-rose-800 bg-rose-950/40 px-3 py-1.5 text-xs font-medium uppercase tracking-wide text-rose-300 transition-colors hover:bg-rose-900/40"
            >
              Stop server
            </button>
          </>
        )}
          </div>
        </div>
        <Legend />
      </div>

      <div className="px-3 pb-4 pt-4">
        {arenaIds.length === 0 && (
          <div
            data-testid={isDisconnected(status) ? "disconnected-notice" : "empty-notice"}
            className="mb-4 rounded-lg border border-dashed border-slate-800 p-10 text-center"
          >
            {isDisconnected(status) ? (
              <div className="text-sm font-medium text-slate-300">disconnected</div>
            ) : (
              <>
                <div className="text-sm font-medium text-slate-300">
                  {status === "connected" ? "Connected - no arenas yet" : "Connecting..."}
                </div>
                <div className="mt-1 text-xs text-slate-400">
                  Arenas will appear here once one starts
                </div>
              </>
            )}
          </div>
        )}

        <div className="flex w-full flex-col gap-4">
          {arenaIds.map((id) => (
            <ArenaPanel key={id} arenaId={id} isStale={isDisconnected(status)} />
          ))}
        </div>
      </div>

      {isStopConfirmOpen && (
        <StopServerConfirmation
          onConfirm={handleStopServer}
          onCancel={() => setIsStopConfirmOpen(false)}
        />
      )}

      {details.map((panel) => (
        <SelectedDetailPopover
          key={panel.key}
          panel={panel}
          onTogglePin={() => toggleDetailPin(panel.key)}
          onClose={() => (panel.isPinned ? closeDetail(panel.key) : clearSelected())}
        />
      ))}
      <CpuProfilePopover />
    </div>
  );
}
