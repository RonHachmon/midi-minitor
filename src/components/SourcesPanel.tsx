import { useState } from "react";
import { setGroupSelected, setSourceSelected } from "../ipc";
import { useMonitorStore } from "../store";
import { TriStateCheckbox } from "./TriStateCheckbox";

/**
 * The bordered, scrollable tree of monitored sources.
 *
 * Reproduces `screenshots/sources.png`: the `MIDI sources` group over its named
 * children, the standalone `Act as a destination for other programs` row, then
 * `Spy on output to destinations`. Groups start expanded, and their members are
 * indented one level.
 *
 * # Why selection keys on the id rather than the name
 *
 * Two attached devices can genuinely report the same display name — two identical
 * controllers, or two ports of one interface. They are separate sources that
 * happen to share a label, so ticking one must not tick the other.
 *
 * # What is new here, and what is deliberately not
 *
 * Real hardware creates states the reference screenshots had no way to depict: no
 * devices found, the MIDI system unreachable, a port that will not open, a
 * remembered device that is not attached, and a group whose capability is not
 * available yet. Those get new text.
 *
 * Nothing else changes. Every control the screenshots show keeps its label,
 * position, type, grouping, and indentation — the reference surface is frozen and
 * only the space around it is open.
 */
export function SourcesPanel() {
  const groups = useMonitorStore((state) => state.groups);
  const midiSystem = useMonitorStore((state) => state.midiSystem);
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  // Reaching the MIDI system and finding nothing are different facts, and the
  // remedies differ too — plug something in, versus grant access. Showing one
  // message for both would leave the user guessing which they are looking at.
  const systemUnavailable =
    midiSystem.type === "unavailable" ? midiSystem.data.detail : null;

  const hasAnySource = groups.some((group) => group.sources.length > 0);

  return (
    <div className="max-h-[168px] overflow-auto rounded-[3px] border border-(--color-chrome-border) bg-white px-1.5 py-1">
      {systemUnavailable !== null ? (
        <p className="px-1 py-1 text-(--color-ink-faint)">
          The MIDI system could not be reached: {systemUnavailable}
        </p>
      ) : null}

      {systemUnavailable === null && !hasAnySource ? (
        <p className="px-1 py-1 text-(--color-ink-faint)">
          No MIDI devices were found.
        </p>
      ) : null}

      {groups.map((group, index) => {
        // A group with no id is the standalone row block, which has no heading
        // and no parent checkbox in the reference image.
        if (group.id === null || group.label === null) {
          return group.sources.map((source) => (
            <SourceRow key={source.id} source={source} />
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
            {isOpen ? (
              <>
                {group.sources.map((source) => (
                  <SourceRow key={source.id} source={source} indent />
                ))}
                {/*
                  A group that cannot offer anything explains itself where its
                  children would be. It stays on screen because the screenshots
                  are the design authority; it says why because an empty group
                  with no explanation reads as broken.
                */}
                {group.unavailableReason !== null ? (
                  <p className="py-0.5 pl-6 text-(--color-ink-faint)">
                    {group.unavailableReason}
                  </p>
                ) : null}
              </>
            ) : null}
          </div>
        );
      })}
    </div>
  );
}

/**
 * One selectable source, with its reason for being unavailable if it has one.
 *
 * The checkbox stays operable while unavailable: a user must be able to select a
 * device that is currently held by another application, so that it starts being
 * monitored the moment it is released.
 */
function SourceRow({
  source,
  indent = false,
}: {
  source: { id: number; name: string; selected: boolean; unavailable: string | null };
  indent?: boolean;
}) {
  return (
    <div>
      <TriStateCheckbox
        state={source.selected ? "checked" : "unchecked"}
        label={source.name}
        {...(indent ? { indent: 2 } : {})}
        onToggle={(next) => void setSourceSelected(source.id, next)}
      />
      {source.unavailable !== null ? (
        <p
          className={`text-(--color-ink-faint) ${indent ? "pl-10" : "pl-6"}`}
        >
          {source.unavailable}
        </p>
      ) : null}
    </div>
  );
}
