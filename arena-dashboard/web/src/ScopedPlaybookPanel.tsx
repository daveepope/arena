import { fillClasses, formatDetail, isActiveState } from "./presentation";
import { useDashboardStore } from "./store";

const NO_PLAYBOOKS = new Map();

export function ScopedPlaybookPanel({ arenaId }: { arenaId: string }) {
  const playbooks = useDashboardStore((s) => s.arenas.get(arenaId)?.playbooks ?? NO_PLAYBOOKS);
  const select = useDashboardStore((s) => s.select);

  const rows = [...playbooks.entries()].sort(([left], [right]) => left.localeCompare(right));

  return (
    <aside
      data-testid={`scoped-playbooks-${arenaId}`}
      className="h-[480px] w-56 shrink-0 overflow-y-auto rounded-md border border-slate-800 bg-[#0e1330] p-2"
    >
      <h3 className="mb-2 px-1 text-[10px] uppercase tracking-wide text-slate-400">playbooks</h3>
      {rows.length === 0 ? (
        <p data-testid={`no-playbooks-${arenaId}`} className="px-1 text-[11px] text-slate-500">
          no playbooks registered
        </p>
      ) : (
        <ul className="flex flex-col gap-1">
          {rows.map(([playbookId, playbook]) => (
            <li key={playbookId}>
              <button
                type="button"
                data-testid={`scoped-playbook-${playbookId}`}
                onClick={(e) =>
                  select({
                    title: playbookId,
                    detail: formatDetail(playbookId, "playbook", playbook.state, playbook.faults, 0),
                    position: { x: e.clientX, y: e.clientY },
                  })
                }
                className={`w-full cursor-pointer rounded-sm border border-slate-700 px-2 py-1 text-left text-[11px] transition-colors ${
                  playbook.state === "faulted" || playbook.faults.length > 0
                    ? "bg-rose-600/30 text-rose-200"
                    : fillClasses(playbook.state)
                } ${isActiveState(playbook.state) ? "animate-pulse-soft" : ""}`}
              >
                <span className="block truncate">{playbookId}</span>
                <span className="block text-[9px] uppercase tracking-wide opacity-70">
                  {playbook.kind} / {playbook.state}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </aside>
  );
}
