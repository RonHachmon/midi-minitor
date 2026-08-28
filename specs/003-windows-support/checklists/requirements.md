# Specification Quality Checklist: Windows Support

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-08-28
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

## Validation Notes

**Iteration 1 findings and how they were resolved:**

1. *The two at-risk capabilities must be decided, not deferred.* The user input required an explicit
   decision for each. Resolved by a dedicated **Platform Capability Decisions** section placed before
   the user stories, stating for each capability what a Windows user is promised, what they are not
   promised, where the application says so, and which alternatives were rejected and why. Both
   decisions are then carried into testable requirements (FR-031 to FR-036 for Decision 1, FR-037 to
   FR-041 for Decision 2) and into acceptance scenarios (US5, US6).

2. *"Says so on the control" was initially a principle without a requirement.* Resolved by FR-042 to
   FR-044, which make the honesty rule itself testable — including FR-044, which forbids showing an
   unavailability statement on a platform where the capability works, so the rule cannot be satisfied
   by blanket disclaimers.

3. *macOS non-regression risked being an aspiration.* Resolved by promoting it to its own P1 user
   story (US3) with acceptance scenarios that re-run the previous feature's scenarios, plus FR-049,
   FR-050, and SC-007.

4. *Saved-selection identity on Windows had an unhandled failure mode.* Windows is assumed to expose a
   stable per-port identity, but the spec must not fail silently if it does not for some port.
   Resolved by FR-020 and a matching edge case: match on name, and where two present ports share that
   name with nothing else to distinguish them, apply the selection to neither and say so rather than
   guessing one.

5. *Platform naming vs. implementation detail.* The spec names operating systems (macOS, Windows) and
   observable platform behaviour, because the feature is about platform differences and the user framed
   it that way. It names no library, API, crate, or system call. Checked and clean.

**Remaining open items**: none. The spec is ready for `/speckit-plan`.

**Note for planning**: two assumptions are load-bearing and worth confirming first in research —
that Windows exposes a per-port identity stable across replug and reboot (FR-017), and that no Windows
delivery path exposes the wire byte stream without discarding malformed bytes (Decision 2). The spec
states the fallback for the first and the revisit condition for the second, so neither blocks planning.
