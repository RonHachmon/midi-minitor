import type { DisplaySettingsDto, DisplayViewDto } from "../../bindings";
import { setDisplaySettings } from "../../ipc";
import { RadioGroupRow } from "./RadioGroupRow";

/**
 * The `Display` tab: five radio groups, a checkbox, and three explanatory lines.
 *
 * # Why a change sends the whole settings object
 *
 * The core takes all six at once and answers with the tab and a re-rendered
 * table in one payload, so there is no interval where the setting has changed
 * and the rows have not. Building that object here means copying the current
 * model and replacing one field, which is what `withOption` does.
 *
 * # Why the option ids are matched rather than parsed
 *
 * Each option carries the wire value it selects, generated from the same serde
 * attribute as the TypeScript union. Assigning it into the settings object needs
 * a cast, because the group is identified at runtime by a string id while the
 * field types are known only at compile time — the alternative, five separate
 * handlers with five hard-coded field names, would restate in TypeScript a
 * structure the core already sends.
 */
export function DisplayTab({ view }: { view: DisplayViewDto }) {
  const apply = (settings: DisplaySettingsDto) => {
    void setDisplaySettings(settings);
  };

  return (
    <div className="mx-auto flex w-fit flex-col gap-1 px-4 py-3">
      {view.groups.map((group) => (
        <RadioGroupRow
          key={group.id}
          group={group}
          onPick={(optionId) =>
            apply(withOption(view.settings, group.id, optionId))
          }
        />
      ))}

      <label className="mt-3 flex items-center gap-1.5 text-[13px] leading-5">
        <input
          type="checkbox"
          checked={view.expertEnabled}
          onChange={(event) =>
            apply({ ...view.settings, expert: event.target.checked })
          }
        />
        {view.expertLabel}
      </label>

      {/* The three lines beneath the checkbox, verbatim and in the reference
          image's order. Indented beneath it, as the image sets them. */}
      <ul className="mt-1 ml-5 list-disc pl-3 text-[11px] leading-[1.45] text-(--color-ink-soft)">
        {view.expertNotes.map((note) => (
          <li key={note}>{note}</li>
        ))}
      </ul>
    </div>
  );
}

/**
 * Copies the settings with one group's choice replaced.
 *
 * The group id names the field it sets — the core and this function agree on
 * five strings, which is the one coupling the runtime-identified structure
 * cannot avoid. An id that matches no field leaves the settings untouched, so a
 * model newer than this build degrades to doing nothing rather than to sending a
 * malformed object.
 */
function withOption(
  settings: DisplaySettingsDto,
  groupId: string,
  optionId: string,
): DisplaySettingsDto {
  switch (groupId) {
    case "time":
      return { ...settings, time: optionId as DisplaySettingsDto["time"] };
    case "note":
      return { ...settings, note: optionId as DisplaySettingsDto["note"] };
    case "controller":
      return {
        ...settings,
        controller: optionId as DisplaySettingsDto["controller"],
      };
    case "data":
      return { ...settings, data: optionId as DisplaySettingsDto["data"] };
    case "program":
      return {
        ...settings,
        program: optionId as DisplaySettingsDto["program"],
      };
    default:
      return settings;
  }
}
