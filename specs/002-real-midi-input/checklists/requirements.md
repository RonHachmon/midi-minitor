# Specification Quality Checklist: Real MIDI Input

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-08-26
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

**Status**: All items pass. 40 functional requirements, 16 success criteria, 5 prioritized user
stories, no unresolved clarifications. Ready for `/speckit-plan`.

## Notes

### Clarifications resolved by the user (2026-08-26)

1. **`Spy on output to destinations` — deferred to a later feature.** The group stays on screen with
   its verbatim label, empty, stating its own unavailability (FR-028 to FR-030, US5). Deferred rather
   than removed because Principle VI forbids deleting a control the screenshots depict; deferred
   rather than delivered because it needs privileged system support installed outside the application.
   FR-029 forbids any placeholder source or event in the group, which is what keeps the deferral
   compatible with "nothing mocked".
2. **macOS only.** FR-039. No requirement is specified or verified for Windows or Linux. This
   collapses two would-be per-platform matrices into single behaviours: offering a destination for
   other applications (FR-024 to FR-027) and stable port identity for persistence (FR-031).

### Resolved during drafting (recorded in Assumptions rather than asked)

- **Simulator disposition** — "nothing mocked" is read as deletion from the product, not a hidden
  fallback. Enforced by FR-002 and SC-014.
- **Selection identity across sessions** — persistence keys on the most stable identity the system
  offers for a port rather than on list position, since a numeric index changes between sessions and
  between physical connectors. FR-031 to FR-034.
- **Default selection for newly discovered ports** — unselected when saved settings exist, all
  selected on a genuine first launch. Preserves "traffic appears immediately on first launch" without
  letting a newly attached device flood a list the user had narrowed.
- **Input only** — transmitting, echoing, and routing are out of scope; the inbound destination is the
  one exception, and it is already on the reference surface.

### Constitution alignment

- **Principle VI** (screenshots are design authority) — FR-036 and FR-038 freeze the referenced
  surface. New interface elements are confined to states only real hardware creates. The deferred spy
  group is retained rather than deleted specifically to satisfy this principle.
- **Principle VII** (mock foundation built to be extended) — this feature is the substitution the
  `EventSource` port seam was built for. FR-037 requires all previously specified behaviour to work
  unchanged.
- **Verification Standard** — acceptance scenarios are written as manual steps against the running
  application with real hardware attached. No test artifacts are specified.

### Flagged for planning, not blocking

`SourceCatalogue` currently documents itself as fixed at startup with no add or remove. FR-009
(hot-plug) contradicts that directly, so a domain-level change there is likely unavoidable despite the
port seam. This is a planning concern to surface in Complexity Tracking, not a specification gap.
