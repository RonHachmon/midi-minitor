import { create } from "zustand";
import type {
  DisplayViewDto,
  OtherViewDto,
  PreferencesTabDto,
} from "./bindings";

/**
 * Presentation state for the preferences screen.
 *
 * # Why this holds no labels and no rules
 *
 * Every string the tabs show — the group labels, the radio labels, the checkbox,
 * the three explanatory lines, the tab names — arrives from Rust.
 * `screenshots/setting.jpg` is the design authority, and those strings are its
 * normative content, so a copy of them here would be a second source that could
 * drift from the image. Which options exist, which is selected, and which tabs
 * are usable are all decided in the core for the same reason. That holds for
 * `Other`'s theme labels too, which no reference image fixes: the core owning
 * them is about who decides an option exists, not about which authority named
 * it.
 *
 * What is left is genuinely presentational: which tab the user is looking at.
 *
 * # Why the tab choice is not persisted
 *
 * It is a property of the moment, not a setting. `Display` is the useful place
 * to reopen on every time — it is where the settings someone came to change
 * almost always are — and which tab happened to be open last is not something
 * the user chose in the sense that the settings on it were.
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
   *
   * It also carries `tabs`, the model of the strip itself, and is the **only**
   * source for which tabs exist — [`SettingsState.otherView`] deliberately does
   * not repeat that list, so the two can never disagree about it.
   */
  view: DisplayViewDto | null;
  /**
   * The core's model of the `Other` tab, or `null` before it has been read.
   *
   * Read at startup as well as on mount, because the window is painted in the
   * theme it reports whichever screen is showing.
   */
  otherView: OtherViewDto | null;
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

  /** Replaces the `Display` model with what the core just returned. */
  applyView: (view: DisplayViewDto) => void;
  /** Replaces the `Other` model with what the core just returned. */
  applyOtherView: (otherView: OtherViewDto) => void;
  /** Shows one of the three tabs. */
  setTab: (tab: PreferencesTab) => void;
  /** Shows or clears the message beside the controls. */
  setMessage: (message: string | null) => void;
}

/** The tab shown first. */
const DEFAULT_TAB = "display";

export const useSettingsStore = create<SettingsState>((set) => ({
  view: null,
  otherView: null,
  tab: DEFAULT_TAB,
  message: null,

  applyView: (view) => set({ view, message: null }),
  applyOtherView: (otherView) => set({ otherView, message: null }),
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
