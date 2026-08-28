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
  /**
   * Whether the box cannot be switched at all.
   *
   * For a capability the platform in use does not have — not for something that
   * is merely unavailable right now. A row whose device another program is
   * holding stays operable on purpose, so that ticking it starts monitoring the
   * moment that program lets go.
   */
  disabled?: boolean;
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
  disabled = false,
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
        disabled={disabled}
        onChange={() => onToggle(state !== "checked")}
        className="h-[13px] w-[13px] shrink-0 accent-(--color-accent)"
      />
      {/*
        The label keeps its verbatim text and its position when disabled — only
        the platform's own disabled rendering changes, which is a difference the
        user already expects. The reason why it is disabled is rendered by the
        caller, next to the row.
      */}
      <span className={`truncate ${disabled ? "text-(--color-ink-faint)" : ""}`}>
        {label}
      </span>
    </label>
  );
}
