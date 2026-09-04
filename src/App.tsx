import { useEffect } from "react";
import { MonitorScreen } from "./screens/MonitorScreen";
import { SendScreen } from "./screens/SendScreen";
import { startStream, subscribeCatalogue } from "./ipc";
import { useSendStore, type Screen } from "./sendStore";

/**
 * The window, and the switch between its two screens.
 *
 * # Why the subscriptions live here rather than in the monitor screen
 *
 * They must outlive whichever screen is showing. Moving to the send screen
 * unmounts the monitor, and if the event stream were subscribed there, that would
 * silently stop the application taking in traffic — the opposite of the promise
 * that using one screen does not disturb the other. Held at this level, the
 * stream is opened once when the window opens and never torn down by navigation.
 *
 * The monitor's own state — retained events, selections, filters, whether it is
 * paused — is not at risk either way: it lives in Rust, so unmounting a component
 * loses none of it.
 *
 * # Why the switcher is new surface rather than a change to old surface
 *
 * The reference images are the design authority for the monitor, and this feature
 * adds capability rather than editing it. A strip above the window relabels
 * nothing, reorders nothing, and restyles nothing the screenshots depict; every
 * control they show keeps its label, its type, its position, and its default.
 */
export function App() {
  const screen = useSendStore((state) => state.screen);
  const setScreen = useSendStore((state) => state.setScreen);

  useEffect(() => startStream(), []);

  // Separate from the event stream: this fires when someone plugs or unplugs a
  // device, not when a message arrives. Subscribing returns the current
  // catalogue too, so there is no moment where the panel is subscribed but
  // empty.
  useEffect(() => void subscribeCatalogue(), []);

  return (
    <div className="flex h-full flex-col bg-(--color-chrome)">
      <nav
        aria-label="Screen"
        className="flex shrink-0 gap-1 border-b border-(--color-chrome-border) px-2 pt-2 pb-1.5"
      >
        <ScreenTab screen="monitor" label="Monitor" current={screen} onPick={setScreen} />
        <ScreenTab screen="send" label="Send" current={screen} onPick={setScreen} />
      </nav>

      {screen === "monitor" ? <MonitorScreen /> : <SendScreen />}
    </div>
  );
}

/** One screen button, pressed when it is the screen being shown. */
function ScreenTab({
  screen,
  label,
  current,
  onPick,
}: {
  screen: Screen;
  label: string;
  current: Screen;
  onPick: (screen: Screen) => void;
}) {
  return (
    <button
      type="button"
      // `aria-pressed` rather than a role and a selected state: this is a pair of
      // toggles, and the existing `chrome-button` styling already answers to it,
      // so the pressed look comes from the stylesheet the rest of the window uses.
      aria-pressed={current === screen}
      onClick={() => onPick(screen)}
      className="chrome-button px-3 py-1 text-[13px]"
    >
      {label}
    </button>
  );
}
