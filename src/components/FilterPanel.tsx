import type { ChannelModeDto, FilterViewDto } from "../bindings";
import { setFilter } from "../ipc";
import { useMonitorStore } from "../store";
import { DataPrefixRules } from "./DataPrefixRules";
import { TriStateCheckbox } from "./TriStateCheckbox";

/** Collects the ids of every currently ticked checkbox. */
function enabledKinds(filter: FilterViewDto): string[] {
  const fromCategories = filter.categories.flatMap((category) =>
    category.kinds.filter((kind) => kind.enabled).map((kind) => kind.id),
  );
  const fromStandalone = filter.standalone
    .filter((kind) => kind.enabled)
    .map((kind) => kind.id);
  return fromCategories.concat(fromStandalone);
}

/**
 * The three-column message filter and the channel radio pair.
 *
 * # Why the panel's contents come from Rust
 *
 * Every label, category, and ordering here is read from the model the core
 * supplies rather than hard-coded in this file. The reference screenshot is the
 * design authority for those strings, and the core is where that authority is
 * recorded — so the interface cannot reword `Aftertouch (Poly)` or invent a
 * category, even by accident.
 */
export function FilterPanel() {
  const filter = useMonitorStore((state) => state.filter);
  if (filter === null) {
    return null;
  }

  const enabled = enabledKinds(filter);

  const toggleKind = (id: string, on: boolean) => {
    const next = on
      ? enabled.concat(id)
      : enabled.filter((kind) => kind !== id);
    void setFilter(next, filter.channelMode);
  };

  const toggleCategory = (categoryId: string, on: boolean) => {
    const category = filter.categories.find((entry) => entry.id === categoryId);
    if (category === undefined) {
      return;
    }
    const ids = category.kinds.map((kind) => kind.id);
    const next = on
      ? Array.from(new Set(enabled.concat(ids)))
      : enabled.filter((kind) => !ids.includes(kind));
    void setFilter(next, filter.channelMode);
  };

  const setChannelMode = (mode: ChannelModeDto) => {
    void setFilter(enabled, mode);
  };

  const oneChannelValue =
    filter.channelMode.type === "oneChannel" ? filter.channelMode.data : 1;

  return (
    <div className="flex flex-col gap-3">
      <div className="grid grid-cols-3 gap-x-6">
        {filter.categories.map((category) => (
          <div key={category.id}>
            <TriStateCheckbox
              state={category.state}
              label={category.label}
              onToggle={(next) => toggleCategory(category.id, next)}
            />
            {category.kinds.map((kind) => (
              <TriStateCheckbox
                key={kind.id}
                state={kind.enabled ? "checked" : "unchecked"}
                label={kind.label}
                indent={1}
                onToggle={(next) => toggleKind(kind.id, next)}
              />
            ))}
            {/* System Exclusive and Invalid sit beneath the third column in the
                reference image rather than in a column of their own. */}
            {category.id === "realTime"
              ? filter.standalone.map((kind) => (
                  <div key={kind.id} className="mt-2">
                    <TriStateCheckbox
                      state={kind.enabled ? "checked" : "unchecked"}
                      label={kind.label}
                      onToggle={(next) => toggleKind(kind.id, next)}
                    />
                  </div>
                ))
              : null}
          </div>
        ))}
      </div>

      <fieldset className="flex flex-col gap-[2px]">
        <legend className="sr-only">Channels</legend>
        <label className="flex items-center gap-1.5 text-[13px] leading-[18px]">
          <input
            type="radio"
            name="channel-mode"
            checked={filter.channelMode.type === "allChannels"}
            onChange={() => setChannelMode({ type: "allChannels" })}
            className="h-[13px] w-[13px] accent-(--color-accent)"
          />
          All Channels
        </label>
        <label className="flex items-center gap-1.5 text-[13px] leading-[18px]">
          <input
            type="radio"
            name="channel-mode"
            checked={filter.channelMode.type === "oneChannel"}
            onChange={() =>
              setChannelMode({ type: "oneChannel", data: oneChannelValue })
            }
            className="h-[13px] w-[13px] accent-(--color-accent)"
          />
          One Channel
          <input
            type="number"
            min={1}
            max={16}
            value={oneChannelValue}
            disabled={filter.channelMode.type !== "oneChannel"}
            onChange={(entry) =>
              setChannelMode({
                type: "oneChannel",
                data: Number(entry.target.value),
              })
            }
            className="w-[44px] rounded-[3px] border border-(--color-chrome-border) bg-(--color-field) px-1 py-[1px] text-[13px] disabled:bg-(--color-header) disabled:text-(--color-ink-faint)"
          />
        </label>
      </fieldset>

      <DataPrefixRules />
    </div>
  );
}
