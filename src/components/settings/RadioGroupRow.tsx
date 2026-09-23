import type { DisplayGroupDto } from "../../bindings";

/**
 * One labelled row of the `Display` tab: a right-aligned label and its radios.
 *
 * # Why this component knows no labels
 *
 * Every string it renders arrives in `group`, from Rust. `screenshots/setting.jpg`
 * is the design authority and its strings are normative content, so a literal
 * here would be a second copy that could drift from the image — and the strings
 * in question are exactly the fragile kind: `Note (Middle C = C3)` carries
 * parentheses and an equals sign, and `1 – 128 (Standard)` carries an **en
 * dash**, which any well-meaning edit would normalise to a hyphen.
 *
 * # Why these are real radio inputs
 *
 * The reference image shows radio buttons, and the constitution has control
 * types reproduced as shown — a radio pair stays a radio pair, not a segmented
 * control or a switch. Real inputs sharing a `name` also give the group keyboard
 * behaviour and screen-reader semantics for free, which a div-based imitation
 * would have to reimplement and would get wrong.
 */
export function RadioGroupRow({
  group,
  onPick,
}: {
  group: DisplayGroupDto;
  onPick: (optionId: string) => void;
}) {
  return (
    <div className="flex items-start gap-3 py-1.5">
      {/* Right-aligned, as the reference image sets every group label. The fixed
          width is what makes the five labels share one right edge. */}
      <div className="w-[7.5rem] shrink-0 pt-px text-right text-[13px] leading-5">
        <div>{group.label}</div>
        {group.secondLine !== null && <div>{group.secondLine}</div>}
      </div>

      <div
        role="radiogroup"
        aria-label={group.label}
        className="flex flex-col gap-0.5"
      >
        {group.options.map((option) => (
          <label
            key={option.id}
            className="flex items-center gap-1.5 text-[13px] leading-5"
          >
            <input
              type="radio"
              name={group.id}
              checked={option.selected}
              onChange={() => onPick(option.id)}
            />
            {option.label}
          </label>
        ))}
      </div>
    </div>
  );
}
