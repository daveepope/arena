import { useEffect, useRef, useState } from "react";

import {
  dimUnmatchedFrames,
  flamegraphDocument,
  formatSelfPct,
  highlightFrames,
  severityClasses,
  visibleHotspots,
} from "./cpuProfile";
import { useDashboardStore } from "./store";
import type { RenderedProfile } from "./types";

const PANEL_WIDTH = 760;
const PANEL_MAX_HEIGHT = 560;
const PANEL_MIN_WIDTH = 360;
const PANEL_MIN_HEIGHT = 260;
const VIEWPORT_MARGIN = 8;
const FLAMEGRAPH_MIN_HEIGHT = 240;

type LoadState = "loading" | "ready" | "unavailable";
type Layout = "stacked" | "side-by-side";

function clampPanelPosition(x: number, y: number): { left: number; top: number } {
  const maxLeft = window.innerWidth - PANEL_WIDTH - VIEWPORT_MARGIN;
  const maxTop = window.innerHeight - PANEL_MAX_HEIGHT - VIEWPORT_MARGIN;
  return {
    left: Math.min(Math.max(x, VIEWPORT_MARGIN), Math.max(maxLeft, VIEWPORT_MARGIN)),
    top: Math.min(Math.max(y, VIEWPORT_MARGIN), Math.max(maxTop, VIEWPORT_MARGIN)),
  };
}

function Flamegraph({
  svg,
  filter,
  hoveredFunction,
  minHeight,
}: {
  svg: string;
  filter: string;
  hoveredFunction: string | null;
  minHeight: number;
}) {
  const frameRef = useRef<HTMLIFrameElement>(null);
  const [isFrameReady, setIsFrameReady] = useState(false);

  useEffect(() => {
    setIsFrameReady(false);
  }, [svg]);

  useEffect(() => {
    const doc = frameRef.current?.contentDocument;
    if (!doc || !isFrameReady) return;
    dimUnmatchedFrames(doc, filter);
  }, [filter, isFrameReady, svg]);

  useEffect(() => {
    const doc = frameRef.current?.contentDocument;
    if (!doc || !isFrameReady) return;
    highlightFrames(doc, hoveredFunction);
  }, [hoveredFunction, isFrameReady, svg]);

  return (
    <iframe
      ref={frameRef}
      title="cpu flamegraph"
      data-testid="cpu-flamegraph"
      srcDoc={flamegraphDocument(svg)}
      onLoad={() => setIsFrameReady(true)}
      style={{ minHeight }}
      className="w-full flex-1 rounded-md border border-slate-800 bg-[#1e1e1e]"
    />
  );
}

