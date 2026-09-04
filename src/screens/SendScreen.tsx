import { useEffect } from "react";
import { BytePreview } from "../components/send/BytePreview";
import { Composer } from "../components/send/Composer";
import { PublishRow } from "../components/send/PublishRow";
import { RawEntry } from "../components/send/RawEntry";
import { RequestLibrary } from "../components/send/RequestLibrary";
import { SendLog } from "../components/send/SendLog";
import { TargetPicker } from "../components/send/TargetPicker";
import { loadSendView, send, subscribeSendTargets } from "../ipc";
import { useSendStore } from "../sendStore";

/**
 * The send screen.
 *
 * Two columns: what to send on the left, and where it goes plus what already went
 * on the right. The order down the left is the order a first-time user needs it —
 * pick something ready-made, adjust it if you want to, see what it will send.
 *
 * # Why the model is re-read on every mount
 *
 * Devices come and go while the other screen is showing, and this screen's own
 * subscription only starts here. Reading once on mount closes the gap between
 * what the core knows and what the screen was last told.
 *
 * The composition itself is *not* re-read into a fresh state — it lives in Rust,
 * so what the user was building is still there. That is FR-019 satisfied by where
 * the state lives rather than by anything this component does.
 *
 * # Why the outcome message sits here rather than in the window's banner
 *
 * The same split the Filter panel already makes: a failure the user can tie to
 * the control they just used belongs beside that control. Everything on this
 * screen is attributable — they pressed send, chose a target, typed a name.
 */
export function SendScreen() {
  const view = useSendStore((state) => state.view);
  const message = useSendStore((state) => state.message);
  const setMessage = useSendStore((state) => state.setMessage);

  useEffect(() => void loadSendView(), []);
  useEffect(() => void subscribeSendTargets(), []);

  const canSend = view !== null && view.targets.chosen !== null;

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex min-h-0 flex-1">
        <div className="flex min-h-0 w-1/2 flex-col border-r border-(--color-hairline)">
          <Composer />
          <RawEntry />
          <BytePreview />
          <div className="shrink-0 px-3 pb-3">
            <button
              type="button"
              onClick={() => void send()}
              disabled={!canSend}
              className="chrome-button w-full px-3 py-1.5 text-[13px]"
            >
              Send
            </button>
            {!canSend && view !== null && (
              <p className="pt-1 text-[13px] text-(--color-ink-soft)">
                Choose where to send before sending.
              </p>
            )}
          </div>
        </div>

        <div className="flex min-h-0 w-1/2 flex-col">
          <TargetPicker />
          <PublishRow />
          <RequestLibrary />
          <SendLog />
        </div>
      </div>

      {message !== null && (
        <div
          role="alert"
          className="flex shrink-0 items-center gap-2 border-t border-(--color-hairline) bg-(--color-danger-tint) px-3 py-1.5 text-[13px] text-(--color-danger)"
        >
          <span className="flex-1">{message}</span>
          <button
            type="button"
            onClick={() => setMessage(null)}
            aria-label="Dismiss"
            className="icon-button px-1.5"
          >
            ✕
          </button>
        </div>
      )}
    </div>
  );
}
