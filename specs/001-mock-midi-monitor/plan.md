# Implementation Plan: Mock MIDI Monitor

**Branch**: `001-mock-midi-monitor` | **Date**: 2026-08-26 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-mock-midi-monitor/spec.md`

## Summary

A Tauri 2 desktop application that reproduces the Snoize-style MIDI Monitor window from
`screenshots/` and drives it with simulated MIDI traffic. All monitoring rules — source
selection, message-type and channel filtering, hex-prefix matching, and bounded retention — live
in a Tauri-free Rust core crate. The React webview is a thin renderer: it holds no business rules,
receives already-filtered event batches, and dispatches user intent back as typed commands.

The two throughput-critical decisions come from the Tauri documentation rather than from habit.
Tauri's event system is documented as *"not designed for low latency or high throughput"* and as
having *"no strong type support"* — both disqualifying for a 500 event/second stream under a
constitution that mandates a typed IPC contract. The stream therefore uses a **`tauri::ipc::Channel`**
(the mechanism Tauri documents for streaming), carrying **batches** coalesced at ~60 Hz rather than
one message per MIDI event. TypeScript types for every command, payload, and error are **generated**
from the Rust declarations by `tauri-specta`, which renders `Channel<T>` natively — satisfying the
constitution's ban on hand-maintained parallel type definitions.

## Technical Context

**Language/Version**: Rust 1.77+ (Tauri 2 MSRV), edition 2021 · TypeScript 5.x on Node 24.18.0
(verified present on this machine) · npm 11.16.0 (verified)

**Primary Dependencies**:
- Core (no Tauri): `serde`, `thiserror`, `specta`, `rand`
- App shell: `tauri` 2.x, `tauri-specta` 2.0.0-rc, `specta-typescript`, `tauri-plugin-store`
- Webview: React 19, Vite 6/7, Tailwind CSS v4 (`@tailwindcss/vite`), `@tanstack/react-virtual`,
  `zustand`, `@tauri-apps/api`

**Storage**: One JSON settings file managed by `tauri-plugin-store` in the OS app-config directory.
Retained MIDI events are session-only and never persisted (FR-047).

**Testing**: **None.** The project constitution's Verification Standard prohibits test files, test
dependencies, doctests, and CI test steps. Verification is `cargo clippy` clean, `tsc --noEmit`
clean, and running the app against the reference screenshots.

**Target Platform**: Cross-platform desktop (Windows / macOS / Linux). Primary development and
verification target is Windows 11, matching this machine.

**Project Type**: Desktop application — Cargo workspace (Rust core crate + Tauri app crate) with a
Vite/React webview.

**Performance Goals**: Sustain ≥500 simulated events/second with the window responsive (SC-005);
every control reacts within 250 ms; event list scrolls at 60 fps with the retention cap at its
maximum.

**Constraints**: No real MIDI I/O, drivers, or ports (FR-004, SC-011). Offline; no network access.
Zero-warning `cargo clippy` and zero-error strict `tsc`. Reference screenshots are normative for
every control they depict (Constitution VI).

**Scale/Scope**: One window; five UI regions (Sources, Filter, retention row, event table, column
menu); ~14 message types; retention cap 1–100 000 events, default 1000; ~10 React components.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design.*

Evaluated against `.specify/memory/constitution.md` v1.1.0.

| Principle | Gate | Pre-Phase 0 | Post-Phase 1 |
|-----------|------|-------------|--------------|
| I. Code Quality | Typed error enums, no `unwrap`/`expect`/`any`, no magic values, single responsibility | PASS | PASS |
| II. Library Reuse | Every solved problem delegated to a maintained crate/package; hand-rolling justified in module docs | PASS | PASS |
| III. Documentation | rustdoc/TSDoc on every public item, module docs, why-not-what | PASS | PASS |
| IV. Layered Architecture | Inward-only deps; typed IPC contract generated from Rust | PASS | PASS |
| V. Named Patterns | Patterns named with the problem they solve; ceremony rejected | PASS | PASS |
| VI. Screenshot Authority | Labels verbatim, control types, order, default states | PASS | PASS |
| VII. Mock Foundation | Simulator behind a port; no simulation leakage upward | PASS | PASS |
| Verification Standard | No test files, test deps, doctests, or CI test steps | PASS | PASS |

**How each gate is satisfied by this design:**

- **I** — Three error enums (`CoreError`, `SettingsError`, `IpcError`) via `thiserror`; all fallible
  paths return `Result`. Every literal from the screenshots (`1000`, column widths, channel range
  `1..=16`, batch interval) becomes a named constant in a `constants` module. `unwrap` appears only
  in `main.rs` startup, where failure means the app cannot run.
- **II** — `serde`/`thiserror`/`specta`/`rand` in the core; `tauri-plugin-store` for persistence
  rather than hand-rolled file I/O; `@tanstack/react-virtual` rather than a hand-written windowing
  loop; `zustand` rather than a bespoke store. The one deliberate hand-roll — the MIDI message model
  — is justified in [research.md](./research.md) and will be repeated in its module docs.
- **III** — Enforced at review via the Definition of Done; `#![warn(missing_docs)]` on both crates
  makes an undocumented public item a compiler warning, and warnings are already treated as errors.
