import { useState, type ReactNode } from "react";

/** Props for {@link DisclosureSection}. */
export interface DisclosureSectionProps {
  /** The heading text, shown beside the triangle. */
  title: string;
  /** What the section reveals when expanded. */
  children: ReactNode;
}

/**
 * A collapsible section headed by a disclosure triangle.
 *
 * # Why a triangle and not an accordion or a tab
 *
 * The reference screenshots show macOS disclosure triangles — `▶` collapsed,
 * `▼` expanded — and the screenshots are this project's design authority. A
 * component library's accordion would bring its own chevron, border, and
 * animation, all of which would be visible departures from the image.
 *
 * Both sections start collapsed, matching the reference main window.
 */
export function DisclosureSection({ title, children }: DisclosureSectionProps) {
  const [expanded, setExpanded] = useState(false);

  return (
    <section>
      <button
        type="button"
        aria-expanded={expanded}
        onClick={() => setExpanded((open) => !open)}
        className="flex items-center gap-1 px-3 py-0.5 text-[13px] text-(--color-ink-soft) hover:text-(--color-ink)"
      >
        <span aria-hidden className="inline-block w-3 text-[9px] leading-none">
          {expanded ? "▼" : "▶"}
        </span>
        {title}
      </button>
      {expanded ? <div className="px-3 pb-2">{children}</div> : null}
    </section>
  );
}
