# Specification Quality Checklist: A Preferences Surface, With Display Formats Implemented

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-09-23
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

All items pass. Two clarifications were raised and resolved by the user on 2026-09-23:

1. **FR-011 — note format default.** `screenshots/setting.jpg` selects `Note (Middle C = C3)`,
   while the monitor renders notes today under the middle-C-equals-C4 convention, documented in
   `crates/midi-core/src/domain/message.rs` as matching `screenshots/data.png`. Two reference
   images pointed different ways and the constitution makes both normative.
   **Resolved: `setting.jpg` wins** (FR-011a). Note names shift down one octave from today's
   default; the superseded justification in `message.rs` must be replaced rather than left to be
   restored as a defect. Both conventions remain selectable (FR-016).

2. **FR-025 — retroactive re-rendering.** Whether a format change redraws retained events or
   applies only to new arrivals. **Resolved: retained events redraw, for all six settings
   including `Time format`** (FR-025, FR-025a). This means an event must carry its arrival
   moment on the host clock from capture, regardless of which time format is selected at the
   time.

Two constitutional consequences for `/speckit-plan` to carry into Complexity Tracking:

- **Principle VI.** FR-011a is a deliberate, user-approved divergence from
  `screenshots/data.png` for note names. It is a conflict between two normative images resolved
  in favour of the newer one, not an unjustified deviation — but it must be recorded as such.
- **Verification Standard.** Any test-oriented phases in the plan and tasks templates are inert
  in this repository and must be recorded as skipped, not silently dropped.
