<!--
SYNC IMPACT REPORT
==================
Version change: 1.0.0 → 1.1.0
Bump rationale: MINOR. Two principles added; no existing principle removed, weakened,
or redefined. All 1.0.0 guidance carries forward verbatim.

Added principles:
  VI. The Screenshots Are the Design Authority — reference images in screenshots/ are
    normative for layout, labels, control types, grouping, order, and default states.
  VII. A Mock Foundation Built to Be Extended — the simulated event source is a
    replaceable infrastructure implementation; later features extend rather than edit.

Modified sections:
  Development Workflow & Quality Gates — Definition of Done gains a design-fidelity
    gate and a mock-containment gate; review focus order updated.

Removed sections: none

Principles carried forward unchanged from 1.0.0:
  I. Code Quality Is Non-Negotiable
  II. Reuse Libraries Before Writing Code
  III. Documentation Explains Why
  IV. Layered Architecture With a Typed IPC Contract
  V. Named Patterns, Never Ceremony
  Verification Standard (testing exclusion) — unchanged and still binding.

Templates requiring review:
  .specify/templates/plan-template.md — contains test-oriented phases that conflict
    with the Verification Standard; agents MUST skip them per Governance.
  .specify/templates/tasks-template.md — same; test task categories are inert here.

Downstream artifacts affected:
  specs/001-mock-midi-monitor/spec.md — already consistent with VI and VII; its
    Assumptions section records the same screenshot-fidelity and mock-scope reading.

Follow-up TODOs: none
-->

# MIDI Monitor Constitution

## Core Principles

### I. Code Quality Is Non-Negotiable

Every unit of code MUST be defensible on four counts, in both the Rust core and the
TypeScript webview:

- **Single responsibility.** A module, type, or function has exactly one reason to
  change. A function that both decides and performs MUST be split. A struct that both
  models domain state and marshals transport data MUST be split.
- **Intention-revealing names.** Names state purpose, not mechanism or type.
  `PortId`, not `String`. `drop_stale_events`, not `process`. Abbreviations are
  forbidden unless they are the domain's own vocabulary (`midi`, `sysex`, `cc`).
- **Typed errors as values.** Fallible operations return `Result<T, E>` with a
  purpose-built error enum (`thiserror` in the core, discriminated unions in the
  webview). `unwrap`, `expect`, `panic!`, and thrown strings are forbidden in
  non-`main` code paths. Errors carry the context needed to act on them.
- **No dead code, no magic values.** Unused items are deleted, not commented out or
  `#[allow(dead_code)]`-ed. Every literal that is not `0`, `1`, or `""` MUST be a
  named `const` or configuration field at the boundary that owns its meaning.

**Rationale:** A greenfield project accumulates its permanent habits in its first
weeks. These four rules are cheap to hold from commit one and prohibitively expensive
to retrofit. With no test suite to catch regressions (see Verification Standard),
the type system and clear structure are the only safety net — they MUST carry that
weight.

### II. Reuse Libraries Before Writing Code

Hand-rolled implementations of solved problems are prohibited. Before writing any
non-domain code, the crate or npm ecosystem MUST be searched for an established
solution.

- Prefer a maintained library for: MIDI I/O, serialization, error derivation, logging,
  date/time, async runtime, state management, virtualized lists, and UI primitives.
- A dependency is justified when it is actively maintained, has a compatible license,
  and its API surface fits the layer that consumes it.
- Hand-rolling is permitted only when: no library exists, every candidate pulls in a
  disproportionate dependency tree, or the need is genuinely domain-specific to MIDI
  monitoring. The justification MUST be recorded in the module-level documentation.
- Wrapping a dependency behind a thin project-owned type is encouraged where it keeps
  the dependency out of the domain layer (see Principle IV). Reimplementing it is not.

**Rationale:** Library code is already documented, already hardened by real users, and
already free. Time spent re-solving MIDI parsing or virtual scrolling is time not spent
on the monitor itself, and the reimplementation will be the buggiest code in the repo.

### III. Documentation Explains Why

Documentation coverage is a completion criterion, not a follow-up task.

