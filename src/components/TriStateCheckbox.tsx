import { useEffect, useRef } from "react";
import type { CheckStateDto } from "../bindings";

/** Props for {@link TriStateCheckbox}. */
export interface TriStateCheckboxProps {
  /** Checked, unchecked, or the indeterminate middle state. */
  state: CheckStateDto;
  /** The label rendered beside the box. */
  label: string;
  /** Extra indentation depth, in nesting levels. */
  indent?: number;
  /** Called with the state the user is asking for. */
  onToggle: (next: boolean) => void;
}

/**
 * A checkbox that can also show a mixed state.
 *
 * # Why the DOM property is set through a ref
 *
 * `indeterminate` is not an HTML attribute — it exists only as a property on the
 * element, so React cannot set it declaratively. Without this effect a group
 * whose children disagree would render as plain unchecked, losing the
 * distinction the reference interface relies on.
 *
 * A mixed box resolves to checked when clicked, which is the platform
 * convention: the user is asking for "all of them", not for another half-state.
 */
export function TriStateCheckbox({
  state,
  label,
  indent = 0,
  onToggle,
}: TriStateCheckboxProps) {
  const box = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (box.current) {
      box.current.indeterminate = state === "mixed";
    }
  }, [state]);

  return (
    <label
      className="flex items-center gap-1.5 py-[1px] text-[13px] leading-[18px]"
      style={{ paddingLeft: `${indent * 18}px` }}
    >
      <input
        ref={box}
        type="checkbox"
        checked={state === "checked"}
        onChange={() => onToggle(state !== "checked")}
        className="h-[13px] w-[13px] shrink-0 accent-(--color-accent)"
      />
      <span className="truncate">{label}</span>
    </label>
  );
}
