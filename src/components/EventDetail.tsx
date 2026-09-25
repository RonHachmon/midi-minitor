import { useEffect, useRef } from "react";
import type { EventDto } from "../bindings";

/** Bytes per line of the dump, the width every hex viewer has settled on. */
const BYTES_PER_LINE = 16;

/** Hex digits in an offset label, enough for the 64 KB System Exclusive cap. */
const OFFSET_DIGITS = 4;

/**
 * One line of the dump: an offset and the bytes that start there.
 */
interface DumpLine {
  /** Byte offset of the first byte on this line, as hex. */
  offset: string;
  /** Up to [`BYTES_PER_LINE`] bytes, space separated. */
  bytes: string;
}

/**
 * Splits the event's raw hex into offset-labelled lines.
 *
 * # Why this is done here rather than in Rust
 *
 * [`super::EventTable`]'s module note explains that MIDI *rules* belong in Rust
 * — note names, controller names, what a Data cell means. This is not one. It is
 * line wrapping: the same string, folded at a width chosen for a dialog box.
 * Sending a pre-wrapped copy would put a third rendering of the same bytes on
 * every event in the stream, to serve a view of one row that is usually closed.
 *
 * Deriving it here costs nothing until a row is actually opened, and the string
 * it derives from is the one the core already vouches for.
 */
function dumpLines(rawHex: string): DumpLine[] {
  const pairs = rawHex.match(/../g) ?? [];
  const lines: DumpLine[] = [];
  for (let start = 0; start < pairs.length; start += BYTES_PER_LINE) {
    lines.push({
      offset: start.toString(16).toUpperCase().padStart(OFFSET_DIGITS, "0"),
      bytes: pairs.slice(start, start + BYTES_PER_LINE).join(" "),
    });
  }
  return lines;
}

/**
 * The bytes of one event, in full, as raw hexadecimal.
 *
 * # Why this surface exists
 *
 * The Data cell truncates. It has to — it is one column of a fixed-width row,
 * and a System Exclusive transfer runs to
 * [`MAX_SYSEX_BYTES`](../../crates/midi-core/src/constants.rs) bytes. The row's
 * tooltip carries the whole string but cannot be read at that length, selected,
 * or scrolled. Neither can show a user which byte sits at which offset.
 *
 * So this is where the bytes go when there are more of them than a table can
 * hold, and it is additive: nothing the screenshots depict changes shape to
 * accommodate it, which is the rule for a surface the reference never captured.
 *
 * # Why it is the native `dialog` element
 *
 * `showModal` brings the focus trap, the Escape binding, inertness of the page
 * behind it, and a `::backdrop` to style — all behaviours that a `div` overlay
 * has to reimplement and usually gets subtly wrong. The platform's version is
 * already correct and already accessible.
 *
 * # Why it renders raw bytes regardless of `Expert mode`
 *
 * `Expert mode` decides what the *table* shows. This dialog has one job, and a
 * user who opens it has already said which representation they want by opening
 * it. Making its contents depend on a checkbox elsewhere would mean the only
 * place that always shows the bytes sometimes does not.
 */
export function EventDetail({
  event,
  onClose,
}: {
  /** The event to describe, or `null` when nothing is open. */
  event: EventDto | null;
  /** Called when the dialog is dismissed, however it was dismissed. */
  onClose: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);

  // Driving the element from the prop rather than rendering it conditionally:
  // `showModal` is what puts the dialog in the top layer, so React must not be
  // the thing that decides whether it is in the DOM.
  useEffect(() => {
    const element = dialog.current;
    if (element === null) {
      return;
    }
    if (event !== null && !element.open) {
      element.showModal();
    } else if (event === null && element.open) {
      element.close();
    }
  }, [event]);

  const lines = event === null ? [] : dumpLines(event.rawHex);
  const byteCount = event === null ? 0 : (event.rawHex.match(/../g) ?? []).length;

  return (
    <dialog
      ref={dialog}
      className="event-detail"
      aria-labelledby="event-detail-title"
      // Escape and the close button both raise `close`, so one handler covers
      // every route out and the parent never has to distinguish them.
      onClose={onClose}
      // A click that lands on the element itself rather than on its contents is
      // a click on the backdrop: the padding-free dialog has no other surface.
      onClick={(clicked) => {
        if (clicked.target === dialog.current) {
          onClose();
        }
      }}
    >
      {event !== null && (
        <div className="flex max-h-[84vh] flex-col">
          <div className="flex shrink-0 items-center justify-between gap-4 border-b border-(--color-hairline) px-4 py-2">
            <h2 id="event-detail-title" className="text-[13px] font-medium">
              {event.message}
              <span className="ml-2 text-(--color-ink-faint)">
                {event.time}
              </span>
            </h2>
            <button
              type="button"
              className="chrome-button px-2 py-0.5 text-[13px]"
              onClick={onClose}
            >
              Close
            </button>
          </div>

          <dl className="grid shrink-0 grid-cols-[auto_1fr] gap-x-4 gap-y-1 border-b border-(--color-hairline) px-4 py-3 text-[13px]">
            <Field label="Source" value={event.source} />
            <Field
              label="Channel"
              // An em dash rather than the table's blank cell: a labelled field
              // left empty reads as a rendering fault, where the table's blank
              // is what the reference image requires of it.
              value={event.channel === null ? "—" : String(event.channel)}
            />
            <Field label="Data" value={event.data} />
          </dl>

          <div className="flex min-h-0 flex-1 flex-col bg-(--color-list)">
            <p className="shrink-0 px-4 pt-3 pb-1 text-[12px] text-(--color-ink-faint)">
              Raw bytes · {byteCount}
            </p>
            <div className="min-h-0 flex-1 overflow-auto px-4 pb-3 select-text">
              <table className="font-(family-name:--font-hex) text-[12px] leading-[1.5]">
                <tbody>
                  {lines.map((line) => (
                    <tr key={line.offset}>
                      <td className="pr-4 text-(--color-ink-faint) select-none">
                        {line.offset}
                      </td>
                      <td className="whitespace-pre text-(--color-ink)">
                        {line.bytes}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      )}
    </dialog>
  );
}

/** One labelled value in the header block. */
function Field({ label, value }: { label: string; value: string }) {
  return (
    <>
      <dt className="text-(--color-ink-faint)">{label}</dt>
      <dd className="m-0 break-all">{value}</dd>
    </>
  );
}
