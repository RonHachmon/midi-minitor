import { useState } from "react";
import { setGroupSelected, setSourceSelected } from "../ipc";
import { useMonitorStore } from "../store";
import { TriStateCheckbox } from "./TriStateCheckbox";

/**
 * The bordered, scrollable tree of monitored sources.
 *
 * Reproduces `screenshots/sources.png`: the `MIDI sources` group over its named
 * children, the standalone `Act as a destination for other programs` row, then
 * `Spy on output to destinations` over its child. Groups start expanded, and
 * their members are indented one level.
 *
 * # Why selection keys on the id rather than the name
 *
 * `IAC Driver Bus 1` appears twice in the reference image, under two different
 * groups. They are separate sources that happen to share a label, so ticking one
 * must not tick the other.
 */
export function SourcesPanel() {
  const groups = useMonitorStore((state) => state.groups);
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  return (
    <div className="max-h-[168px] overflow-auto rounded-[3px] border border-(--color-chrome-border) bg-white px-1.5 py-1">
      {groups.map((group, index) => {
        // A group with no id is the standalone row block, which has no heading
        // and no parent checkbox in the reference image.
        if (group.id === null || group.label === null) {
          return group.sources.map((source) => (
            <TriStateCheckbox
              key={source.id}
              state={source.selected ? "checked" : "unchecked"}
              label={source.name}
              onToggle={(next) => void setSourceSelected(source.id, next)}
            />
          ));
        }

        const groupId = group.id;
        const isOpen = collapsed[groupId] !== true;

        return (
          <div key={groupId ?? `standalone-${index}`}>
            <div className="flex items-center gap-1">
              <button
                type="button"
                aria-label={isOpen ? "Collapse group" : "Expand group"}
                aria-expanded={isOpen}
                onClick={() =>
                  setCollapsed((current) => ({
                    ...current,
                    [groupId]: isOpen,
                  }))
                }
                className="w-3 text-[9px] leading-none text-(--color-ink-faint)"
              >
                {isOpen ? "▼" : "▶"}
              </button>
              <TriStateCheckbox
                state={group.state}
                label={group.label}
                onToggle={(next) => void setGroupSelected(groupId, next)}
              />
            </div>
            {isOpen
              ? group.sources.map((source) => (
                  <TriStateCheckbox
                    key={source.id}
                    state={source.selected ? "checked" : "unchecked"}
                    label={source.name}
                    indent={2}
                    onToggle={(next) => void setSourceSelected(source.id, next)}
                  />
                ))
              : null}
          </div>
        );
      })}
    </div>
  );
}
