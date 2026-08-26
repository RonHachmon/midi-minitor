import { useEffect, useState } from "react";
import { clearEvents, setRetentionLimit } from "../ipc";
import { useMonitorStore } from "../store";

/**
 * The `Remember up to [N] events` row and the `Clear` button.
 *
 * # Why the field holds a draft string
 *
 * A controlled numeric input bound straight to the committed limit cannot be
 * emptied or edited mid-value without the core rejecting each intermediate
 * state. The draft lets the user type freely; the value is committed on blur or
 * Enter, and a rejection restores the last accepted number rather than leaving
 * the field showing something that is not in effect.
 *
 * The labels either side of the field are verbatim from the reference window.
 */
export function RetentionRow() {
  const limit = useMonitorStore((state) => state.retentionLimit);
  const [draft, setDraft] = useState(String(limit));

  useEffect(() => {
    setDraft(String(limit));
  }, [limit]);

  const commit = () => {
    const parsed = Number(draft.trim());
    if (!Number.isInteger(parsed)) {
      useMonitorStore
        .getState()
        .setError("Enter a whole number of events.");
      setDraft(String(limit));
      return;
    }
    void setRetentionLimit(parsed);
  };

  return (
    <div className="flex items-center gap-2 px-3 py-2">
      <label className="flex items-center gap-2 text-[13px]">
        Remember up to
        <input
          value={draft}
          onChange={(entry) => setDraft(entry.target.value)}
          onBlur={commit}
          onKeyDown={(key) => {
            if (key.key === "Enter") {
              commit();
            }
          }}
          inputMode="numeric"
          className="w-[92px] rounded-[3px] border border-(--color-chrome-border) bg-white px-2 py-[2px] text-right text-[13px] outline-none focus:border-(--color-accent)"
        />
        events
      </label>
      <button
        type="button"
        onClick={() => void clearEvents()}
        className="ml-auto rounded-[5px] border border-(--color-chrome-border) bg-white px-5 py-[3px] text-[13px] hover:bg-(--color-header)"
      >
        Clear
      </button>
    </div>
  );
}