- **IV** — Enforced by the compiler, not by convention: `midi-core` has **no `tauri` dependency in
  its `Cargo.toml`**, so a domain type cannot accidentally reach for `AppHandle`. The IPC surface
  lives only in `src-tauri`. TypeScript types are generated into `src/bindings.ts`, never edited.
- **V** — Patterns used and named: **Ports & Adapters** (`EventSource` trait — lets the simulator be
  swapped for real MIDI), **Repository** (`SettingsRepository` — keeps `tauri-plugin-store` out of
  the core), **Command** (Tauri command handlers), **Observer/streaming** (`Channel<EventBatch>`),
  **Newtype** (`SourceId`, `ChannelNumber`, `HexPrefix`). Patterns explicitly *rejected* as ceremony:
  no factory layer, no plugin registry, no dependency-injection container, no view-model layer.
- **VI** — A `reference/` note fixes the verbatim label strings and default states pulled from the
  screenshots; the fidelity checklist in [quickstart.md](./quickstart.md) is the manual gate.
- **VII** — The simulator is one `EventSource` implementation. `midi-core` contains no `mock`, `fake`,
  or `is_simulated` in any domain or application type; the word appears only inside the simulator
  module.

**Verification Standard conflict with Spec Kit templates**: the plan and tasks templates assume test
phases. Per the constitution's Governance section those steps are skipped, and this line is the
required record of the skip. No `tests/` directory, `#[cfg(test)]` module, `[dev-dependencies]`
test harness, `*.test.ts`, or CI test step will be created. Rust doc examples use non-running
` ```text ` fences so `cargo test --doc` has nothing to execute.

## Project Structure

### Documentation (this feature)

```text
specs/001-mock-midi-monitor/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/
│   └── ipc-contract.md  # Phase 1 output — typed IPC surface
├── checklists/
│   └── requirements.md  # /speckit-specify output
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
Cargo.toml                        # Workspace: members = ["crates/midi-core", "src-tauri"]

crates/midi-core/                 # Rust core — NO tauri dependency (enforces Principle IV)
├── Cargo.toml
└── src/
    ├── lib.rs                    # Crate docs: layering contract, why tauri is absent here
    ├── constants.rs              # Named values from the screenshots (defaults, ranges, widths)
    ├── domain/
    │   ├── mod.rs
    │   ├── ids.rs                # Newtype: SourceId, ChannelNumber
    │   ├── message.rs            # MessageKind, MessageCategory, MidiMessage, raw bytes
    │   ├── event.rs              # MidiEvent, Timestamp
    │   ├── source.rs             # Source, SourceGroup, SourceKind, CheckState
    │   ├── filter.rs             # FilterSettings, ChannelMode, HexPrefix, matching rules
    │   ├── column.rs             # Column, ColumnVisibility
    │   └── event_log.rs          # Bounded ring buffer, oldest-first eviction
    ├── application/
    │   ├── mod.rs
    │   ├── ports.rs              # EventSource, SettingsRepository, Clock (traits)
    │   ├── monitor.rs            # Monitor use cases: ingest, snapshot, apply settings
    │   └── error.rs              # CoreError
    └── simulator/
        ├── mod.rs                # EventSource impl — the ONLY module that knows data is fake
        ├── traffic.rs            # Musically coherent note/clock/CC generation
        └── catalogue.rs          # The fixed virtual sources named in screenshots/sources.png

