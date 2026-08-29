import { setCompositionField, setCompositionKind } from "../../ipc";
import { useSendStore } from "../../sendStore";
import type { FieldDto } from "../../bindings";

/**
 * The message being built, from controls labelled by meaning.
 *
 * # Why this holds no table of which message carries which values
 *
 * It renders the fields it is handed. "A Control Change carries a channel, a
 * controller, and a value" is the MIDI specification, not a layout preference, so
 * it lives in the core and arrives as data. That is what makes the promise that
 * exactly the right controls are shown structural rather than something a
 * reviewer has to check against a spec sheet.
 *
 * The bounds are handed over the same way. Nothing in this file knows that a
 * velocity stops at 127.
 *
 * # Why a keystroke is held locally before it is sent
 *
 * Committing on every keystroke would fight the user: typing `100` passes through
 * `1` and `10`, and the core would answer each with a re-rendered value. So the
 * in-flight text is presentation state here, and the value is committed when the
 * field is left or the entry is confirmed. What is never computed here is the
 * *result* — the bytes still come back from the core.
 */
export function Composer() {
  const view = useSendStore((state) => state.view);

  if (view === null) {
    return null;
  }

  const { kind, kinds, fields } = view.composition;

  return (
    <section className="border-b border-(--color-hairline) px-3 py-2">
      <h2 className="pb-1 text-[13px] font-semibold text-(--color-ink)">Message</h2>

      <select
        value={kind ?? ""}
        onChange={(event) => void setCompositionKind(event.target.value)}
        aria-label="Message type"
        className="w-full rounded border border-(--color-chrome-border) bg-(--color-control) px-2 py-1 text-[13px] text-(--color-ink)"
      >
        {kind === null && (
          <option value="" disabled>
            Typed as bytes
          </option>
        )}
        {kinds.map((candidate) => (
          <option key={candidate.id} value={candidate.id}>
            {candidate.label}
          </option>
        ))}
      </select>

      {fields.length > 0 && (
        <div className="grid grid-cols-[auto_1fr] items-center gap-x-3 gap-y-1.5 pt-2">
          {fields.map((field) => (
            <ValueField key={field.id} field={field} />
          ))}
        </div>
      )}
    </section>
  );
}

/**
 * One editable value, bounded by what the core says it accepts.
 *
 * The `min` and `max` come from the model, so the control cannot disagree with
 * what the core will accept. A field with no bounds is the System Exclusive
 * payload, which is bytes rather than a number and gets a text entry.
 */
function ValueField({ field }: { field: FieldDto }) {
  const draft = useSendStore((state) => state.drafts[field.id]);
  const setDraft = useSendStore((state) => state.setDraft);
  const clearDraft = useSendStore((state) => state.clearDraft);

  const shown = draft ?? field.value;

  const commit = () => {
    if (draft === undefined || draft === field.value) {
      clearDraft(field.id);
      return;
    }
    void setCompositionField(field.id, draft).then(() => clearDraft(field.id));
  };

  const numeric = field.min !== null && field.max !== null;

  return (
    <>
      <label
        htmlFor={`send-field-${field.id}`}
        className="text-[13px] text-(--color-ink-soft)"
      >
        {field.label}
        {numeric && (
          <span className="pl-1 text-(--color-ink-faint)">
            ({field.min}–{field.max})
          </span>
        )}
      </label>
      <input
        id={`send-field-${field.id}`}
        type={numeric ? "number" : "text"}
        min={field.min ?? undefined}
        max={field.max ?? undefined}
        value={shown}
        onChange={(event) => setDraft(field.id, event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            commit();
          }
        }}
        className="w-full rounded border border-(--color-chrome-border) bg-(--color-control) px-2 py-1 font-mono text-[13px] text-(--color-ink)"
      />
    </>
  );
}
