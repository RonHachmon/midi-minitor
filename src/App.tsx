import { useEffect } from "react";
import { DisclosureSection } from "./components/DisclosureSection";
import { EventTable } from "./components/EventTable";
import { FilterPanel } from "./components/FilterPanel";
import { RetentionRow } from "./components/RetentionRow";
import { SourcesPanel } from "./components/SourcesPanel";
import { startStream, subscribeCatalogue } from "./ipc";
import { useMonitorStore } from "./store";

/**
 * The monitor window.
 *
 * Vertical order follows `screenshots/main-screen.png`: the `Sources` and
 * `Filter` disclosure sections, the retention row with `Clear`, then the event
 * table filling whatever height remains. Expanding a section shrinks the table
 * rather than resizing the window.
 *
 * # What this banner is for, now that rule errors are not in it
 *
 * A failure the user cannot tie to a control they just used — settings that would
 * not save, a source that has gone, a poisoned lock — has nowhere better to go
 * than here. A failure they *can* tie to a control belongs at that control, which
 * is why a refused filter rule is explained inside the Filter panel instead. The
 * split is the point: this strip should be rare enough to be worth reading.
 */
export function App() {
  const error = useMonitorStore((state) => state.error);
  const setError = useMonitorStore((state) => state.setError);

  useEffect(() => startStream(), []);

  // Separate from the event stream: this fires when someone plugs or unplugs a
  // device, not when a message arrives. Subscribing returns the current
  // catalogue too, so there is no moment where the panel is subscribed but
  // empty.
  useEffect(() => void subscribeCatalogue(), []);

  return (
    <div className="flex h-full flex-col bg-(--color-chrome)">
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
    </div>
  );
}