src-tauri/                        # Tauri app shell — IPC surface only, no business rules
├── Cargo.toml
├── build.rs
├── tauri.conf.json
├── capabilities/default.json
├── icons/
└── src/
    ├── main.rs                   # Thin: delegates to lib.rs run()
    ├── lib.rs                    # Builder wiring, specta export, state, background pump
    ├── dto.rs                    # Wire types (Serialize + specta::Type)
    ├── commands.rs               # #[tauri::command] handlers — validate, delegate, map errors
    ├── stream.rs                 # Channel<EventBatch> pump, ~60 Hz coalescing
    ├── error.rs                  # IpcError — typed variants crossing the boundary
    └── settings.rs               # SettingsRepository impl over tauri-plugin-store

src/                              # React webview — renders state, dispatches intent
├── main.tsx
├── App.tsx
├── index.css                     # @import "tailwindcss" + @theme tokens
├── bindings.ts                   # GENERATED by tauri-specta — never hand-edited
├── store.ts                      # zustand: settings + event buffer
├── ipc.ts                        # Thin wrapper over generated bindings; rAF flush
└── components/
    ├── DisclosureSection.tsx     # ▶/▼ triangle section (screenshots: Sources, Filter)
    ├── SourcesPanel.tsx          # Bordered checkbox tree with groups
    ├── FilterPanel.tsx           # Three-column checkbox grid + channel radio pair
    ├── TriStateCheckbox.tsx      # Checked / unchecked / indeterminate
    ├── RetentionRow.tsx          # "Remember up to [N] events" + "Clear"
    ├── HexPrefixFilter.tsx       # Prefix entry + include/exclude mode
    ├── ColumnMenu.tsx            # Column visibility (additive surface — not in screenshots)
    └── EventTable.tsx            # Virtualized rows, Time/Source/Message/Chan/Data

index.html
package.json
vite.config.ts
tsconfig.json
screenshots/                      # Normative design authority (Constitution VI)
```

**Structure Decision**: A two-crate Cargo workspace, not a single `src-tauri` crate. The
constitution requires dependencies to point inward and forbids Tauri types in the domain layer;
splitting `crates/midi-core` (which simply does not list `tauri` as a dependency) from `src-tauri`
makes that violation a **compile error rather than a review comment**. With no test suite, moving
enforcement into the compiler is the project's stated strategy, so this is the cheapest place to buy
it. Layer separation *within* `midi-core` stays as modules — `domain/`, `application/`, `simulator/`
— because a further crate split would add build ceremony without adding enforcement the module
system and review already provide.

The webview mirrors the same discipline by being deliberately thin: `bindings.ts` is generated,
`store.ts` holds only presentation state, and no component computes whether an event should be
visible.

## Complexity Tracking

> Fill ONLY if Constitution Check has violations that must be justified

**No violations.** All eight gates pass in both the pre-research and post-design evaluations, so
this table is intentionally empty.

Two decisions that *look* like added complexity are recorded here for reviewers, with the reasoning
that keeps them inside the principles rather than in violation of them:

| Decision | Why it is not a violation |
|----------|---------------------------|
| Two-crate workspace instead of one crate | Serves Principle IV by making the "no Tauri in the domain" rule compiler-enforced. Principle V's ceremony test asks whether a real pressure exists — with no test suite, unenforceable layering is exactly that pressure. |
| Hand-rolled MIDI message model instead of a MIDI crate | Principle II permits hand-rolling when the need is genuinely domain-specific. The model's shape is dictated by the Filter panel's categories and the Data column's rendering, not by wire parsing; `midi-msg`/`wmidi` model the protocol, not this UI's vocabulary. Justification is recorded in [research.md](./research.md) and will be repeated in `domain/message.rs` module docs, as Principle II requires. |
