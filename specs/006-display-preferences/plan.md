# Implementation Plan: A Preferences Surface, With Display Formats Implemented

**Branch**: `006-display-preferences` | **Date**: 2026-09-23 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/006-display-preferences/spec.md`

## Summary

Add a third application mode reproducing `screenshots/setting.jpg`, with its `Display` tab fully
implemented and its `Sources` and `Other` tabs present but explicitly unavailable.

The technical spine is short, because the codebase already has the shape this feature needs:

1. **A `DisplaySettings` domain type** holding the six choices, persisted alongside the filter,
   columns, retention, and send settings in `PersistedSettings`.
2. **Rendering moves out of `MidiMessage` into a settings-aware `domain/rendering.rs`.** Today
   `MidiMessage::data_display()` renders with no parameters; it becomes a renderer taking
   `&MidiEvent` and `&DisplaySettings`.
3. **Retained events re-render for free** by following the existing precedent: mutating commands
   such as `set_retention_limit` and `clear_events` return a fresh `SnapshotDto` built from
   `Monitor::visible_events()`. `set_display_settings` does the same, so FR-025 costs one return
   value rather than a new mechanism.
4. **Host time is captured at arrival**, because a setting chosen later must be satisfiable by an
   event captured earlier (FR-025a). The `Clock` port gains an `arrival()` reading pairing the
   wall clock with the platform's host tick counter, and `MidiEvent` carries both.
5. **Expert mode reads `MidiEvent.raw`** rather than changing the decoder. The raw bytes are
   already retained and already documented as the authority wherever interpretation is lossy —
   which is exactly the `Note On` velocity-zero case the checkbox exists to expose.

The webview gets a `SettingsScreen`, and — following `get_filter_model` — every label on the
`Display` tab is supplied by Rust so the interface cannot reword what `screenshots/setting.jpg`
fixes.

## Technical Context

**Language/Version**: Rust (edition 2021, rust-version 1.77) for the core, IPC surface, and the
two platform adapters; TypeScript 5 under `strict` for the webview.

**Primary Dependencies**: Tauri 2 with `tauri-specta` 2.0.0-rc.21 / `specta` 2.0.0-rc.22 for the
generated IPC contract; React + Zustand + Tailwind in the webview; `coremidi` 0.9 on macOS;
`windows` 0.62 on Windows; `thiserror` 2; `chrono` for the wall clock.

**New dependencies**: one — `libc` on macOS targets only, for `mach_timebase_info`. On Windows,
`QueryPerformanceCounter` / `QueryPerformanceFrequency` are already reachable through the
`windows` crate by adding the `Win32_System_Performance` feature to the existing dependency.
See [research.md](./research.md) D3.

**Storage**: `tauri-plugin-store` through `StoreSettingsRepository`, holding one
`PersistedSettings` document. This feature adds one nested field under `#[serde(default)]` — the
same additive shape the send screen used, and for the same reason: no migration needed, and a
document written by any earlier version keeps every other setting (FR-029).

**Testing**: None. The constitution's Verification Standard prohibits a test suite in this
repository. Verification is `cargo clippy` clean, `tsc --noEmit` clean, and manual exercise of
the running application — see [quickstart.md](./quickstart.md). The test-oriented phases in the
Spec Kit plan and tasks templates are recorded as skipped in Complexity Tracking.

**Target Platform**: macOS and Windows desktop, one Tauri window.

**Project Type**: Desktop application — Rust core plus a webview, layered per Principle IV.

**Performance Goals**: The event stream sustains 500 events per second through 16 ms batches;
this feature must not slow that path. Rendering moves from "once per event at capture" to "once
per event per snapshot", so the arrival path gains only two integer reads and loses two string
allocations. A settings change re-renders at most `RetentionLimit` events once, which must
complete within the one second SC-002 allows.