- **Every public type and every public method MUST carry a doc comment** — rustdoc
  (`///`) on the Rust side, TSDoc (`/** */`) on the TypeScript side. A `pub` item
  without a doc comment is an incomplete item.
- **Every module MUST carry a module-level doc** — `//!` in Rust, a leading TSDoc
  block in TypeScript — stating the module's responsibility, its place in the layering,
  and the reasoning behind any non-obvious dependency or design choice.
- **Docs explain why, never what.** `/// Returns the port name` restates the signature
  and is a violation. `/// Names come from the OS and are not stable across replug, so
  callers MUST key on PortId instead` is documentation. Record intent, constraints,
  invariants, failure modes, and rejected alternatives.
- Every error variant MUST document the condition that produces it and what a caller
  can do about it.

**Rationale:** The signature already says what. Only the author knows why — which
constraint forced this shape, which simpler approach failed, which invariant the caller
must not break. That knowledge is the part that gets lost, and here it is the primary
substitute for a test suite as executable-intent documentation.

### IV. Layered Architecture With a Typed IPC Contract

The application is layered, and the Rust core / webview boundary is the sharpest seam
in the system.

- **Rust core layers**, with dependencies pointing inward only:
  1. *Domain* — MIDI concepts, state, and rules. Depends on nothing but the standard
     library and pure data crates. No Tauri, no I/O.
  2. *Application* — use cases orchestrating the domain. Defines the ports (traits) it
     needs from the outside world.
  3. *Infrastructure* — device access, persistence, OS integration. Implements the
     application's traits.
  4. *IPC surface* — Tauri commands and events. A translation layer only: it validates,
     delegates, and maps domain errors to contract errors. It MUST contain no business
     logic.
- **The webview MUST NOT contain business rules.** It renders state and dispatches
  intents. Presentation state is separate from domain state received over IPC.
- **The IPC contract MUST be typed end to end and defined once.** Command payloads,
  event payloads, and error shapes are declared in Rust and their TypeScript types are
  generated from those declarations. Hand-maintained parallel type definitions are
  forbidden — they drift silently and, with no tests, drift undetected.
- **IPC never crosses as untyped data.** No `serde_json::Value` in a command signature,
  no `any` or unchecked casts at the invoke boundary. Errors cross as typed variants,
  never as strings.

**Rationale:** The webview boundary is a serialization gap, and serialization gaps are
where type safety silently dies. A generated, typed contract turns a runtime failure
class into a compile-time one — which, given that compiling is our verification
(see Verification Standard), is the only place it can be caught at all.

### V. Named Patterns, Never Ceremony

Established design patterns MUST be applied where they solve a real problem in this
codebase, and MUST NOT be applied where they only add indirection.

- When a pattern is used, the module or type documentation MUST **name the pattern and
  state the problem it solves here**. "Repository — isolates the domain from the
  persistence crate so device storage can be swapped without touching domain types."
- Patterns are warranted when they answer a pressure that actually exists: Repository
  and Ports & Adapters at the infrastructure seam; Command for IPC entry points;
  Observer/pub-sub for MIDI event streaming to the webview; Newtype for domain
  identifiers and units; Builder for multi-field configuration; State machine for
  connection lifecycle.
- Patterns are ceremony when the pressure is absent: an interface with one
  implementation and no foreseeable second, a factory that only calls a constructor, a
  wrapper that only forwards, an abstraction introduced for a requirement nobody has
  stated. These MUST be removed.
- When in doubt, write the direct version. A pattern MAY be introduced the moment the
  second concrete pressure appears; that refactor is cheap, and premature abstraction
  is not.

**Rationale:** Patterns are compressed answers to recurring problems. Naming the
problem forces the check that the problem exists — which is precisely the check that
separates architecture from cargo cult.

### VI. The Screenshots Are the Design Authority

The reference images in `screenshots/` are normative, not inspirational. Where the
interface reproduces something they show, the screenshot wins over developer taste,
framework defaults, and component-library conventions.

