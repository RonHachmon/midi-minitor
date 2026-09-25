import type { SourceDto } from "../bindings";
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
    <div className="max-h-[168px] overflow-auto rounded-[3px] border border-(--color-chrome-border) bg-(--color-list) px-1.5 py-1">
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
 * What a row should say and whether it can be ticked, from its availability.
 *
 * # Why unavailability is not one thing
 *
 * Three of these four states leave the checkbox **operable**, and that is
 * deliberate: a device another program is holding, or one that is simply not
 * plugged in, must be selectable so that monitoring begins the moment it comes
 * back — without the user having to notice and re-tick it.
 *
 * The fourth is different in kind. A capability this platform does not have will
 * never become available, so a box that accepts the click and does nothing would
 * be a lie the user only discovers after wasting time on it. It stays on screen,
 * in its reference position, with its verbatim label — and says why.
 *
 * Matched exhaustively with no default arm, so a new availability state is a
 * type error here rather than a row that silently renders as ordinary.
 */
function describe(availability: SourceDto["availability"]): {
  reason: string | null;
  disabled: boolean;
} {
  switch (availability.type) {
    case "open":
      return { reason: null, disabled: false };
    case "unopenable":
      return { reason: availability.data.detail, disabled: false };
    case "absent":
      return { reason: "Not connected", disabled: false };
    case "unsupported":
      return { reason: availability.data.detail, disabled: true };
  }
}

/**
 * One source row, with its reason for being unavailable if it has one.
 */
function SourceRow({
  source,
  indent = false,
}: {
  source: SourceDto;
  indent?: boolean;
}) {
  const { reason, disabled } = describe(source.availability);

  return (
    <div>
      <TriStateCheckbox
        state={source.selected ? "checked" : "unchecked"}
        label={source.name}
        disabled={disabled}
        {...(indent ? { indent: 2 } : {})}
        onToggle={(next) => void setSourceSelected(source.id, next)}
      />
      {reason !== null ? (
        <p
          className={`text-(--color-ink-faint) ${indent ? "pl-10" : "pl-6"}`}
        >
          {reason}
        </p>
      ) : null}
    </div>
  );
}
