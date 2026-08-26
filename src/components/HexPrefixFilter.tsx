import { useEffect, useState } from "react";
import { setDataPrefixFilter } from "../ipc";
import { useMonitorStore } from "../store";

/**
 * Includes or excludes events by a hexadecimal prefix of their raw bytes.
 *
 * # Why the entry is committed rather than applied per keystroke
 *
 * Half a prefix is a different filter from the whole one, so applying on every
 * character would flash the list through states the user never asked for. The
 * value is committed on blur or Enter.
 *
 * A malformed entry is rejected by the core **without changing state**, so the
 * previously applied filter keeps running and the list does not blank out
 * because of a typo. That message arrives through the shared error banner.
 *
 * Prefixes are entered separated by spaces or commas; an event matches if it
 * matches any of them. Matching is on nibbles, so `9` matches `90` through `9F`.
 */
export function HexPrefixFilter() {
  const filter = useMonitorStore((state) => state.filter);
  const applied = filter?.prefixes.join(" ") ?? "";
  const mode = filter?.prefixMode ?? "include";
  const [draft, setDraft] = useState(applied);

  useEffect(() => {
    setDraft(applied);
  }, [applied]);

  const commit = (nextMode: "include" | "exclude") => {
    const prefixes = draft
      .split(/[\s,]+/u)
      .map((entry) => entry.trim())
      .filter((entry) => entry.length > 0);
    void setDataPrefixFilter(nextMode, prefixes);
  };

  return (
    <div className="flex flex-wrap items-center gap-2 border-t border-(--color-chrome-border) pt-2">
      <label className="flex items-center gap-2 text-[13px]">
        Data starts with
        <input
          value={draft}
          onChange={(entry) => setDraft(entry.target.value)}
          onBlur={() => commit(mode)}
          onKeyDown={(key) => {
            if (key.key === "Enter") {
              commit(mode);
            }
          }}
          placeholder="e.g. 90 B007"
          spellCheck={false}
          className="w-[150px] rounded-[3px] border border-(--color-chrome-border) bg-white px-2 py-[2px] font-(family-name:--font-hex) text-[13px] outline-none focus:border-(--color-accent)"
        />
      </label>

      <label className="flex items-center gap-1.5 text-[13px]">
        <input
          type="radio"
          name="prefix-mode"
          checked={mode === "include"}
          onChange={() => commit("include")}
          className="h-[13px] w-[13px] accent-(--color-accent)"
        />
        Show only
      </label>
      <label className="flex items-center gap-1.5 text-[13px]">
        <input
          type="radio"
          name="prefix-mode"
          checked={mode === "exclude"}
          onChange={() => commit("exclude")}
          className="h-[13px] w-[13px] accent-(--color-accent)"
        />
        Hide
      </label>
    </div>
  );
}
