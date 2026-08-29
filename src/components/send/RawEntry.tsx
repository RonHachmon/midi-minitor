import { composeRaw } from "../../ipc";
import { useSendStore } from "../../sendStore";

/**
 * Hand-typed bytes, for someone who already knows what they want.
 *
 * # Why the text is not parsed here
 *
 * Deciding whether a byte string is a valid MIDI message is a judgement the core
 * already makes, with the very decoder that produces the rows in the event table.
 * A second parser in TypeScript would disagree with it at the edges — which for a
 * *monitor* would mean this screen refusing to transmit something the table
 * displays happily. So the text is presentation state, the core is asked, and
 * whatever it says is what the user is told.
 *
 * # Why this is never the only way to reach anything
 *
 * It is an alternative to the composer above, not a fallback for what the
 * composer cannot express. Every message this application can name can be built
 * from named controls.
 */
export function RawEntry() {
  const rawEntry = useSendStore((state) => state.rawEntry);
  const setRawEntry = useSendStore((state) => state.setRawEntry);

  const submit = () => {
    if (rawEntry.trim() === "") {
      return;
    }
    void composeRaw(rawEntry);
  };

  return (
    <section className="border-b border-(--color-hairline) px-3 py-2">
      <h2 className="pb-1 text-[13px] font-semibold text-(--color-ink)">Or type bytes</h2>
      <div className="flex items-center gap-2">
        <input
          type="text"
          value={rawEntry}
          placeholder="90 3C 64"
          onChange={(event) => setRawEntry(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              submit();
            }
          }}
          aria-label="Bytes to send"
          className="min-w-0 flex-1 rounded border border-(--color-chrome-border) bg-(--color-control) px-2 py-1 font-mono text-[13px] tracking-wider text-(--color-ink)"
        />
        <button
          type="button"
          onClick={submit}
          disabled={rawEntry.trim() === ""}
          className="chrome-button px-3 py-1 text-[13px]"
        >
          Use
        </button>
      </div>
    </section>
  );
}