**Constraints**: Zero `cargo clippy` warnings with `unwrap_used`, `expect_used`, `panic`, and
`missing_docs` denied at the workspace level; `tsc --noEmit` clean under `strict`; no
`serde_json::Value` or `any` crossing the IPC boundary; every `match` over a settings enum
exhaustive with no catch-all arm, so a seventh display setting becomes a compile error at each
site that must handle it.

**Scale/Scope**: Six settings, three tabs, one new screen. Roughly three new Rust modules, four
new webview components, two new IPC commands, and one changed field on `MidiEvent` that
propagates to two platform adapters.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design — see the
re-evaluation below the table.*

| Gate | Requirement | Status | How this design satisfies it |
|------|-------------|--------|------------------------------|
| **I. Single responsibility** | One reason to change per unit | **PASS** | `MidiMessage` stops rendering and returns to modelling alone; rendering moves to `domain/rendering.rs`, whose reason to change is a display rule. This *removes* an existing double responsibility rather than adding one. |
| **I. Intention-revealing names** | Purpose, not mechanism | **PASS** | `TimeFormat`, `NoteFormat`, `ControllerFormat`, `ProgramNumbering`, `ExpertMode`, `HostTicks`, `TickRate`, `Arrival`. No abbreviations outside the domain's own vocabulary. |
| **I. Typed errors as values** | `Result<T, E>` with purpose-built enums | **PASS** | The only fallible new path is reading the platform tick rate at startup; it yields a typed error, never a panic. Display settings themselves cannot fail — every value is an enum, so there is no invalid state to reject. |
| **I. No magic values** | Named consts | **RISK → MANAGED** | The 128 standard controller names and the 12 note names are tables, not scattered literals: one named `const` array each, in one module, with the source of the names documented. |
| **II. Reuse libraries** | Search before writing | **RISK → JUSTIFIED** | `libc` for `mach_timebase_info` and the `windows` crate for QPC are used rather than hand-declared FFI. The controller-name table is hand-written; see Complexity Tracking and [research.md](./research.md) D4. |
| **III. Documentation explains why** | Module + public item docs | **PASS** | Every new module carries a `//!` doc stating its layer and reasoning. `message.rs` must have its superseded middle-C-equals-C4 justification **replaced** with FR-011a's reasoning, not merely deleted — otherwise the next reader restores it as a defect. |
| **IV. Layering** | Dependencies point inward | **PASS** | `DisplaySettings` and the renderers are domain (a display rule is a fact about MIDI — the same argument `data_display` already makes). `Monitor` holds them. The IPC surface translates only. The platform adapters supply host ticks through the existing `Clock` port and gain no new outward dependency. |
| **IV. Typed IPC, generated** | Declared in Rust, generated to TS | **PASS** | Two new commands and their DTOs are `#[specta::specta]` / `derive(Type)`; `src/bindings.ts` is regenerated, never hand-edited. Settings enums cross as generated string unions, never as strings the webview composes. |
| **IV. Labels from Rust** | Screenshot content is normative data | **PASS** | `get_display_model` returns every label verbatim, exactly as `get_filter_model` already does for the Filter panel. The webview renders strings it is handed and cannot invent or reword one. |
| **V. Named patterns, no ceremony** | Pattern must answer real pressure | **PASS** | No new pattern is introduced. The `Clock` port is *extended* rather than joined by a second `HostClock` port — one port, two readings that must be taken at the same instant. A separate port would be an interface with the same two implementations and no independent reason to change. |
| **VI. Screenshots are authority** | Verbatim labels, control types, order, defaults | **PASS, with one recorded conflict** | `screenshots/setting.jpg` is reproduced in full. FR-011a resolves its disagreement with `screenshots/data.png` over note octaves in favour of `setting.jpg`, on the user's explicit instruction — logged in Complexity Tracking with its reason, as Principle VI requires of any deviation. |
| **VI. Nothing added to the referenced surface** | No extra controls | **PASS** | The `Display` tab carries exactly the six controls the image shows plus the three explanatory lines. No search box, no reset button, no preview pane, no icons. |
| **VII. Additive, not disturbing** | New capability earns new surface | **PASS** | The monitor screen's controls are untouched in label, type, order, and default. What changes is the *contents* of three cells, which is the feature's entire purpose. The mode switcher gains a third button — new surface, as the send screen's strip was. |
| **VII. Extend rather than edit** | One variant in one place, exhaustive matching | **PASS** | Each setting is an enum matched exhaustively with no `_ =>` arm, so adding a time format or a controller format becomes a compile error at every site that must handle it. |
| **VII. Mock containment** | No simulation detail above infrastructure | **PASS** | Untouched. This feature adds no event source. |
| **Verification Standard** | No tests, no test deps, no CI test steps | **PASS** | No test artifacts are introduced. The templates' test phases are recorded as skipped in Complexity Tracking rather than silently dropped, per Governance. |

