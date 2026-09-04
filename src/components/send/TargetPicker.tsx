import { setSendTarget } from "../../ipc";
import { useSendStore } from "../../sendStore";

/**
 * Where traffic goes.
 *
 * # Why an empty list is a sentence rather than an empty control
 *
 * A machine with nothing to send to is an ordinary state, not a fault, and a
 * picker with no entries says nothing about which of the two it is. The same
 * honesty the Sources panel already shows for an empty device list.
 *
 * # Why the published source is marked
 *
 * Only that kind of target carries the name the user chose. Traffic sent to a
 * destination carries whatever origin the operating system reports for this
 * application, and letting the two look identical would imply a disguise that is
 * not in force.
 */
export function TargetPicker() {
  const view = useSendStore((state) => state.view);

  if (view === null) {
    return null;
  }

  const { targets, chosen } = view.targets;

  return (
    <section className="border-b border-(--color-hairline) px-3 py-2">
      <h2 className="pb-1 text-[13px] font-semibold text-(--color-ink)">Send to</h2>

      {targets.length === 0 ? (
        <p className="text-[13px] text-(--color-ink-soft)">
          This machine reports nowhere to send. Attach a MIDI device, or publish a source below so
          another program can receive from it.
        </p>
      ) : (
        <select
          value={chosen ?? ""}
          onChange={(event) => void setSendTarget(Number(event.target.value))}
          aria-label="Send to"
          className="w-full rounded border border-(--color-chrome-border) bg-(--color-control) px-2 py-1 text-[13px] text-(--color-ink)"
        >
          <option value="" disabled>
            Choose a target…
          </option>
          {targets.map((target) => (
            <option key={target.id} value={target.id}>
              {target.kind === "publishedSource"
                ? `${target.name} (published by this app)`
                : target.name}
            </option>
          ))}
        </select>
      )}
    </section>
  );
}
