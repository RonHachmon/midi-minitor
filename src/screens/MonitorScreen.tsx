import { DisclosureSection } from "../components/DisclosureSection";
import { EventTable } from "../components/EventTable";
import { FilterPanel } from "../components/FilterPanel";
import { RetentionRow } from "../components/RetentionRow";
import { SourcesPanel } from "../components/SourcesPanel";
import { useMonitorStore } from "../store";

/**
 * The monitor screen.
 *
 * Vertical order follows `screenshots/main-screen.png`: the `Sources` and
 * `Filter` disclosure sections, the retention row with `Clear`, then the event
 * table filling whatever height remains. Expanding a section shrinks the table
 * rather than resizing the window.
 *
 * # Why this moved out of `App` unchanged
 *
 * The window gained a second screen, so the monitor needed to become one of two
 * things a switcher can show rather than the whole of the window. Nothing about
 * it changed in the move: the same components, in the same order, with the same
 * labels and the same default states. The reference images are the design
 * authority for this surface, and a refactor is not a licence to touch it.
 *
 * # What stayed in `App`, and why it matters
 *
 * The event subscription and the catalogue subscription. They live one level up
 * so that switching screens cannot unsubscribe them — which is most of "the two
 * screens do not disturb each other" handled by placement rather than by logic.
 * Monitoring state lives in Rust in any case, so unmounting this component loses
 * nothing.
 *
 * # What this banner is for, now that rule errors are not in it
 *
 * A failure the user cannot tie to a control they just used — settings that would
 * not save, a source that has gone, a poisoned lock — has nowhere better to go
 * than here. A failure they *can* tie to a control belongs at that control, which
 * is why a refused filter rule is explained inside the Filter panel instead. The
 * split is the point: this strip should be rare enough to be worth reading.
 */
export function MonitorScreen() {
  const error = useMonitorStore((state) => state.error);
  const setError = useMonitorStore((state) => state.setError);

  return (
    <>
      <div className="shrink-0 pt-1.5">
        <DisclosureSection title="Sources">
          <SourcesPanel />
        </DisclosureSection>
        <DisclosureSection title="Filter">
          <FilterPanel />
        </DisclosureSection>
        <RetentionRow />
      </div>

      {error === null ? null : (
        <div
          role="alert"
          className="flex shrink-0 items-center gap-2 border-t border-(--color-hairline) bg-(--color-danger-tint) px-3 py-1.5 text-[13px] text-(--color-danger)"
        >
          <span className="flex-1">{error}</span>
          <button
            type="button"
            onClick={() => setError(null)}
            aria-label="Dismiss"
            className="icon-button px-1.5"
          >
            ✕
          </button>
        </div>
      )}

      <EventTable />
    </>
  );
}
