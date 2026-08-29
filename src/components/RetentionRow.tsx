import { useEffect, useState } from "react";
import { clearEvents, setCaptureState, setRetentionLimit } from "../ipc";
import { useMonitorStore } from "../store";

/**
 * The `Remember up to [N] events` row, with `Pause` and `Clear`.
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
 *
 * # A recorded deviation: `Pause` sits inside a depicted row
 *
 * `screenshots/data.png` shows this row holding the retention field and `Clear`
 * alone, and the project's constitution has additive capability earn new surface
 * rather than join a depicted row. `Pause` was placed here by an explicit
 * decision of the project's owner, on the grounds that it belongs with the other
 * control that acts on the retained list. The deviation is bounded: `Clear` keeps
 * its label, its size, its style, and its position at the end of the row, and
 * `Pause` sits before it rather than displacing it. Nothing else the screenshots
 * depict is touched.
 *
 * # Why the button carries a title and no sentence beside it
 *
 * Pausing stops the monitor *recording*, so traffic arriving during a pause is
 * not kept — a real cost, and one worth being able to find. It is on the button's
 * tooltip rather than in a line of text next to it, so the row stays as quiet as
 * the reference image while the fact remains available to anyone who wonders.
 */
export function RetentionRow() {
  const limit = useMonitorStore((state) => state.retentionLimit);
  const captureState = useMonitorStore((state) => state.captureState);
  const paused = captureState.type === "paused";
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
        onClick={() => void setCaptureState({ type: paused ? "running" : "paused" })}
        aria-pressed={paused}
        title={
          paused
            ? "Resume recording. Traffic that passed while paused was not kept."
            : "Stop recording. Events already received are kept; arriving events are not."
        }
        className="chrome-button ml-auto px-5 py-[3px] text-[13px]"
      >
        {paused ? "Resume" : "Pause"}
      </button>
      <button
        type="button"
        onClick={() => void clearEvents()}
        className="chrome-button px-5 py-[3px] text-[13px]"
      >
        Clear
      </button>
    </div>
  );
}
