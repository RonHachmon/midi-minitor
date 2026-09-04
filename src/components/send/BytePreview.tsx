import { useSendStore } from "../../sendStore";

/**
 * Exactly what will be transmitted.
 *
 * # Why this component computes nothing
 *
 * The string it renders was produced by the same encoder that produces the bytes
 * the adapter sends. That is the whole mechanism behind the promise that the
 * preview matches the transmission: they are not two calculations that agree,
 * they are one value shown twice. Formatting bytes here — even correctly — would
 * turn a structural guarantee into a coincidence.
 */
export function BytePreview() {
  const view = useSendStore((state) => state.view);

  if (view === null) {
    return null;
  }

  const { preview, previewError } = view.composition;

  return (
    <div className="px-3 py-2">
      <div className="pb-1 text-[13px] font-semibold text-(--color-ink)">Will send</div>
      {previewError === null ? (
        <code className="block rounded border border-(--color-hairline) bg-(--color-list) px-2 py-1.5 font-mono text-[13px] tracking-wider text-(--color-ink)">
          {preview}
        </code>
      ) : (
        <p className="text-[13px] text-(--color-danger)">{previewError}</p>
      )}
    </div>
  );
}
