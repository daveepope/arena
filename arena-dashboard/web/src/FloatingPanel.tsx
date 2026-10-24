import { useEffect, useState } from "react";

const VIEWPORT_MARGIN = 8;
const SNAP_THRESHOLD_PX = 48;

export interface PanelRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

export type ResizeEdge = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";

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

export function clampPanelPosition(
  x: number,
  y: number,
  size: { width: number; height: number },
): { left: number; top: number } {
  const maxLeft = window.innerWidth - size.width - VIEWPORT_MARGIN;
  const maxTop = window.innerHeight - size.height - VIEWPORT_MARGIN;
  return {
    left: Math.min(Math.max(x, VIEWPORT_MARGIN), Math.max(maxLeft, VIEWPORT_MARGIN)),
    top: Math.min(Math.max(y, VIEWPORT_MARGIN), Math.max(maxTop, VIEWPORT_MARGIN)),
  };
}

export function snapToViewportCorner(
  rect: PanelRect,
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

export function FloatingPanel({
  testId,
  handleId,
  title,
  position,
  size,
  minSize,
  isPinned,
  onTogglePin,
  onClose,
  children,
}: {
  testId: string;
  handleId: string;
  title: string;
  position: { x: number; y: number };
  size: { width: number; height: number };
  minSize: { width: number; height: number };
  isPinned: boolean;
  onTogglePin: () => void;
  onClose: () => void;
  children: React.ReactNode;
}) {
  const [rect, setRect] = useState<PanelRect | null>(null);

  useEffect(() => {
    const { left, top } = clampPanelPosition(position.x, position.y, size);
    setRect({ left, top, width: size.width, height: size.height });
  }, [testId, title, position.x, position.y, size.width, size.height]);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

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
      if (edge.includes("e")) next.width = Math.max(minSize.width, start.width + dx);
      if (edge.includes("s")) next.height = Math.max(minSize.height, start.height + dy);
      if (edge.includes("w")) {
        next.width = Math.max(minSize.width, start.width - dx);
        next.left = start.left + (start.width - next.width);
      }
      if (edge.includes("n")) {
        next.height = Math.max(minSize.height, start.height - dy);
        next.top = start.top + (start.height - next.height);
      }
      setRect(next);
    });
  };

  if (!rect) return null;

  return (
    <>
      {!isPinned && <div className="fixed inset-0 z-40" onClick={onClose} />}
      <div
        data-testid={testId}
        style={{ left: rect.left, top: rect.top, width: rect.width, height: rect.height }}
        className="fixed z-50 flex flex-col gap-2 overflow-hidden rounded-md border border-slate-700 bg-[#121735] p-3 text-slate-200 shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div
          data-testid={`${handleId}-drag-handle`}
          onPointerDown={startDrag}
          title="Drag to move, release near an edge to snap"
          className="-mx-3 -mt-3 flex cursor-grab items-center justify-between gap-2 border-b border-slate-700 bg-[#0b0f28] px-3 py-2 active:cursor-grabbing hover:bg-[#161c44]"
        >
          <h3 className="truncate text-sm font-semibold">{title}</h3>
          <div className="flex shrink-0 items-center gap-2">
            <button
              type="button"
              onPointerDown={(e) => e.stopPropagation()}
              onClick={onTogglePin}
              aria-pressed={isPinned}
              aria-label={isPinned ? "Unpin" : "Pin"}
              title={isPinned ? "Unpin" : "Pin"}
              className={`cursor-pointer rounded-md border px-1.5 py-1 ${
                isPinned
                  ? "border-teal-500 bg-teal-950/60 text-teal-300"
                  : "border-slate-600 bg-[#0e1330] text-slate-300"
              }`}
            >
              <PinIcon isPinned={isPinned} />
            </button>
            <button
              type="button"
              onPointerDown={(e) => e.stopPropagation()}
              onClick={onClose}
              className="cursor-pointer rounded-md border border-slate-700 px-2 py-1 text-xs text-slate-300"
            >
              Close
            </button>
          </div>
        </div>
        <div className="min-h-0 flex-1 overflow-auto">{children}</div>
        {RESIZE_EDGES.map(({ edge, className }) => (
          <div
            key={edge}
            data-testid={`${handleId}-resize-${edge}`}
            onPointerDown={(e) => startResize(e, edge)}
            className={`absolute ${className}`}
          />
        ))}
      </div>
    </>
  );
}
