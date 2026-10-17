import type { Hotspot, Severity } from "./types";

export const PROFILE_AVAILABLE_STATE = "stopped";

const FRAME_SELECTOR = "#frames g";
const DIMMED_OPACITY = "0.15";
const FULL_OPACITY = "1";
const HIGHLIGHT_STROKE = "#f8fafc";

export const PROFILE_REPORT_LABEL = "cpu profile report";

export function profilerLabel(state: string, reportReady: boolean): string {
  if (isProfileAvailable(state, reportReady)) return PROFILE_REPORT_LABEL;
  return `cpu profiler ${state}`;
}

export function isProfileAvailable(
  state: string | undefined,
  reportReady: boolean | undefined,
): boolean {
  return state === PROFILE_AVAILABLE_STATE && reportReady === true;
}

export function matchesFilter(text: string, pattern: string): boolean {
  if (pattern.trim() === "") return true;
  try {
    return new RegExp(pattern, "i").test(text);
  } catch {
    return text.toLowerCase().includes(pattern.toLowerCase());
  }
}

export function frameFunctionName(title: string): string {
  const open = title.lastIndexOf(" (");
  return open === -1 ? title : title.slice(0, open);
}

export function severityClasses(severity: Severity): string {
  switch (severity) {
    case "critical":
      return "border-rose-700 bg-rose-950/50 text-rose-300";
    case "high":
      return "border-orange-700 bg-orange-950/50 text-orange-300";
    case "medium":
      return "border-amber-700 bg-amber-950/50 text-amber-300";
    case "low":
    default:
      return "border-slate-700 bg-slate-900/50 text-slate-400";
  }
}

export function formatSelfPct(selfPct: number): string {
  return `${selfPct.toFixed(1)}%`;
}

interface Frame {
  group: SVGGElement;
  func: string;
}

export function flamegraphFrames(root: Document | Element): Frame[] {
  return Array.from(root.querySelectorAll<SVGGElement>(FRAME_SELECTOR)).map((group) => ({
    group,
    func: frameFunctionName(group.querySelector("title")?.textContent ?? ""),
  }));
}

export function dimUnmatchedFrames(root: Document | Element, pattern: string): number {
  let matched = 0;
  flamegraphFrames(root).forEach(({ group, func }) => {
    const isMatch = matchesFilter(func, pattern);
    if (isMatch) matched += 1;
    group.style.opacity = isMatch ? FULL_OPACITY : DIMMED_OPACITY;
  });
  return matched;
}

export function highlightFrames(root: Document | Element, func: string | null): number {
  let highlighted = 0;
  flamegraphFrames(root).forEach((frame) => {
    const rect = frame.group.querySelector<SVGRectElement>("rect");
    if (!rect) return;
    const isMatch = func !== null && frame.func === func;
    if (isMatch) highlighted += 1;
    rect.style.stroke = isMatch ? HIGHLIGHT_STROKE : "";
    rect.style.strokeWidth = isMatch ? "2" : "";
  });
  return highlighted;
}

export function visibleHotspots(hotspots: Hotspot[], pattern: string): Hotspot[] {
  return hotspots.filter((hotspot) => matchesFilter(hotspot.function, pattern));
}

export function flamegraphDocument(svg: string): string {
  const bindHandlers =
    "<script>document.addEventListener('DOMContentLoaded',function(){" +
    "var root=document.querySelector('svg');" +
    "if(root&&typeof window.init==='function')window.init({target:root});" +
    "});<\/script>";
  return `<!doctype html><html><head><style>html,body{margin:0;padding:0;background:#1e1e1e}svg{width:100%;height:auto}</style></head><body>${svg}${bindHandlers}</body></html>`;
}
