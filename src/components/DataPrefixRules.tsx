import { useState } from "react";
import type { PrefixModeDto } from "../bindings";
import { addDataPrefixRule, removeDataPrefixRule } from "../ipc";
import { useMonitorStore } from "../store";

/** The interface's own name for each rule kind, used wherever a rule is shown. */
const KIND_LABEL: Record<PrefixModeDto, string> = {
  include: "Show only",
  exclude: "Hide",
};

/**
 * The list of data prefix rules, and the row that adds one.
 *
 * # Why the entry is committed on `Add` rather than applied as it is typed
 *
 * Half a prefix is a different filter from the whole one, so applying on every
 * keystroke would flash the list through states nobody asked for. The row this
 * replaced committed on blur or Enter for the same reason; with a list, `Add` is
 * the commit, and Enter is kept as its shortcut.
 *
 * # Why a refusal is explained here rather than in the window's banner
 *
 * A rejected rule is the one failure in this application the user can attribute
 * exactly: they typed it a second ago and they are still looking at the field.
 * The banner sits below the controls and above the table, far enough away that a
 * clash message there reads as something that happened rather than something to
 * fix. So it is shown under the entry row, in the same place every time, and it
 * clears the moment the entry changes.
 *
 * # Why the field clears only on success
 *
 * A rule can be refused for a typo, for repeating a listed prefix, or for
 * contradicting one. In every case the core changes nothing, so the rules already
 * in force keep running and the event list does not blank out — and the text must
 * stay where the user typed it, because the next thing they will do is correct
 * it. Clearing on success alone gives an empty field for the next rule without
 * ever discarding an entry that still needs fixing.
 *
 * # Why each rule's delete control is faint rather than hidden
 *
 * Revealing it only on hover would look tidier and would leave a first-time user
 * unable to tell that a rule can be removed at all. It stays visible at half
 * strength and comes up to full on the row's hover tint, which points at it
 * without requiring the user to have guessed it was there.
 *
 * # Why the list is rendered from the core's model
 *
 * `filter.rules` comes back from Rust normalised, and its `prefix` is the key the
 * delete control sends back. Nothing here parses, uppercases, or de-duplicates a
 * prefix; doing so would be a second implementation of a rule that has one home.
 */
export function DataPrefixRules() {
  const filter = useMonitorStore((state) => state.filter);
  const [draft, setDraft] = useState("");
  const [kind, setKind] = useState<PrefixModeDto>("include");
  const [refusal, setRefusal] = useState<string | null>(null);

  const rules = filter?.rules ?? [];

  // An empty entry is sent rather than swallowed here: "a prefix cannot be
  // empty" is the core's answer, and refusing silently in this file would both
  // duplicate that rule and leave the user pressing a button that does nothing.
  const add = async () => {
    const message = await addDataPrefixRule(draft, kind);
    setRefusal(message);
    if (message === null) {
      setDraft("");
    }
  };

  const remove = async (prefix: string) => {
    setRefusal(await removeDataPrefixRule(prefix));
  };

  return (
    <div className="flex flex-col gap-2 border-t border-(--color-chrome-border) pt-2">
      <div className="flex flex-wrap items-center gap-2">
        <label className="flex items-center gap-2 text-[13px]">
          Data starts with
          <input
            value={draft}
            onChange={(entry) => {
              setDraft(entry.target.value);
              // The message described the previous entry. Keeping it beside a
              // field the user has since edited would be stale advice.
              setRefusal(null);
            }}
            onKeyDown={(key) => {
              if (key.key === "Enter") {
                void add();
              }
            }}
            placeholder="e.g. 90"
            spellCheck={false}
            aria-invalid={refusal !== null}
            className="w-[150px] rounded-[3px] border border-(--color-chrome-border) bg-white px-2 py-[2px] font-(family-name:--font-hex) text-[13px] outline-none transition-colors focus:border-(--color-accent) aria-[invalid=true]:border-(--color-danger)"
          />
        </label>

        <label className="flex items-center gap-1.5 text-[13px]">
          <input
            type="radio"
            name="prefix-kind"
            checked={kind === "include"}
            onChange={() => setKind("include")}
            className="h-[13px] w-[13px] accent-(--color-accent)"
          />
          Show only
        </label>
        <label className="flex items-center gap-1.5 text-[13px]">
          <input
            type="radio"
            name="prefix-kind"
            checked={kind === "exclude"}
            onChange={() => setKind("exclude")}
            className="h-[13px] w-[13px] accent-(--color-accent)"
          />
          Hide
        </label>

        <button
          type="button"
          onClick={() => void add()}
          className="chrome-button px-4 py-[3px] text-[13px]"
        >
          Add
        </button>
      </div>

      {refusal === null ? null : (
        <p
          role="alert"
          className="rounded-[4px] bg-(--color-danger-tint) px-2 py-1 text-[13px] text-(--color-danger)"
        >
          {refusal}
        </p>
      )}

      {rules.length === 0 ? (
        <p className="text-[13px] text-(--color-ink-faint)">
          No data rules — every message passes.
        </p>
      ) : (
        <ul className="flex max-h-[104px] flex-col gap-[1px] overflow-y-auto">
          {rules.map((rule) => (
            <li
              key={rule.prefix}
              className="rule-row group flex w-fit min-w-[150px] items-center gap-2 px-1.5 py-[1px] text-[13px]"
            >
              <span className="text-(--color-ink-soft)">
                {KIND_LABEL[rule.kind]}
              </span>
              <span className="font-(family-name:--font-hex)">
                {rule.prefix}
              </span>
              <button
                type="button"
                onClick={() => void remove(rule.prefix)}
                aria-label={`Delete ${KIND_LABEL[rule.kind]} ${rule.prefix} rule`}
                title="Delete this rule"
                className="icon-button ml-auto pl-2 opacity-50 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
              >
                ✕
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
