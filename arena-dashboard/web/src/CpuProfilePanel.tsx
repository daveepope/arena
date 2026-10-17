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

export function CpuProfilePopover() {
  const profileView = useDashboardStore((s) => s.profileView);
  const closeProfileView = useDashboardStore((s) => s.closeProfileView);
  const toggleProfileViewPin = useDashboardStore((s) => s.toggleProfileViewPin);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") closeProfileView();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [closeProfileView]);

  if (!profileView) return null;

  const { left, top } = clampPanelPosition(profileView.position.x, profileView.position.y);

  return (
    <>
      {!profileView.isPinned && <div className="fixed inset-0 z-40" onClick={closeProfileView} />}
      <div
        data-testid="cpu-profile-panel"
        style={{ left, top, width: PANEL_WIDTH, maxHeight: PANEL_MAX_HEIGHT }}
        className="fixed z-50 flex flex-col gap-2 rounded-md border border-slate-700 bg-[#121735] p-3 text-slate-200 shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between gap-2">
          <h3 className="truncate text-sm font-semibold">cpu profile · {profileView.componentId}</h3>
          <div className="flex shrink-0 items-center gap-2">
            <button
              type="button"
              onClick={toggleProfileViewPin}
              aria-pressed={profileView.isPinned}
              className="cursor-pointer rounded-md border border-slate-600 bg-[#0e1330] px-2 py-1 text-xs uppercase tracking-wide text-slate-300"
            >
              {profileView.isPinned ? "Unpin" : "Pin"}
            </button>
            <button
              type="button"
              onClick={closeProfileView}
              className="cursor-pointer rounded-md border border-slate-700 px-2 py-1 text-xs text-slate-300"
            >
              Close
            </button>
          </div>
        </div>
        <CpuProfileView componentId={profileView.componentId} isFullScreen={false} />
      </div>
    </>
  );
}
