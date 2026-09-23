import { useEffect } from "react";
import { DisplayTab } from "../components/settings/DisplayTab";
import { UnavailableTab } from "../components/settings/UnavailableTab";
import { loadDisplayView } from "../ipc";
import { currentTab, useSettingsStore } from "../settingsStore";

/**
 * The preferences surface: three tabs, of which `Display` is implemented.
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
 * The tab the application mark is shown on.
 *
 * Matched against the id the core supplies rather than against a position, so
 * reordering the tabs there cannot silently move the artwork to another one.
 */
const SOURCES_TAB = "sources";

export function SettingsScreen() {
  const view = useSettingsStore((state) => state.view);
  const tab = useSettingsStore((state) => state.tab);
  const setTab = useSettingsStore((state) => state.setTab);
  const message = useSettingsStore((state) => state.message);

  useEffect(() => void loadDisplayView(), []);

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

      {showing?.available === true ? (
        <DisplayTab view={view} />
      ) : (
        <UnavailableTab
          note={showing?.unavailableNote ?? ""}
          mark={showing?.id === SOURCES_TAB}
        />
      )}
    </div>
  );
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
