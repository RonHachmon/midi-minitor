import { useState } from "react";
import { setColumnVisibility } from "../ipc";
import { useMonitorStore } from "../store";

/**
 * Chooses which columns the event table shows.
 *
 * # Why this is a small header affordance rather than a panel
 *
 * Column visibility is not in the reference screenshots. Additive capability has
 * to earn new surface without disturbing anything the images depict, so it
 * cannot be folded into the Filter panel or change the header's appearance at
 * rest — hence a single unobtrusive control at the end of the header row.
 *
 * The core refuses to hide the last visible column, and that refusal arrives
 * here as a typed error rather than being pre-empted by disabling the box: one
 * rule, enforced in one place.
 */
export function ColumnMenu() {
  const columns = useMonitorStore((state) => state.columns);
  const [open, setOpen] = useState(false);

  const toggle = (id: string, visible: boolean) => {
    const next = columns
      .filter((column) => (column.id === id ? visible : column.visible))
      .map((column) => column.id);
    void setColumnVisibility(next);
  };

  return (
    <div className="relative flex justify-end px-1">
      <button
        type="button"
        aria-label="Choose columns"
        aria-expanded={open}
        onClick={() => setOpen((shown) => !shown)}
        className="px-1 text-[11px] leading-none text-(--color-ink-faint) hover:text-(--color-ink)"
      >
        ⋮
      </button>
      {open ? (
        <div
          className="absolute top-full right-1 z-10 min-w-[140px] rounded border border-(--color-chrome-border) bg-(--color-chrome) p-1 shadow-md"
          onMouseLeave={() => setOpen(false)}
        >
          {columns.map((column) => (
            <label
              key={column.id}
              className="flex items-center gap-1.5 px-1 py-[2px] text-[13px]"
            >
              <input
                type="checkbox"
                checked={column.visible}
                onChange={(entry) => toggle(column.id, entry.target.checked)}
                className="h-[13px] w-[13px] accent-(--color-accent)"
              />
              {column.label}
            </label>
          ))}
        </div>
      ) : null}
    </div>
  );
}
