import { useEffect, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { ColumnDto, EventDto } from "../bindings";
import { useMonitorStore } from "../store";
import { ColumnMenu } from "./ColumnMenu";
import { EventDetail } from "./EventDetail";

/**
 * Width for each column, keyed by its wire identifier.
 *
 * # Why Time is the one column allowed to grow
 *
 * `108px` is what `screenshots/data.png` shows, and it is exactly enough for the
 * `HH:MM:SS.mmm` that column held when the reference was captured. Display
 * preferences can now put a host-clock reading there instead — thirteen digits
 * for a tick count, more again for seconds carried to the nanosecond — and a
 * fixed width truncated those to an ellipsis, which defeats the whole reason
 * someone selects a host format.
 *
 * `minmax(108px, max-content)` keeps the depicted width in the depicted state:
 * under `Clock time` every value is the same length and the column stays at
 * 108px, so the reference image still matches. It grows only for a format the
 * screenshots never depicted, which is the additive-surface rule working as
 * intended rather than an exception to it.
 */
const COLUMN_WIDTH: Record<string, string> = {
  time: "minmax(108px, max-content)",
  source: "210px",
  message: "150px",
  chan: "52px",
  data: "minmax(0, 1fr)",
};

/** Row height in pixels, matching the `--spacing-row` token. */
const ROW_HEIGHT = 24.5;

/** Reads one column's cell out of an event. */
function cellValue(event: EventDto, columnId: string): string {
  switch (columnId) {
    case "time":
      return event.time;
    case "source":
      return event.source;
    case "message":
      return event.message;
    case "chan":
      // Blank rather than a placeholder: messages carrying no channel must show
      // an empty cell, as in the reference window.
      return event.channel === null ? "" : String(event.channel);
    case "data":
      return event.data;
    default:
      return "";
  }
}

/**
 * Explains an empty list.
 *
 * # Why four messages rather than one
 *
 * An empty monitor has four quite different causes and the user's next action
 * differs for each: nothing is selected, everything is filtered out, the monitor
 * is paused, or traffic simply has not arrived yet. A single "Waiting for events…"
 * would be actively misleading in the first three — the application would look
 * stalled when it is doing exactly what it was told. `retainedCount` separates
 * the filtered case: events are being retained, they are just not passing the
 * filter.
 *
 * # Why the order is what it is
 *
 * Selection is checked before pause because it is the more actionable of the two:
 * a user who resumes a monitor with no source selected still sees nothing, and
 * would have been told the wrong thing. Pause is checked before "waiting" because
 * while paused nothing is coming — saying otherwise would be a straightforward
 * untruth.
 */
function emptyReason(
  monitoring: boolean,
  retainedCount: number,
  paused: boolean,
): string {
  if (!monitoring) {
    return "No sources selected — nothing is being monitored.";
  }
  if (retainedCount > 0) {
    return `All ${retainedCount} retained events are hidden by the current filter.`;
  }
  if (paused) {
    return "Paused — no events were retained before pausing.";
  }
  return "Waiting for events…";
}

/**
 * The scrolling event list.
 *
 * # Why the rows are virtualized
 *
 * The retention cap reaches 100 000 events. Rendering that many table rows
 * would cost hundreds of megabytes of DOM and drop the window well below the
 * responsiveness the specification requires. The virtualizer keeps only the
 * visible rows mounted, so scroll cost is independent of how much is retained.
 *
 * # Why it does not decide what to show
 *
 * Every event handed to this component has already passed the filters in Rust.
 * There is no predicate here beyond which *columns* are visible — filtering in
 * the webview would put a MIDI rule on the wrong side of the boundary.
 */
export function EventTable() {
  const events = useMonitorStore((state) => state.events);
  const columns = useMonitorStore((state) => state.columns);
  const monitoring = useMonitorStore((state) => state.monitoring);
  const retainedCount = useMonitorStore((state) => state.retainedCount);
  const captureState = useMonitorStore((state) => state.captureState);
  const byteFidelity = useMonitorStore((state) => state.byteFidelity);

  const scroller = useRef<HTMLDivElement>(null);
  const pinnedToBottom = useRef(true);

  // The open row is held by id rather than as the event object, so the dialog
  // re-reads the current rendering of that event. A display setting changed
  // while it is open therefore updates it, instead of leaving a stale copy on
  // screen; and an event dropped by the retention cap resolves to `undefined`,
  // which closes the dialog rather than freezing a row that no longer exists.
  const [openId, setOpenId] = useState<number | null>(null);
  const opened = openId === null
    ? null
    : (events.find((candidate) => candidate.id === openId) ?? null);

  const virtualizer = useVirtualizer({
    count: events.length,
    getScrollElement: () => scroller.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 12,
  });

  // New events arrive at the bottom, so the list follows them — but only while
  // the user is already there. Yanking the view back down while someone is
  // reading history would make the monitor unusable at speed.
  useEffect(() => {
    if (pinnedToBottom.current && events.length > 0) {
      virtualizer.scrollToIndex(events.length - 1, { align: "end" });
    }
  }, [events.length, virtualizer]);

  const visible: ColumnDto[] = columns.filter((column) => column.visible);
  const template = visible
    .map((column) => COLUMN_WIDTH[column.id] ?? "minmax(0, 1fr)")
    .join(" ");

  // The note belongs to the Data column, so it appears and disappears with it —
  // a statement about a column nobody is looking at is clutter, and one about a
  // column that is visible is the whole obligation.
  //
  // Matched exhaustively with no default arm: a platform that reports its bytes
  // some third way must become a type error here rather than silently rendering
  // as though it were faithful.
  const dataVisible = visible.some((column) => column.id === "data");
  const fidelityNote =
    byteFidelity.type === "assembled" ? byteFidelity.data.detail : null;

  // The list is deliberately selectable so values can be copied out of it, and
  // finishing a drag-selection produces a click. Opening the dialog on that
  // click would make the two features fight, so a click that ends a selection
  // is treated as part of the selection rather than as a row activation.
  const openRow = (id: number) => {
    const selection = window.getSelection();
    if (selection !== null && selection.toString().length > 0) {
      return;
    }
    setOpenId(id);
  };

  const onScroll = () => {
    const element = scroller.current;
    if (!element) {
      return;
    }
    const distance =
      element.scrollHeight - element.scrollTop - element.clientHeight;
    pinnedToBottom.current = distance < ROW_HEIGHT * 2;
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col border-t border-(--color-hairline) bg-(--color-list)">
      {/*
        The menu sits outside the grid rather than as an extra child of it: the
        grid has exactly one track per visible column, so a sixth child would be
        placed in an implicit track past the container's right edge and never be
        clickable. Overlaying it keeps the header's tracks aligned with the body
        rows, which is what matters for the columns to line up.
      */}
      <div className="relative shrink-0 border-b border-(--color-hairline) bg-(--color-header)">
        <div
          className="grid items-center pr-6 text-[13px] text-(--color-ink-soft)"
          style={{ gridTemplateColumns: template }}
        >
          {visible.map((column, index) => (
            <div
              key={column.id}
              className={`px-2 py-1 ${index > 0 ? "border-l border-(--color-hairline)" : ""}`}
            >
              {column.label}
            </div>
          ))}
        </div>
        <div className="absolute inset-y-0 right-0 flex items-center">
          <ColumnMenu />
        </div>
      </div>

      {/*
        A standing statement, not a per-row annotation. The application cannot
        know which individual messages the platform assembled — it never sees the
        original — so a per-row claim would be a fabrication. It sits under the
        header, with the column it is about.
      */}
      {dataVisible && fidelityNote !== null ? (
        <p className="shrink-0 border-b border-(--color-hairline) bg-(--color-header) px-2 py-1 text-[12px] text-(--color-ink-faint)">
          {fidelityNote}
        </p>
      ) : null}

      <div
        ref={scroller}
        onScroll={onScroll}
        className="min-h-0 flex-1 overflow-auto select-text"
      >
        {events.length === 0 ? (
          <p className="px-2 py-3 text-[13px] text-(--color-ink-faint)">
            {emptyReason(
              monitoring,
              retainedCount,
              captureState.type === "paused",
            )}
          </p>
        ) : (
          <div
            className="relative w-full"
            style={{ height: `${virtualizer.getTotalSize()}px` }}
          >
            {virtualizer.getVirtualItems().map((item) => {
              const event = events[item.index];
              if (event === undefined) {
                return null;
              }
              return (
                <div
                  key={event.id}
                  title={event.rawHex}
                  // A button rather than a row with a handler bolted on: this
                  // opens a dialog, which is what a button does, and the role
                  // is what tells a screen reader the list is explorable at all.
                  role="button"
                  tabIndex={0}
                  onClick={() => openRow(event.id)}
                  onKeyDown={(pressed) => {
                    if (pressed.key === "Enter" || pressed.key === " ") {
                      // Space scrolls the list by default, which would move the
                      // row out from under the user as it opened.
                      pressed.preventDefault();
                      openRow(event.id);
                    }
                  }}
                  className="event-row absolute top-0 left-0 grid w-full items-center text-[13px] text-(--color-ink)"
                  style={{
                    height: `${item.size}px`,
                    transform: `translateY(${item.start}px)`,
                    gridTemplateColumns: template,
                  }}
                >
                  {visible.map((column) => (
                    <div
                      key={column.id}
                      className="truncate px-2"
                    >
                      {cellValue(event, column.id)}
                    </div>
                  ))}
                </div>
              );
            })}
          </div>
        )}
      </div>

      <EventDetail event={opened} onClose={() => setOpenId(null)} />
    </div>
  );
}
