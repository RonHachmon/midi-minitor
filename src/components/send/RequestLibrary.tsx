import { deleteRequest, renameRequest, saveRequest, selectRequest } from "../../ipc";
import { useSendStore } from "../../sendStore";

/**
 * The ready-made requests, and the ones the user saved beside them.
 *
 * # Why a built-in offers no delete control
 *
 * Not because the control would fail — it would, and the core says so — but
 * because offering an action that can only be refused is the operable-but-inert
 * control the specification rules out. Whether an entry may be changed is a fact
 * the core sends (`builtIn`), not something this component works out.
 *
 * # Why picking a request does not send it
 *
 * Its values stay adjustable after it is loaded, which is the point of a request
 * carrying defaults rather than fixed bytes. Pick, adjust if you want to, send:
 * two clicks when you do not.
 */
export function RequestLibrary() {
  const view = useSendStore((state) => state.view);
  const saveName = useSendStore((state) => state.saveName);
  const setSaveName = useSendStore((state) => state.setSaveName);

  if (view === null) {
    return null;
  }

  const submit = () => {
    if (saveName.trim() === "") {
      return;
    }
    void saveRequest(saveName).then((failure) => {
      if (failure === null) {
        setSaveName("");
      }
    });
  };

  return (
    <section className="flex min-h-0 flex-1 flex-col border-b border-(--color-hairline)">
      <h2 className="shrink-0 px-3 pt-2 pb-1 text-[13px] font-semibold text-(--color-ink)">
        Requests
      </h2>

      <ul className="min-h-0 flex-1 overflow-y-auto border-y border-(--color-hairline) bg-(--color-list)">
        {view.requests.map((request) => (
          <li key={request.name} className="rule-row flex items-start gap-2 px-3 py-1.5">
            <button
              type="button"
              onClick={() => void selectRequest(request.name)}
              className="flex-1 text-left"
            >
              <span className="block text-[13px] text-(--color-ink)">{request.name}</span>
              {request.description !== "" && (
                <span className="block text-[12px] text-(--color-ink-soft)">
                  {request.description}
                </span>
              )}
              <code className="block pt-0.5 font-mono text-[12px] tracking-wider text-(--color-ink-faint)">
                {request.preview}
              </code>
            </button>
            {!request.builtIn && (
              <>
                <button
                  type="button"
                  onClick={() => {
                    const renamed = window.prompt(
                      `Rename "${request.name}" to:`,
                      request.name,
                    );
                    if (renamed !== null && renamed !== request.name) {
                      void renameRequest(request.name, renamed);
                    }
                  }}
                  aria-label={`Rename ${request.name}`}
                  className="icon-button px-1.5"
                >
                  ✎
                </button>
                <button
                  type="button"
                  onClick={() => void deleteRequest(request.name)}
                  aria-label={`Delete ${request.name}`}
                  className="icon-button px-1.5"
                >
                  ✕
                </button>
              </>
            )}
          </li>
        ))}
      </ul>

      <div className="flex shrink-0 items-center gap-2 px-3 py-2">
        <input
          type="text"
          value={saveName}
          placeholder="Save this message as…"
          onChange={(event) => setSaveName(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              submit();
            }
          }}
          aria-label="Name for the saved request"
          className="min-w-0 flex-1 rounded border border-(--color-chrome-border) bg-(--color-control) px-2 py-1 text-[13px] text-(--color-ink)"
        />
        <button
          type="button"
          onClick={submit}
          disabled={saveName.trim() === ""}
          className="chrome-button px-3 py-1 text-[13px]"
        >
          Save
        </button>
      </div>
    </section>
  );
}