**Post-Phase-1 re-evaluation**: re-checked after [data-model.md](./data-model.md) and
[contracts/](./contracts/) were written. No gate changed status. The design introduced no new
abstraction beyond the `Clock` extension, added no dependency beyond the single `libc` entry
already recorded, and the entries in Complexity Tracking below are the complete set of justified
departures.

## Project Structure

### Documentation (this feature)

```text
specs/006-display-preferences/
├── plan.md              # This file (/speckit-plan output)
├── research.md          # Phase 0 output — 7 decisions
├── data-model.md        # Phase 1 output — domain types and their invariants
├── quickstart.md        # Phase 1 output — the manual verification run
├── contracts/
│   ├── display-settings.md   # The domain type and its rendering rules
│   ├── ipc-commands.md       # The two new commands and their DTOs
│   └── clock-port.md         # The extended Clock port and its two implementations
├── checklists/
│   └── requirements.md  # Spec quality checklist — 16/16
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
crates/midi-core/src/
├── domain/
│   ├── display.rs          # NEW — DisplaySettings and its six enums, with verbatim labels
│   ├── rendering.rs        # NEW — settings-aware renderers for Time, Message, and Data
│   ├── controller.rs       # NEW — the 128 standard controller names
│   ├── message.rs          # CHANGED — data_display/note_name move out; the C4 doc is replaced
│   ├── event.rs            # CHANGED — `timestamp: Timestamp` becomes `arrival: Arrival`
│   ├── ids.rs              # CHANGED — HostTicks, TickRate, Arrival join Timestamp
│   └── mod.rs              # CHANGED — declares the new modules
└── application/
    ├── ports.rs            # CHANGED — Clock gains arrival() and tick_rate()
    ├── monitor.rs          # CHANGED — holds DisplaySettings and the TickRate; adds a setter
    └── settings.rs         # CHANGED — PersistedSettings gains `display`, #[serde(default)]

crates/midi-macos/src/
└── source.rs               # CHANGED — per-packet timestamps survive the decode loop

crates/midi-windows/src/
├── receive.rs              # CHANGED — the callback's arrival reading carries host ticks
└── source.rs               # CHANGED — the two abandon sites take an Arrival

src-tauri/
├── Cargo.toml              # CHANGED — libc (macOS) and windows/Win32_System_Performance
└── src/
    ├── commands.rs         # CHANGED — get_display_model, set_display_settings
    ├── dto.rs              # CHANGED — DisplayViewDto and friends; EventDto takes settings
    ├── error.rs            # CHANGED — maps CoreError::UnusableTickRate
    ├── settings.rs         # CHANGED — SystemClock implements the extended port
    └── lib.rs              # CHANGED — reads the tick rate; registers the two commands

src/
├── bindings.ts             # REGENERATED — never hand-edited
├── App.tsx                 # CHANGED — a third ScreenTab
├── ipc.ts                  # CHANGED — loadDisplayView, setDisplaySettings
├── sendStore.ts            # CHANGED — Screen gains "settings"; its doc says three
├── settingsStore.ts        # NEW — the display model and the chosen tab
├── screens/
│   └── SettingsScreen.tsx  # NEW — the tab bar and the three panels
└── components/settings/
    ├── DisplayTab.tsx      # NEW — the six controls, rendered from the Rust-supplied model
    ├── RadioGroupRow.tsx   # NEW — one right-aligned label and its stacked radios
    └── UnavailableTab.tsx  # NEW — what Sources and Other say
```