function HotspotTable({
  profile,
  filter,
  onHover,
}: {
  profile: RenderedProfile;
  filter: string;
  onHover: (func: string | null) => void;
}) {
  const rows = visibleHotspots(profile.hotspots, filter);

  if (rows.length === 0) {
    return (
      <div data-testid="cpu-hotspots-empty" className="p-3 text-xs text-slate-500">
        No functions match the filter
      </div>
    );
  }

  return (
    <table data-testid="cpu-hotspots" className="w-full text-left text-xs">
      <thead className="text-slate-500">
        <tr>
          <th className="px-2 py-1 font-medium">Function</th>
          <th className="px-2 py-1 font-medium">Self</th>
          <th className="px-2 py-1 font-medium">Share</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((hotspot) => (
          <tr
            key={hotspot.function}
            data-testid={`cpu-hotspot-${hotspot.function}`}
            onMouseEnter={() => onHover(hotspot.function)}
            onMouseLeave={() => onHover(null)}
            className="cursor-default border-t border-slate-800 hover:bg-slate-800/40"
          >
            <td className="px-2 py-1 font-mono text-slate-300">{hotspot.function}</td>
            <td className="px-2 py-1 text-slate-400">{hotspot.self_count}</td>
            <td className="px-2 py-1">
              <span
                className={`rounded-sm border px-1.5 py-0.5 ${severityClasses(hotspot.severity)}`}
              >
                {formatSelfPct(hotspot.self_pct)}
              </span>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

export function CpuProfileView({
  componentId,
  isFullScreen,
}: {
  componentId: string;
  isFullScreen: boolean;
}) {
  const [profile, setProfile] = useState<RenderedProfile | null>(null);
  const [loadState, setLoadState] = useState<LoadState>("loading");
  const [filter, setFilter] = useState("");
  const [hoveredFunction, setHoveredFunction] = useState<string | null>(null);
  const [layout, setLayout] = useState<Layout>("stacked");

  useEffect(() => {
    let abandoned = false;
    setLoadState("loading");
    fetch(`/profile/cpu/${encodeURIComponent(componentId)}`)
      .then((response) => (response.ok ? response.json() : null))
      .then((loaded: RenderedProfile | null) => {
        if (abandoned) return;
        setProfile(loaded);
        setLoadState(loaded ? "ready" : "unavailable");
      })
      .catch(() => {
        if (!abandoned) setLoadState("unavailable");
      });
    return () => {
      abandoned = true;
    };
  }, [componentId]);

  if (loadState === "loading") {
    return (
      <div data-testid="cpu-profile-loading" className="p-3 text-xs text-slate-400">
        Loading profile...
      </div>
    );
  }

  if (loadState === "unavailable" || !profile) {
    return (
      <div data-testid="cpu-profile-unavailable" className="p-3 text-xs text-rose-300">
        No cpu profile is available for {componentId}
      </div>
    );
  }

  const isSideBySide = layout === "side-by-side";

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2">
      <div className="flex flex-wrap items-center gap-2">
        <div className="flex flex-1 items-center gap-1">
          <input
            type="text"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filter functions (regex)"
            aria-label="Filter functions"
            className="min-w-0 flex-1 rounded-md border border-slate-700 bg-[#0e1330] px-2 py-1 text-xs text-slate-200 placeholder:text-slate-500"
          />
          {filter !== "" && (
            <button
              type="button"
              onClick={() => setFilter("")}
              className="cursor-pointer rounded-md border border-slate-700 px-2 py-1 text-xs text-slate-300"
            >
              Clear
            </button>
          )}
        </div>
        <button
          type="button"
          onClick={() => setLayout(isSideBySide ? "stacked" : "side-by-side")}
          className="cursor-pointer rounded-md border border-slate-600 bg-[#0e1330] px-2 py-1 text-xs uppercase tracking-wide text-slate-300"
        >
          {isSideBySide ? "Stacked" : "Side by side"}
        </button>
        {!isFullScreen && (
          <a
            href={`/?profile=${encodeURIComponent(componentId)}`}
            target="_blank"
            rel="noreferrer"
            className="cursor-pointer rounded-md border border-slate-600 bg-[#0e1330] px-2 py-1 text-xs uppercase tracking-wide text-slate-300"
          >
            Open in new tab
          </a>
        )}
      </div>

      <div className={`flex min-h-0 flex-1 gap-2 ${isSideBySide ? "flex-row" : "flex-col"}`}>
        <Flamegraph
          svg={profile.svg}
          filter={filter}
          hoveredFunction={hoveredFunction}
          minHeight={FLAMEGRAPH_MIN_HEIGHT}
        />
        <div
          className={`min-h-0 overflow-auto rounded-md border border-slate-800 bg-[#0e1330] ${
            isSideBySide ? "w-80 shrink-0" : "max-h-48"
          }`}
        >
          <HotspotTable profile={profile} filter={filter} onHover={setHoveredFunction} />
        </div>
      </div>
    </div>
  );
}

const SNAP_THRESHOLD_PX = 48;

function PinIcon({ isPinned }: { isPinned: boolean }) {
  return (
    <svg
      viewBox="0 0 16 16"
      aria-hidden="true"
      className="h-3.5 w-3.5"
      fill={isPinned ? "currentColor" : "none"}
      stroke="currentColor"
      strokeWidth="1.3"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M9.5 1.5 14.5 6.5 12 7l-4 4-1.5-1.5 4-4 .5-2.5Z" />
      <path d="M6.5 9.5 2 14" />
    </svg>
  );
}

type ResizeEdge = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";

const RESIZE_EDGES: { edge: ResizeEdge; className: string }[] = [
  { edge: "n", className: "left-2 right-2 top-0 h-1.5 cursor-ns-resize" },
  { edge: "s", className: "bottom-0 left-2 right-2 h-1.5 cursor-ns-resize" },
  { edge: "w", className: "bottom-2 left-0 top-2 w-1.5 cursor-ew-resize" },
  { edge: "e", className: "bottom-2 right-0 top-2 w-1.5 cursor-ew-resize" },
  { edge: "nw", className: "left-0 top-0 h-3 w-3 cursor-nwse-resize" },
  { edge: "ne", className: "right-0 top-0 h-3 w-3 cursor-nesw-resize" },
  { edge: "sw", className: "bottom-0 left-0 h-3 w-3 cursor-nesw-resize" },
  { edge: "se", className: "bottom-0 right-0 h-3 w-3 cursor-nwse-resize" },
];

export function snapToViewportCorner(
  rect: { left: number; top: number; width: number; height: number },
  viewport: { width: number; height: number },
): { left: number; top: number } {
  const right = viewport.width - VIEWPORT_MARGIN - rect.width;
  const bottom = viewport.height - VIEWPORT_MARGIN - rect.height;
  const left =
    rect.left <= VIEWPORT_MARGIN + SNAP_THRESHOLD_PX
      ? VIEWPORT_MARGIN
      : rect.left >= right - SNAP_THRESHOLD_PX
        ? Math.max(right, VIEWPORT_MARGIN)
        : rect.left;
  const top =
    rect.top <= VIEWPORT_MARGIN + SNAP_THRESHOLD_PX
      ? VIEWPORT_MARGIN
      : rect.top >= bottom - SNAP_THRESHOLD_PX
        ? Math.max(bottom, VIEWPORT_MARGIN)
        : rect.top;
  return { left, top };
}

export function CpuProfilePopover() {
  const profileView = useDashboardStore((s) => s.profileView);
  const closeProfileView = useDashboardStore((s) => s.closeProfileView);
  const toggleProfileViewPin = useDashboardStore((s) => s.toggleProfileViewPin);
  const [rect, setRect] = useState<{ left: number; top: number; width: number; height: number } | null>(
    null,
  );

  useEffect(() => {
    if (!profileView) return;
    const { left, top } = clampPanelPosition(profileView.position.x, profileView.position.y);
    setRect({ left, top, width: PANEL_WIDTH, height: PANEL_MAX_HEIGHT });
  }, [profileView?.componentId, profileView?.position.x, profileView?.position.y]);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") closeProfileView();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [closeProfileView]);

  const trackPointer = (
    event: React.PointerEvent,
    onDelta: (dx: number, dy: number) => void,
    onFinish?: () => void,
  ) => {
    event.preventDefault();
    event.stopPropagation();
    const origin = { x: event.clientX, y: event.clientY };
    const onMove = (move: PointerEvent) => onDelta(move.clientX - origin.x, move.clientY - origin.y);
    const onUp = () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      onFinish?.();
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  };

  const startDrag = (event: React.PointerEvent) => {
    if (!rect) return;
    const start = { ...rect };
    trackPointer(
      event,
      (dx, dy) => setRect({ ...start, left: start.left + dx, top: start.top + dy }),
      () =>
        setRect((current) =>
          current === null
            ? current
            : {
                ...current,
                ...snapToViewportCorner(current, {
                  width: window.innerWidth,
                  height: window.innerHeight,
                }),
              },
        ),
    );
  };

  const startResize = (event: React.PointerEvent, edge: ResizeEdge) => {
    if (!rect) return;
    const start = { ...rect };
    trackPointer(event, (dx, dy) => {
      const next = { ...start };
      if (edge.includes("e")) next.width = Math.max(PANEL_MIN_WIDTH, start.width + dx);
      if (edge.includes("s")) next.height = Math.max(PANEL_MIN_HEIGHT, start.height + dy);
      if (edge.includes("w")) {
        next.width = Math.max(PANEL_MIN_WIDTH, start.width - dx);
        next.left = start.left + (start.width - next.width);
      }
      if (edge.includes("n")) {
        next.height = Math.max(PANEL_MIN_HEIGHT, start.height - dy);
        next.top = start.top + (start.height - next.height);
      }
      setRect(next);
    });
  };

  if (!profileView || !rect) return null;

  return (
    <>
      {!profileView.isPinned && <div className="fixed inset-0 z-40" onClick={closeProfileView} />}
      <div
        data-testid="cpu-profile-panel"
        style={{ left: rect.left, top: rect.top, width: rect.width, height: rect.height }}
        className="fixed z-50 flex flex-col gap-2 overflow-hidden rounded-md border border-slate-700 bg-[#121735] p-3 text-slate-200 shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div
          data-testid="cpu-profile-drag-handle"
          onPointerDown={startDrag}
          title="Drag to move, release near an edge to snap"
          className="-mx-3 -mt-3 flex cursor-grab items-center justify-between gap-2 border-b border-slate-700 bg-[#0b0f28] px-3 py-2 active:cursor-grabbing hover:bg-[#161c44]"
        >
          <h3 className="truncate text-sm font-semibold">cpu profile · {profileView.componentId}</h3>
          <div className="flex shrink-0 items-center gap-2">
            <button
              type="button"
              onPointerDown={(e) => e.stopPropagation()}
              onClick={toggleProfileViewPin}
              aria-pressed={profileView.isPinned}
              aria-label={profileView.isPinned ? "Unpin" : "Pin"}
              title={profileView.isPinned ? "Unpin" : "Pin"}
              className={`cursor-pointer rounded-md border px-1.5 py-1 ${
                profileView.isPinned
                  ? "border-teal-500 bg-teal-950/60 text-teal-300"
                  : "border-slate-600 bg-[#0e1330] text-slate-300"
              }`}
            >
              <PinIcon isPinned={profileView.isPinned} />
            </button>
            <button
              type="button"
              onPointerDown={(e) => e.stopPropagation()}
              onClick={closeProfileView}
              className="cursor-pointer rounded-md border border-slate-700 px-2 py-1 text-xs text-slate-300"
            >
              Close
            </button>
          </div>
        </div>
        <CpuProfileView componentId={profileView.componentId} isFullScreen={false} />
        {RESIZE_EDGES.map(({ edge, className }) => (
          <div
            key={edge}
            data-testid={`cpu-profile-resize-${edge}`}
            onPointerDown={(e) => startResize(e, edge)}
            className={`absolute ${className}`}
          />
        ))}
      </div>
    </>
  );
}