- **Labels are copied verbatim**, character for character, including capitalisation,
  punctuation, and parentheses: `Remember up to`, `events`, `Clear`, `Time`, `Source`,
  `Message`, `Chan`, `Data`, `Act as a destination for other programs`,
  `Spy on output to destinations`, `Aftertouch (Poly)`, `Start/Stop/Continue`,
  `Song Position Pointer`, `All Channels`, `One Channel`. Rewording, sentence-casing,
  pluralising, or "improving" a label is a violation.
- **Control types are reproduced as shown.** A disclosure triangle stays a disclosure
  triangle — not a tab, accordion, or button. A checkbox stays a checkbox; a radio pair
  stays a radio pair — not a segmented control or switch. A bordered scrollable list
  stays one.
- **Grouping, order, and indentation are preserved**: the column order
  Time / Source / Message / Chan / Data; the three filter columns and the exact order of
  entries within each; the source tree's groups and the indentation of their children.
- **Default states match the reference**: both disclosure sections start collapsed, all
  filter checkboxes start checked, `All Channels` starts selected, and the retention
  field starts at `1000`.
- **Nothing is added to the referenced surface.** No extra buttons, toolbars, icons,
  badges, branding, tooltips-as-decoration, animations, or restyling of what the
  screenshots depict.
- **Deviation is permitted only where the platform cannot reproduce the original** —
  native window chrome, native system font, native checkbox and radio rendering. Every
  such deviation MUST be recorded in the module documentation with the reason
  (Principle III), and MUST be the smallest deviation that works.
- Verification is a side-by-side comparison of the running window against the reference
  image, as part of the manual run required by the Verification Standard.

**Rationale:** "Looks exactly like the screenshots" is the acceptance criterion the
project was given. Left to judgement, every developer and every UI library nudges
spacing, casing, and control choices toward its own defaults, and the result drifts a
little at a time until it resembles the reference only in outline. Naming the images as
the authority makes fidelity checkable instead of arguable.

### VII. A Mock Foundation Built to Be Extended

This application is a mock that real capability will be built on top of. That imposes
two obligations, and rules out a third thing that looks like extensibility but is not.

- **The simulated event source is one infrastructure implementation of an application
  port, and nothing more.** Substituting real MIDI input MUST be a matter of providing a
  different implementation of that port, touching no domain type, no use case, and no
  webview code.
- **Nothing above the infrastructure layer may know the data is simulated.** No `if
  mock` branches, no `is_mock` flags on domain types, no `Fake`/`Dummy`/`Mock` in domain
  or application vocabulary, no simulation-only fields riding along on real types.
  Generation parameters belong to the simulator, not to the domain.
- **Later features extend rather than edit.** Adding a message type, a column, a filter
  kind, or a source kind MUST mean adding one variant or entry in one place, with
  exhaustive matching making the compiler enumerate every site that must be updated.
  Wide `if`/`else` chains, stringly-typed dispatch, and catch-all `_ =>` arms over these
  domains are forbidden precisely because they let a new variant slip through silently.
- **Additive features MUST NOT disturb the referenced surface.** New capability earns
  new surface — a new column, a new section, a new panel — and MUST NOT relabel,
  reorder, restyle, or repurpose any control the screenshots show. This is how VI and
  VII coexist: the reference surface is frozen, the space around it is open.
- **Extension readiness is honest layering and exhaustive types, not a plugin system.**
  Principle V still governs: the one seam that MUST exist is simulated-versus-real event
  sources, because that substitution is already known to be coming. Abstractions for
  features nobody has specified are ceremony and MUST be rejected — "we might extend it
  later" is not a pressure, it is a guess.

**Rationale:** A mock built as a throwaway becomes a rewrite the moment real input
arrives, because the simulation leaks upward into types and screens that had no reason
to know about it. Confining it behind a port costs nothing now and makes the swap a
one-file change. The counterweight matters just as much: "extensible" is the most common
excuse for speculative abstraction, so the extensibility this project requires is
deliberately narrow and named.

## Verification Standard

**This project has no test suite, and testing is deliberately not a principle of this
constitution.** The following are prohibited in this repository:

- Test files of any kind — `tests/` directories, `#[cfg(test)]` modules, `*.test.ts`,
  `*.spec.ts`, snapshot fixtures.