**Two corrections made during implementation**, recorded here rather than left to diverge:

- The `libc` and `windows` performance-counter dependencies belong to **`src-tauri`**, not to the
  platform adapter crates this plan originally named. `SystemClock` lives in `src-tauri`, and the
  adapters reach the clock through the `Clock` port rather than calling the platform themselves.
- **`src/store.ts` needed no change.** Its existing `applySnapshot` and `highWaterMark` already
  do exactly what a display change requires, which is the point D1 was making about this feature
  reusing a mechanism rather than adding one.

**Structure Decision**: the existing four-crate workspace and webview layout are kept unchanged.
This feature adds modules inside them rather than introducing a new crate or directory, because
nothing here crosses a new boundary: display settings are domain state, their renderers are
domain rules, their commands are IPC translation, and their controls are webview presentation —
each already has a home. A `crates/midi-display` crate would be an abstraction with one consumer
and no second pressure, which Principle V requires be rejected.

The one structural judgement worth naming is putting `DisplaySettings` and the renderers in the
**domain** rather than in the IPC surface or the webview. The precedent is explicit:
`MidiMessage::data_display` already lives in the domain, and its doc states why — what belongs in
the Data cell is a fact about MIDI, not a presentation choice, and rendering it in the webview
would put a MIDI rule on the far side of a serialization boundary and duplicate it. A
*settings-dependent* rendering is that same fact with a parameter. The webview continues to
receive finished strings.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| **Principle VI** — the first-launch note format diverges from `screenshots/data.png` | FR-011a. `screenshots/setting.jpg` selects `Note (Middle C = C3)`; `screenshots/data.png` shows notes produced under the middle-C-equals-C4 convention. Both images are normative and they disagree. The user resolved the conflict in favour of `setting.jpg` on 2026-09-23. | Keeping C4 as the default was the alternative and was explicitly rejected by the user. It would leave the `Display` tab's default contradicting the very image that defines that tab — a deviation on the surface this feature is *about*, taken to preserve fidelity on a surface it only incidentally affects. Both conventions remain selectable (FR-016), so neither reading is lost. The superseded justification in `message.rs` is **replaced** with this reasoning rather than deleted, so it is not restored later as a defect. |
| **Principle II** — the 128 standard controller names are hand-written | FR-017, FR-018. `Controller format` → `Standard name` needs display strings for controller numbers, and none exists in the workspace today. | Adding a MIDI crate solely for name strings was rejected on two counts. First, none publishes them as display strings — `wmidi::ControlFunction` exposes Rust identifiers as associated constants, not text, so a mapping table would still have to be written by hand. Second, `Cargo.lock` contains no general MIDI crate at all: features 002 and 003 rejected `midir`, `wmidi`, `midi-msg`, and `midly` because each refuses the malformed input a monitor exists to show. Pulling a full MIDI model back in to read a name table is exactly the disproportionate dependency tree Principle II names as grounds to hand-roll, and controller names are domain vocabulary, which is its other stated exception. The justification is recorded in the module docs as Principle II requires. |
| **Verification Standard** — the templates' test phases are skipped | Governance requires the skip be recorded rather than silently ignored. | The Spec Kit plan and tasks templates assume a test-first workflow. This repository prohibits test files, test dependencies, doctests, and CI test steps. Every test-oriented phase and task category from those templates is inert here; verification is `cargo clippy`, `tsc --noEmit`, and the manual run in [quickstart.md](./quickstart.md). |
