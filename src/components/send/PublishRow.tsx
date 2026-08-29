import { setPublicationEnabled, setPublicationName } from "../../ipc";
import { useSendStore } from "../../sendStore";

/**
 * The disguise: a MIDI source other programs list and can receive from.
 *
 * # Why this component knows nothing about which platform it is on
 *
 * It renders `support`, which the adapter supplied. macOS can publish a source;
 * Windows cannot without a system-wide driver. Neither fact is written here, and
 * neither is written anywhere above the adapter — the platform speaks for itself
 * and every layer between renders what it is told. A `navigator.platform` check
 * in this file would undo two features' worth of that arrangement.
 *
 * Where the platform cannot publish, the control is **disabled**, not merely
 * failing. A control that looks live and does nothing is worse than one that
 * explains itself before it is tried.
 *
 * # Why renaming asks first
 *
 * The receiving program remembers its own settings against the name it saw.
 * Changing it presents that program with a new, unconfigured device — so the user
 * finds out before their mapping stops working, rather than afterwards.
 */
export function PublishRow() {
  const view = useSendStore((state) => state.view);
  const publishName = useSendStore((state) => state.publishName);
  const setPublishName = useSendStore((state) => state.setPublishName);
  const setMessage = useSendStore((state) => state.setMessage);

  if (view === null) {
    return null;
  }

  const { name, published, support, error } = view.publication;
  const unsupported = support.type === "unsupported";

  const rename = () => {
    if (publishName.trim() === "" || publishName === name) {
      return;
    }
    if (published) {
      const proceed = window.confirm(
        `Rename the published source to "${publishName}"?\n\n` +
          `A program receiving from "${name}" keeps its settings against that name. ` +
          `It will see this device disappear and a new one appear, and any mapping you made ` +
          `will need making again.`,
      );
      if (!proceed) {
        setPublishName(name);
        return;
      }
    }
    void setPublicationName(publishName);
  };

  return (
    <section className="border-b border-(--color-hairline) px-3 py-2">
      <h2 className="pb-1 text-[13px] font-semibold text-(--color-ink)">
        Appear as a MIDI source
      </h2>

      <label className="flex items-center gap-2 text-[13px] text-(--color-ink)">
        <input
          type="checkbox"
          checked={published}
          disabled={unsupported}
          onChange={(event) => void setPublicationEnabled(event.target.checked)}
        />
        <span className={unsupported ? "text-(--color-ink-faint)" : undefined}>
          Publish a source other programs can receive from
        </span>
      </label>

      {unsupported ? (
        <p className="pt-1 text-[13px] text-(--color-ink-soft)">
          {support.type === "unsupported" ? support.data.detail : null}
        </p>
      ) : (
        <div className="flex items-center gap-2 pt-2">
          <label
            htmlFor="publish-name"
            className="shrink-0 text-[13px] text-(--color-ink-soft)"
          >
            Name
          </label>
          <input
            id="publish-name"
            type="text"
            value={publishName}
            onChange={(event) => setPublishName(event.target.value)}
            onBlur={rename}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                rename();
              }
            }}
            className="min-w-0 flex-1 rounded border border-(--color-chrome-border) bg-(--color-control) px-2 py-1 text-[13px] text-(--color-ink)"
          />
        </div>
      )}

      {error !== null && (
        <p
          role="alert"
          className="pt-1 text-[13px] text-(--color-danger)"
          onClick={() => setMessage(null)}
        >
          {error}
        </p>
      )}
    </section>
  );
}
