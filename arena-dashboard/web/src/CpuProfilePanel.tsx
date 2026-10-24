import { useEffect, useRef, useState } from "react";

import {
  dimUnmatchedFrames,
  flamegraphDocument,
  formatSelfPct,
  highlightFrames,
  isProfileAvailable,
  isProfileFailed,
  profilerFailureReason,
  profilerLabel,
  severityClasses,
  visibleHotspots,
} from "./cpuProfile";
import { FloatingPanel } from "./FloatingPanel";
import { useDashboardStore } from "./store";
import type { CpuProfilerState, RenderedProfile, RuntimeFrames } from "./types";

const PANEL_WIDTH = 760;
const PANEL_MAX_HEIGHT = 560;
const PANEL_MIN_WIDTH = 360;
const PANEL_MIN_HEIGHT = 260;
const FLAMEGRAPH_MIN_HEIGHT = 240;

type LoadState = "loading" | "ready" | "unavailable";
type Layout = "stacked" | "side-by-side";

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

function RuntimeFramesInfo({ frames }: { frames: RuntimeFrames[] }) {
  return (
    <div
      data-testid="cpu-runtime-frames-info"
      className="rounded-md border border-slate-700 bg-[#0b0f28] p-2 text-xs text-slate-400"
    >
      <p className="mb-1 text-slate-300">Hidden as runtime frames, by language:</p>
      {frames.map((group) => (
        <p key={group.language} className="font-mono">
          <span className="text-slate-300">{group.language}</span>: {group.frames.join(", ")}
        </p>
      ))}
    </div>
  );
}

function HotspotTable({
  profile,
  filter,
  hideRuntimeFrames,
  onHover,
}: {
  profile: RenderedProfile;
  filter: string;
  hideRuntimeFrames: boolean;
  onHover: (func: string | null) => void;
}) {
  const ranked =
    hideRuntimeFrames && profile.application_hotspots !== undefined
      ? profile.application_hotspots
      : profile.hotspots;
  const rows = visibleHotspots(ranked, filter);

  if (rows.length === 0) {
    return (
      <div data-testid="cpu-hotspots-empty" className="p-3 text-xs text-slate-500">
        No functions match the filter
      </div>
    );
  }

  return (
    <table data-testid="cpu-hotspots" className="w-full table-fixed text-left text-xs">
      <thead className="text-slate-500">
        <tr>
          <th className="w-auto px-2 py-1 font-medium">Function</th>
          <th className="w-14 px-2 py-1 text-right font-medium">Self</th>
          <th className="w-20 px-2 py-1 text-right font-medium">Share</th>
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
            <td className="truncate px-2 py-1 font-mono text-slate-300" title={hotspot.function}>
              {hotspot.function}
            </td>
            <td className="px-2 py-1 text-right text-slate-400">{hotspot.self_count}</td>
            <td className="px-2 py-1 text-right">
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
  const [hideRuntimeFrames, setHideRuntimeFrames] = useState(false);
  const [showRuntimeFrames, setShowRuntimeFrames] = useState(false);
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
          data-testid="cpu-hide-runtime-frames"
          onClick={() => setHideRuntimeFrames(!hideRuntimeFrames)}
          aria-pressed={hideRuntimeFrames}
          className={`cursor-pointer rounded-md border px-2 py-1 text-xs uppercase tracking-wide ${
            hideRuntimeFrames
              ? "border-teal-500 bg-teal-950/60 text-teal-300"
              : "border-slate-600 bg-[#0e1330] text-slate-300"
          }`}
        >
          App only
        </button>
        <button
          type="button"
          data-testid="cpu-runtime-frames-info-toggle"
          onClick={() => setShowRuntimeFrames(!showRuntimeFrames)}
          aria-pressed={showRuntimeFrames}
          aria-label="Which functions are hidden"
          title="Which functions are hidden"
          className="cursor-pointer rounded-md border border-slate-600 bg-[#0e1330] px-2 py-1 text-xs uppercase tracking-wide text-slate-300"
        >
          i
        </button>
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

      {showRuntimeFrames && <RuntimeFramesInfo frames={profile.runtime_frames ?? []} />}

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
          <HotspotTable
            profile={profile}
            filter={filter}
            hideRuntimeFrames={hideRuntimeFrames}
            onHover={setHoveredFunction}
          />
        </div>
      </div>
    </div>
  );
}


const PROFILE_BUTTON_CLASSES: Record<"node" | "detail", string> = {
  node: "mt-0.5 px-1 py-0 text-[8px] leading-[1.5] tracking-tight",
  detail: "px-2 py-1 text-[10px] tracking-wide",
};

export function CpuProfileButton({
  componentId,
  profiler,
  variant,
}: {
  componentId: string;
  profiler: CpuProfilerState;
  variant: "node" | "detail";
}) {
  const openProfileView = useDashboardStore((s) => s.openProfileView);
  const isAvailable = isProfileAvailable(profiler.state, profiler.reportReady);
  const hasFailed = isProfileFailed(profiler.state);
  const failureReason = profilerFailureReason(profiler.state, profiler.reason);

  const chipClasses = isAvailable
    ? "cursor-pointer border-teal-500 bg-teal-950/60 text-teal-300"
    : hasFailed
      ? "cursor-default border-rose-600 bg-rose-950/60 text-rose-300"
      : "cursor-default border-slate-600 bg-slate-900/60 text-slate-400";

  const chip = (
    <button
      type="button"
      data-testid={`profiler-label-${componentId}`}
      disabled={!isAvailable}
      onClick={(e) => {
        e.stopPropagation();
        openProfileView(componentId, { x: e.clientX, y: e.clientY });
      }}
      className={`${PROFILE_BUTTON_CLASSES[variant]} whitespace-nowrap rounded-sm border uppercase ${chipClasses}`}
    >
      {profilerLabel(profiler.state, profiler.reportReady)}
    </button>
  );

  if (failureReason === null) return chip;

  if (variant === "node") {
    return (
      <span data-testid={`profiler-failure-${componentId}`} title={failureReason}>
        {chip}
      </span>
    );
  }

  return (
    <div className="flex flex-col gap-1">
      {chip}
      <p
        data-testid={`profiler-failure-${componentId}`}
        className="whitespace-pre-wrap text-[10px] leading-snug text-rose-300"
      >
        {failureReason}
      </p>
    </div>
  );
}

export function CpuProfilePopover() {
  const profiles = useDashboardStore((s) => s.profiles);
  const closeProfileView = useDashboardStore((s) => s.closeProfileView);
  const toggleProfileViewPin = useDashboardStore((s) => s.toggleProfileViewPin);

  return (
    <>
      {profiles.map((panel) => (
        <FloatingPanel
          key={panel.key}
          testId="cpu-profile-panel"
          handleId="cpu-profile"
          title={`cpu profile \u00b7 ${panel.componentId}`}
          position={panel.position}
          size={{ width: PANEL_WIDTH, height: PANEL_MAX_HEIGHT }}
          minSize={{ width: PANEL_MIN_WIDTH, height: PANEL_MIN_HEIGHT }}
          isPinned={panel.isPinned}
          onTogglePin={() => toggleProfileViewPin(panel.key)}
          onClose={() => closeProfileView(panel.key)}
        >
          <CpuProfileView componentId={panel.componentId} isFullScreen={false} />
        </FloatingPanel>
      ))}
    </>
  );
}
