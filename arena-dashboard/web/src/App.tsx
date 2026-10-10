import { useEffect, useMemo, useState } from "react";

import { ArenaGraph } from "./ArenaGraph";
import { useDashboardStore } from "./store";
import type { ArenaState } from "./types";

type ConnectionStatus = "connecting" | "connected" | "error";

const EMPTY_LOG: never[] = [];

function ArenaPanel({ arenaId }: { arenaId: string }) {
  const log = useDashboardStore((s) => s.arenas.get(arenaId)?.log ?? EMPTY_LOG);

  return (
    <div className="rounded-lg border border-neutral-800 bg-neutral-900/40 p-3">
      <h2 className="mb-2 text-sm font-semibold text-neutral-300">{arenaId}</h2>
      <ArenaGraph arenaId={arenaId} />
      <div className="mt-3 h-32 overflow-y-auto rounded-md border border-neutral-800 bg-neutral-900 p-2 font-mono text-xs text-neutral-400">
        {log.map((entry) => (
          <div key={entry.id} className={entry.isFault ? "text-red-400" : undefined}>
            {entry.text}
          </div>
        ))}
      </div>
    </div>
  );
}

export function App() {
  const ingest = useDashboardStore((s) => s.ingest);
  const arenas = useDashboardStore((s) => s.arenas);
  const arenaIds = useMemo(() => Array.from(arenas.keys()), [arenas]);
  const selected = useDashboardStore((s) => s.selected);
  const clearSelected = useDashboardStore((s) => s.clearSelected);
  const [status, setStatus] = useState<ConnectionStatus>("connecting");

  useEffect(() => {
    const source = new EventSource("/events");
    source.onopen = () => setStatus("connected");
    source.onerror = () => setStatus(source.readyState === EventSource.CONNECTING ? "connecting" : "error");
    source.onmessage = (evt) => {
      try {
        const data = JSON.parse(evt.data) as ArenaState;
        ingest(data);
      } catch {}
    };
    return () => source.close();
  }, [ingest]);

  return (
    <div className="w-full px-6 py-4 text-neutral-200">
      <div className="mb-3 flex items-center gap-2">
        <h1 className="text-base font-semibold text-neutral-200">Arena Dashboard</h1>
        <span
          className={`h-2 w-2 rounded-full ${
            status === "connected" ? "bg-emerald-500" : status === "error" ? "bg-red-500" : "bg-amber-400"
          }`}
          title={`SSE: ${status}`}
        />
        <span className="text-xs text-neutral-500">{status}</span>
      </div>

      {status === "error" && (
        <div className="mb-4 rounded-md border border-red-900 bg-red-950/40 p-4 text-sm text-red-300">
          Lost connection to the dashboard server's /events stream. Is the server still running?
        </div>
      )}

      {status !== "error" && arenaIds.length === 0 && (
        <div className="mb-4 rounded-lg border border-dashed border-neutral-800 p-10 text-center">
          <div className="text-sm font-medium text-neutral-300">Connected - no arenas yet</div>
          <div className="mt-1 text-xs text-neutral-500">Arenas will appear here once one starts</div>
        </div>
      )}

      <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
        {arenaIds.map((id) => (
          <ArenaPanel key={id} arenaId={id} />
        ))}
      </div>

      {selected && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/60" onClick={clearSelected}>
          <div
            className="max-w-md rounded-md border border-neutral-700 bg-neutral-900 p-4 text-neutral-200"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="mb-2 text-sm font-semibold">{selected.title}</h3>
            <pre className="whitespace-pre-wrap text-xs text-neutral-400">{selected.detail}</pre>
            <button
              type="button"
              onClick={clearSelected}
              className="mt-3 rounded-md border border-neutral-700 px-3 py-1 text-xs"
            >
              Close
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
