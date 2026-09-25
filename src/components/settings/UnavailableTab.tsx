import { AppMark } from "./AppMark";

/**
 * What a preferences tab shows when its settings are not built yet.
 *
 * # Why this exists rather than the tab being hidden or disabled
 *
 * `screenshots/setting.jpg` shows three tabs, and the screenshots are the design
 * authority — removing two of them would be a fidelity defect. But the project
 * also holds that a control which cannot do anything is never left
 * operable-but-inert, which is what a tab full of radio buttons that changed
 * nothing would be. Saying so plainly is the only option that keeps both rules:
 * the tab is there, it is reachable, and it tells the truth in one click.
 *
 * # Why the sentence comes from the core
 *
 * The same reason every other string on this screen does. The core decides which
 * tabs are usable, so this component cannot render an operable control for one
 * that is not, and filling `Sources` in later is a change to the model rather
 * than a restructuring of the screen.
 *
 * # Why the mark appears on one tab and not the other
 *
 * It is shown where it was asked for, on `Sources`, and nowhere else. The mark
 * is decoration, and decoration repeated on every empty surface stops reading as
 * a mark and starts reading as filler. [`AppMark`] records why this surface is
 * one the artwork is allowed to occupy at all.
 */
export function UnavailableTab({
  note,
  mark = false,
}: {
  note: string;
  /** Whether to show the application mark above the note. */
  mark?: boolean;
}) {
  return (
    <div className="flex flex-col items-center gap-4 px-4 py-6">
      {mark && <AppMark />}
      <p className="max-w-[24rem] text-center text-[13px] leading-[1.5] text-(--color-ink-soft)">
        {note}
      </p>
    </div>
  );
}
