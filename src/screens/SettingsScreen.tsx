import { useEffect } from "react";
import type {
  DisplayViewDto,
  OtherViewDto,
  PreferencesTabDto,
} from "../bindings";
import { DisplayTab } from "../components/settings/DisplayTab";
import { OtherTab } from "../components/settings/OtherTab";
import { UnavailableTab } from "../components/settings/UnavailableTab";
import { loadDisplayView, loadOtherView } from "../ipc";
import { currentTab, useSettingsStore } from "../settingsStore";

/**
 * The preferences surface: three tabs, of which `Display` and `Other` are
 * implemented.
 *
 * # Two recorded deviations from the reference image
 *
 * **The window.** `screenshots/setting.jpg` is a macOS preferences *window*,
 * with its own title bar reading `MIDI Monitor Preferences` and its own close,
 * minimise, and zoom controls. This is a third mode of the one window instead,
 * reached from the same strip that already switches between `Monitor` and
 * `Send`. The user asked for a settings *mode*, and native window chrome is the
 * platform detail the constitution explicitly permits deviating on. Everything
 * inside the window is reproduced; the chrome around it is not.
 *
 * **The selected tab's colour.** The reference fills the active tab with the
 * macOS accent blue, which is that platform's segmented control drawing itself.
 * This uses the same pressed face the `Monitor` and `Send` switches already
 * use, so the two strips in one window do not disagree about what "this one is
 * active" looks like. The *state* is reproduced — the active tab is plainly
 * distinguishable — while the exact fill follows the house control style.
 *
 * # Why the model is read on every mount
 *
 * Leaving and returning is free, and the settings may have been written by
 * another path since. Re-reading costs one command and removes a class of stale
 * display entirely, which is worth more than the round trip saves.
 */
/**
 * The tab ids, matched against what the core supplies rather than against a
 * position, so reordering the tabs there cannot silently move a panel — or the
 * application mark — onto a different one.
 */
const DISPLAY_TAB = "display";
/** The tab the application mark is shown on. */
const SOURCES_TAB = "sources";
/** The tab the theme picker is on. */
const OTHER_TAB = "other";

export function SettingsScreen() {
  const view = useSettingsStore((state) => state.view);
  const tab = useSettingsStore((state) => state.tab);
  const setTab = useSettingsStore((state) => state.setTab);
  const message = useSettingsStore((state) => state.message);
  const otherView = useSettingsStore((state) => state.otherView);

  useEffect(() => {
    void loadDisplayView();
    void loadOtherView();
  }, []);

  if (view === null) {
    // The honest state before the core has answered: the screen has asked and is
    // waiting, which is not the same as having no settings to show.
    return (
      <div className="flex-1 overflow-auto bg-(--color-chrome)">
        {message !== null && <Message text={message} />}
      </div>
    );
  }

  const showing = currentTab(view, tab);

  return (
    <div className="flex-1 overflow-auto bg-(--color-chrome)">
      {/* The tab bar, in the reference image's order, with `Display` first. */}
      <div
        role="tablist"
        aria-label="Preferences"
        className="flex justify-center gap-1 px-4 pt-3 pb-1"
      >
        {view.tabs.map((entry) => (
          <button
            key={entry.id}
            type="button"
            role="tab"
            aria-selected={showing?.id === entry.id}
            onClick={() => setTab(entry.id)}
            className="chrome-button px-4 py-1 text-[13px]"
          >
            {entry.label}
          </button>
        ))}
      </div>

      {message !== null && <Message text={message} />}

      <TabBody showing={showing} view={view} otherView={otherView} />
    </div>
  );
}

/**
 * The panel for the tab currently showing.
 *
 * # Why this dispatches on the id rather than on the availability flag
 *
 * With one implemented tab, `available` was enough to choose between a panel and
 * an apology. With two it says only that *some* panel exists — which one is the
 * tab's own identity. Matching on the id means a third tab is a case here rather
 * than a restructuring.
 *
 * A tab the core marks available that this build has no panel for renders
 * nothing. The core has no apology to offer for it — `unavailableNote` is null
 * precisely because it believes the tab works — and inventing a sentence here
 * would be the webview wording a control's state, which is the one thing this
 * screen does not do. The strip still works, so the user is one click from
 * somewhere real.
 */
function TabBody({
  showing,
  view,
  otherView,
}: {
  showing: PreferencesTabDto | undefined;
  view: DisplayViewDto;
  otherView: OtherViewDto | null;
}) {
  if (showing?.available !== true) {
    return (
      <UnavailableTab
        note={showing?.unavailableNote ?? ""}
        mark={showing?.id === SOURCES_TAB}
      />
    );
  }

  switch (showing.id) {
    case DISPLAY_TAB:
      return <DisplayTab view={view} />;
    case OTHER_TAB:
      // `null` while the read is in flight, which is the honest state: the
      // screen has asked and not been answered. Normally it is already filled,
      // because the window reads this at startup to paint itself.
      return otherView === null ? null : <OtherTab view={otherView} />;
    default:
      return null;
  }
}

/** A failure shown beside the controls rather than in the window banner. */
function Message({ text }: { text: string }) {
  return (
    <p
      role="status"
      className="mx-4 rounded-[4px] bg-(--color-danger-tint) px-2 py-1 text-[13px] text-(--color-danger)"
    >
      {text}
    </p>
  );
}