- Test dependencies and harnesses in `Cargo.toml` (`[dev-dependencies]` added for
  testing) or `package.json` — Vitest, Jest, Playwright, `mockall`, `proptest`, and
  equivalents.
- Doctests. Rust doc examples MUST be written in non-running fences (`text` or
  `ignore`) so `cargo test --doc` has nothing to execute.
- CI test steps. Pipelines build; they do not test.

**Verification is compiling and running the application.** A change is verified when:

1. `cargo build` (or `cargo check`) completes with **zero warnings** — warnings are
   treated as errors, and `cargo clippy` MUST be clean.
2. The TypeScript project type-checks with `tsc --noEmit` under `strict` mode with
   zero errors.
3. The app launches via `tauri dev` or a release build, and the changed behavior is
   exercised by hand through the UI.

**Consequences that MUST shape the code:** because no test catches a regression, the
compiler MUST. This raises, rather than lowers, the bar on Principles I and IV —
make illegal states unrepresentable, keep exhaustive `match` without catch-all arms
where a new variant should force a compile error, use newtypes over primitives, and
never reach for `any`, `as` casts, or `unwrap` to quiet the compiler.

## Development Workflow & Quality Gates

**Definition of done.** A change is complete only when every gate passes:

1. `cargo clippy` — zero warnings.
2. `cargo fmt --check` and the webview formatter — clean.
3. `tsc --noEmit` under `strict` — zero errors.
4. The app builds, launches, and the change is manually exercised.
5. Every new or modified public item carries rustdoc/TSDoc explaining *why*
   (Principle III).
6. Every new or modified module carries a module-level doc.
7. No test artifacts introduced (Verification Standard).
8. No dead code, no unnamed magic values, no `unwrap`/`expect`/`any` in non-`main`
   paths (Principle I).
9. IPC changes regenerate the TypeScript contract from the Rust declarations
   (Principle IV).
10. Any change touching a surface the screenshots depict has been compared side by side
    against the reference image — labels verbatim, control types, order, indentation,
    and default states all matching (Principle VI). Deviations forced by the platform
    are documented with their reason.
11. No simulation detail has leaked above the infrastructure layer, and no additive
    feature has altered a control the screenshots show (Principle VII).

**Adding a dependency** requires a one-line justification in the consuming module's
docs. **Hand-rolling** something a library provides requires a justification naming the
libraries considered and why each was rejected (Principle II).

**Review focus, in order:** layering violations first (an inward-pointing dependency
broken is the costliest defect here), then mock containment, then IPC contract typing,
then error typing, then screenshot fidelity, then documentation coverage, then naming,
then pattern justification.

## Governance

This constitution supersedes all other development practices, conventions, and agent
defaults for this repository. Where a Spec Kit template, tool default, or generated
plan calls for tests, test tasks, or test-first workflow, **this constitution wins and
those steps MUST be skipped** — record the skip in the plan's Complexity Tracking or
equivalent section rather than silently ignoring it.

**Amendment procedure.** Amendments MUST be proposed as a change to this file with a
written rationale, a version bump, and — where the amendment invalidates existing code
— a migration note describing what must change. An amendment takes effect when merged.

**Versioning policy.** Semantic versioning applies to governance:

- **MAJOR** — a principle is removed or redefined in a backward-incompatible way, the
  Verification Standard is reversed, or the reference screenshots are replaced or
  retired as the design authority.
- **MINOR** — a principle or section is added, or existing guidance is materially
  expanded.
- **PATCH** — clarification, wording, or typo fixes that do not change what is
  required.

**Compliance review.** Every change is reviewed against the Definition of Done gates
above. Complexity that departs from these principles MUST be justified in writing at
the point of departure — in the module docs for code, in Complexity Tracking for plans.
Unjustified complexity is rejected rather than negotiated. Agents and contributors MUST
read this constitution before planning or implementing work; `CLAUDE.md` carries
runtime development guidance and MUST remain consistent with it.

**Version**: 1.1.0 | **Ratified**: 2026-08-26 | **Last Amended**: 2026-08-26
