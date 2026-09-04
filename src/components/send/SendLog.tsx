import { resend } from "../../ipc";
import { useSendStore } from "../../sendStore";

/**
 * What this session has sent, and how each attempt went.
 *
 * # Why a failed send is listed rather than omitted
 *
 * Without it, a send that failed silently and a send that succeeded but was
 * ignored by the receiving program look identical — and telling those two apart
 * is the entire reason to open a monitor. So a failure gets a row, visibly
 * different from a success, carrying the reason.
 *
 * # Why a success says "Sent" and not "Delivered"
 *
 * Sent means the platform accepted the bytes, and nothing more. Whether the
 * program at the other end received them, was configured to listen, or acted on
 * them is unknowable from here. Claiming otherwise would be the most misleading
 * thing this screen could do, because the first thing a real user hits is a
 * device that is listed but not yet mapped.
 */
export function SendLog() {
  const view = useSendStore((state) => state.view);

  if (view === null) {
    return null;
  }

  return (
    <section className="flex min-h-0 flex-1 flex-col">
      <h2 className="shrink-0 px-3 pt-2 pb-1 text-[13px] font-semibold text-(--color-ink)">
        Sent
      </h2>

      {view.records.length === 0 ? (
        <p className="px-3 pb-2 text-[13px] text-(--color-ink-soft)">
          Nothing sent yet this session.
        </p>
      ) : (
        <ul className="min-h-0 flex-1 overflow-y-auto border-t border-(--color-hairline) bg-(--color-list)">
          {[...view.records].reverse().map((record) => (
            <li key={record.id} className="event-row flex items-start gap-2 px-3 py-1.5">
              <div className="min-w-0 flex-1">
                <div className="flex items-baseline gap-2 text-[13px]">
                  <span className="font-mono text-(--color-ink-faint)">{record.time}</span>
                  <span className="text-(--color-ink)">{record.message}</span>
                  <span className="truncate text-(--color-ink-soft)">→ {record.target}</span>
                </div>
                <code className="block font-mono text-[12px] tracking-wider text-(--color-ink-faint)">
                  {record.data}
                </code>
                {record.failure === null ? (
                  <span className="text-[12px] text-(--color-ink-soft)">Sent</span>
                ) : (
                  <span className="text-[12px] text-(--color-danger)">
                    Failed — {record.failure}
                  </span>
                )}
              </div>
              <button
                type="button"
                onClick={() => void resend(record.id)}
                className="chrome-button shrink-0 px-2 py-0.5 text-[12px]"
              >
                Send again
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
