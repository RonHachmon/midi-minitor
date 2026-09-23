import { create } from "zustand";
import type { DisplayViewDto, PreferencesTabDto } from "./bindings";

/**
 * Presentation state for the preferences screen.
 *
 * # Why this holds no labels and no rules
 *
 * Every string the `Display` tab shows — the group labels, the radio labels, the
 * checkbox, the three explanatory lines, the tab names — arrives in `view` from
 * Rust. `screenshots/setting.jpg` is the design authority, and those strings are
 * its normative content, so a copy of them here would be a second source that
 * could drift from the image. Which options exist, which is selected, and which
 * tabs are usable are all decided in the core for the same reason.
 *
 * What is left is genuinely presentational: which tab the user is looking at.
 *
 * # Why the tab choice is not persisted
 *
 * It is a property of the moment, not a setting. Reopening the preferences on
 * the tab that is actually implemented is the useful default every time, and
 * restoring `Other` across a restart would be restoring the user to a dead end.
 */

/** Which preferences tab is showing. */
export type PreferencesTab = string;

/** What the preferences screen renders and remembers. */
export interface SettingsState {
  /**
   * The core's model of the `Display` tab, or `null` before it has been read.
   *
   * `null` is the honest state on first mount: the screen has asked and not yet
   * been answered, which is different from having no settings.
   */
  view: DisplayViewDto | null;
  /** Which tab the user is looking at. */
  tab: PreferencesTab;
  /**
   * A failure that belongs beside the controls rather than in the window banner.
   *
   * The same split the Filter panel and the send screen already make: a failure
   * the user can tie to the control they just used is shown at that control.
   * Changing a display setting is always attributable — they just clicked it.
   */
  message: string | null;

  /** Replaces the model with what the core just returned. */
  applyView: (view: DisplayViewDto) => void;
  /** Shows one of the three tabs. */
  setTab: (tab: PreferencesTab) => void;
  /** Shows or clears the message beside the controls. */
  setMessage: (message: string | null) => void;
}

/** The tab shown first, and the only one this version implements. */
const DEFAULT_TAB = "display";

export const useSettingsStore = create<SettingsState>((set) => ({
  view: null,
  tab: DEFAULT_TAB,
  message: null,

  applyView: (view) => set({ view, message: null }),
  setTab: (tab) => set({ tab }),
  setMessage: (message) => set({ message }),
}));

/**
 * The tab the model says is showing, or the first one, if the chosen id is gone.
 *
 * Guards a narrow case with an unpleasant outcome: a tab id held in this store
 * that the core no longer lists would otherwise render an empty panel with no
 * way back. Falling through to the first tab always lands somewhere real.
 */
export function currentTab(
  view: DisplayViewDto,
  tab: PreferencesTab,
): PreferencesTabDto | undefined {
  return view.tabs.find((entry) => entry.id === tab) ?? view.tabs[0];
}
