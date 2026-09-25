import type { AppearanceSettingsDto, OtherViewDto } from "../../bindings";
import { setAppearanceSettings } from "../../ipc";
import { RadioGroupRow } from "./RadioGroupRow";

/**
 * The `Other` tab: the theme picker.
 *
 * # Why this shares `RadioGroupRow` with the `Display` tab
 *
 * That component renders a group knowing none of its strings — every label,
 * every option, and which one is selected arrive from Rust. A theme group is
 * that same shape, so this tab is a different model through the same renderer
 * rather than a second radio implementation that could drift from the first in
 * spacing, label alignment, or keyboard behaviour.
 *
 * # Why the container repeats `DisplayTab`'s classes
 *
 * The two tabs sit in one strip and the user moves between them. Different
 * padding would shift the panel as they did, which would read as the window
 * twitching rather than as two tabs of one surface.
 */
export function OtherTab({ view }: { view: OtherViewDto }) {
  return (
    <div className="mx-auto flex w-fit flex-col gap-1 px-4 py-3">
      {view.groups.map((group) => (
        <RadioGroupRow
          key={group.id}
          group={group}
          // This tab has one group, so there is no second label to line up
          // with and the alignment width would only push the row off centre.
          aligned={false}
          onPick={(optionId) =>
            void setAppearanceSettings(
              withOption(view.settings, group.id, optionId),
            )
          }
        />
      ))}
    </div>
  );
}

/**
 * Copies the settings with one group's choice replaced.
 *
 * The same shape as `DisplayTab`'s function of this name, and for the same
 * reason: the group is identified at runtime by a string id while the field
 * types are known only at compile time. An id matching no field leaves the
 * settings untouched, so a model newer than this build degrades to doing nothing
 * rather than to sending a malformed object.
 */
function withOption(
  settings: AppearanceSettingsDto,
  groupId: string,
  optionId: string,
): AppearanceSettingsDto {
  switch (groupId) {
    case "theme":
      return { ...settings, theme: optionId as AppearanceSettingsDto["theme"] };
    default:
      return settings;
  }
}
