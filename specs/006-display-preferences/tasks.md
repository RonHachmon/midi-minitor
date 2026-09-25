---

description: "Task list for 006-display-preferences"
---

# Tasks: A Preferences Surface, With Display Formats Implemented

**Input**: Design documents from `/specs/006-display-preferences/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/)

**Tests**: **None.** No test tasks appear below and none may be added. The constitution's
Verification Standard prohibits test files, test dependencies, doctests, and CI test steps in this
repository. The tasks template's test categories are inert here; the skip is recorded in
[plan.md](./plan.md) Complexity Tracking, as Governance requires. Verification is `cargo clippy`
clean, `tsc --noEmit` clean, and the manual run in [quickstart.md](./quickstart.md).

**Organization**: grouped by user story, so each can be implemented and hand-verified on its own.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: which user story the task serves (US1 … US6)
- Paths are relative to the repository root

## Path Conventions

This is a Tauri desktop application, not a single-project or web layout:

- `crates/midi-core/src/domain/` — MIDI concepts and rules; depends on nothing outward
- `crates/midi-core/src/application/` — use cases and the ports they need
- `crates/midi-macos/src/`, `crates/midi-windows/src/` — platform adapters
- `src-tauri/src/` — the IPC surface; translation only, no business logic
- `src/` — the webview; renders state, dispatches intents, holds no MIDI rules

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: make the two platform clock APIs reachable before anything needs them.

- [X] T001 [P] Add the `Win32_System_Performance` feature to a `windows` dependency in `src-tauri/Cargo.toml`, with a comment naming what it is for (`QueryPerformanceCounter` / `QueryPerformanceFrequency`)
- [X] T002 [P] Add `libc` as a macOS-target-only dependency in `src-tauri/Cargo.toml` for `mach_timebase_info` and `mach_absolute_time`, with the Principle II justification beside it

> **Corrected during implementation.** Both tasks originally named
> `crates/midi-windows/Cargo.toml` and `crates/midi-macos/Cargo.toml`. That was wrong:
> `SystemClock` lives in `src-tauri/src/settings.rs`, and the platform adapters reach the clock
> through `midi_core::application::ports::Clock` rather than calling it themselves. Putting the
> dependencies in the adapters would have left them unused there and still missing where they
> were needed. Both platform manifests were reverted untouched.
- [X] T003 Confirm the baseline is clean before changing behaviour: `cargo clippy --workspace --all-targets`, `cargo fmt --check`, `npx tsc --noEmit`

**Checkpoint**: both platform clocks are reachable; the tree still builds warning-free.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the settings spine — a setting can be chosen, persisted, crossed to Rust, and reach
the renderer. **Rendering still ignores the settings at the end of this phase**; each user story
wires one setting in. That is deliberate: it keeps behaviour identical until a story changes it,
so every story is a visible increment.

**⚠️ CRITICAL**: no user story work can begin until this phase is complete.

### Domain

- [X] T004 [P] Create `crates/midi-core/src/domain/display.rs` with `TimeFormat`, `NoteFormat`, `ControllerFormat`, `DataFormat`, `ProgramNumbering`, and `ExpertMode`, each with `const fn label(self) -> &'static str` and `const ALL`, per the table in [data-model.md](./data-model.md). Labels verbatim from `screenshots/setting.jpg` — note the en dash (U+2013) in `1 – 128 (Standard)` and `0 – 127 (Less common)`
- [X] T005 Add `DisplaySettings` to `crates/midi-core/src/domain/display.rs` with the six fields and a `Default` impl matching FR-011 (`ClockTime`, `NameMiddleC3`, `StandardName`, `Decimal`, `FromOne`, `ExpertMode::Off`); derive `Copy` and document why
- [X] T006 Add the group labels (`Time format`, `Note format`, `Controller format`, `Data format`, `Program number`, `(Decimal)`) and the three Expert-mode explanatory lines as named consts in `crates/midi-core/src/domain/display.rs` (FR-008, FR-010)
- [X] T007 Declare the new module in `crates/midi-core/src/domain/mod.rs`
- [X] T008 Create `crates/midi-core/src/domain/rendering.rs` and move `data_display` and the private `note_name` into it from `crates/midi-core/src/domain/message.rs`, taking `&MidiEvent` and `DisplaySettings`. **Render exactly as today, consulting no setting yet.** Module doc states its responsibility and why rendering is domain rather than webview
- [X] T009 In `crates/midi-core/src/domain/message.rs`, remove `data_display`/`note_name` and **replace** the superseded middle-C-equals-C4 justification with FR-011a's reasoning from [plan.md](./plan.md) Complexity Tracking — do not merely delete it, or the next reader restores C4 as a bug fix

### Application

- [X] T010 Add `#[serde(default)] pub display: DisplaySettings` to `PersistedSettings` in `crates/midi-core/src/application/settings.rs`, documenting why no migration is needed (FR-029), mirroring the existing `send` field's doc
- [X] T011 Make each field of `DisplaySettings` tolerate an unrecognised stored value by falling back to that field's default in `crates/midi-core/src/domain/display.rs` — per-field, not struct-level, so one bad value cannot discard the other five or take filters, columns, retention, sources, and saved requests down with it (FR-030)
- [X] T012 Give `Monitor` a `display: DisplaySettings` field, a `set_display` setter, a `display()` accessor, and include it in `persisted_settings()` in `crates/midi-core/src/application/monitor.rs`

### IPC surface

- [X] T013 [P] Add `DisplaySettingsDto` to `src-tauri/src/dto.rs` with the six fields as generated string unions plus `expert: bool`, deriving `Serialize`, `Deserialize`, and `Type`, per [contracts/ipc-commands.md](./contracts/ipc-commands.md)
- [X] T014 [P] Add `DisplayOptionDto`, `DisplayGroupDto`, `PreferencesTabDto`, `DisplayViewDto`, and `DisplayChangeDto` to `src-tauri/src/dto.rs` per [contracts/ipc-commands.md](./contracts/ipc-commands.md)
- [X] T015 Change `EventDto::from_event` in `src-tauri/src/dto.rs` to take the display settings and delegate `time` and `data` to `domain::rendering`; update `SnapshotDto::from_monitor` to pass what the `Monitor` now holds
- [X] T016 Add `get_display_model` to `src-tauri/src/commands.rs` returning `DisplayViewDto` built from the domain labels, mirroring `get_filter_model` and its rationale
- [X] T017 Add `set_display_settings` to `src-tauri/src/commands.rs` taking `DisplaySettingsDto`, calling `Monitor::set_display`, persisting via `state.persist(&monitor, &sender)?`, and returning `DisplayChangeDto { view, snapshot }`
- [X] T018 Register both commands in the `tauri_specta` builder in `src-tauri/src/lib.rs`
- [X] T019 Build once so `tauri-specta` regenerates `src/bindings.ts`; confirm the generator header is intact and both commands appear. Never hand-edit this file

### Webview

- [X] T020 [P] Extend `Screen` to `"monitor" | "send" | "settings"` in `src/sendStore.ts` and update the doc comments that say there are two screens
- [X] T021 [P] Create `src/settingsStore.ts` holding the `DisplayViewDto` (null before first read) and the mutation that calls `setDisplaySettings`, following the shape of `src/sendStore.ts`
- [X] T022 Add a third `ScreenTab` labelled `Settings` in `src/App.tsx`, leaving the existing two unchanged in label, type, and order
- [X] T023 [P] Create `src/components/settings/RadioGroupRow.tsx` — one right-aligned group label (with optional second line) and its stacked radios, rendering only strings it is handed
- [X] T024 Create `src/screens/SettingsScreen.tsx` with the three-tab bar (`Display`, `Sources`, `Other`, `Display` selected) rendered from `DisplayViewDto.tabs`, and mount it from `src/App.tsx`
- [X] T025 Create `src/components/settings/DisplayTab.tsx` rendering the five radio groups, the `Expert mode` checkbox, and the three explanatory lines entirely from the Rust-supplied model — no user-visible string hard-coded in TypeScript (FR-008, FR-010, [research.md](./research.md) D6)
- [X] T026 Let `src/store.ts` accept a replacement snapshot from a display change, reusing the existing `high_water_mark` guard so an in-flight batch cannot duplicate rows

**Checkpoint**: the `Display` tab renders, every control is operable, choices persist across a
restart — and the monitor still renders exactly as it did before this feature. Nothing visible
has changed in the event table yet. That is correct at this point.

---

## Phase 3: User Story 1 - Read the traffic in hexadecimal (Priority: P1) 🎯 MVP

**Goal**: `Data format` actually changes what the monitor shows, for retained rows as well as
arriving ones.

**Independent Test**: with events retained, switch `Data format` to `Hexadecimal number`, return
to the monitor, and confirm retained *and* new rows are hexadecimal. Switch back and confirm they
return to decimal.

- [X] T027 [US1] Wire `DataFormat` into the value renderers in `crates/midi-core/src/domain/rendering.rs` — velocities, controller values, pressures, pitch bend, song position, song select, and time code render decimal or hexadecimal (FR-019)
- [X] T028 [US1] Leave System Exclusive reporting `N bytes` and invalid messages reporting reason plus byte count under both formats in `crates/midi-core/src/domain/rendering.rs` — `Hexadecimal number` must not turn a SysEx transfer into an unbounded byte dump (Edge Cases)
- [X] T029 [US1] Confirm `set_display_settings` in `src-tauri/src/commands.rs` returns a snapshot rebuilt from `Monitor::visible_events()`, so retained rows re-render without the log being touched, reordered, or trimmed (FR-025, FR-026)
- [X] T030 [US1] Hand-verify Scenario 2 and Scenario 8 steps 1–2 of [quickstart.md](./quickstart.md): retained rows re-render within a second with no scroll or pause change, and the choice survives a restart

**Checkpoint**: User Story 1 is fully functional and demonstrable on its own.

---

## Phase 4: User Story 2 - Reach the preferences without disturbing the monitor (Priority: P1)

**Goal**: visiting the preferences mode costs the user nothing — no missed events, no reset
state, on either the monitor or the send screen.

**Independent Test**: with traffic arriving, switch to the preferences mode, wait, switch back;
confirm the events that arrived are present and that filters, columns, source selections,
retention, and paused state are unchanged.

- [X] T031 [US2] Confirm `set_display_settings` in `src-tauri/src/commands.rs` does **not** call `state.pump.discard_pending()`. `set_capture_state` and `clear_events` do because they freeze or empty the list; a display change does neither, and queued events must still be delivered ([contracts/ipc-commands.md](./contracts/ipc-commands.md))
- [X] T032 [US2] Confirm the persist call in `set_display_settings` goes through `state.persist(&monitor, &sender)` so the `send` half of the document is preserved — the erasure `PersistedSettings::with_send` exists to make greppable (FR-003)
- [X] T033 [US2] Confirm the event-stream and catalogue subscriptions in `src/App.tsx` still sit above the screen switch now that a third screen exists, so mounting `SettingsScreen` cannot tear down the stream (FR-002)
- [X] T034 [US2] Hand-verify Scenario 3 of [quickstart.md](./quickstart.md), including the send screen's target, publication, composer, and saved requests being untouched

**Checkpoint**: User Stories 1 and 2 both work independently.

---

## Phase 5: User Story 3 - Choose how notes, controllers, and programs are named (Priority: P2)

**Goal**: the monitor's vocabulary matches the equipment on the desk.

**Independent Test**: capture a Note On, a Control Change, and a Program Change; change each of
the three settings in turn and confirm the matching cell changes and the other two do not.

- [X] T035 [P] [US3] Create `crates/midi-core/src/domain/controller.rs` with a named `const [&str; 128]` of standard controller names and a lookup returning `Option<&'static str>`; module doc records the Principle II hand-rolling justification from [research.md](./research.md) D4 and names where the naming comes from
- [X] T036 [US3] Declare the module in `crates/midi-core/src/domain/mod.rs`
- [X] T037 [US3] Wire `NoteFormat` into note rendering in `crates/midi-core/src/domain/rendering.rs` for Note On, Note Off, and Aftertouch (Poly): `NameMiddleC3` → octave `note / 12 - 2`, `NameMiddleC4` → `note / 12 - 1`, plus decimal and hexadecimal. Every number 0–127 must produce a name under both conventions, running negative at the bottom (FR-015, FR-016)
- [X] T038 [US3] Wire `ControllerFormat` into controller rendering in `crates/midi-core/src/domain/rendering.rs`, falling back to the number when the table has no name — never blank, never a placeholder (FR-017, FR-018)
- [X] T039 [US3] Wire `ProgramNumbering` into program rendering in `crates/midi-core/src/domain/rendering.rs`: `FromOne` → `value + 1`, `FromZero` → `value` (FR-020)
- [X] T040 [US3] Confirm independence in `crates/midi-core/src/domain/rendering.rs`: the velocity beside a note is governed by `DataFormat`, never `NoteFormat`; a controller's value likewise (FR-021)
- [X] T041 [US3] Hand-verify Scenario 4 of [quickstart.md](./quickstart.md), including controller 85 (no standard name), note 0, and note 127

**Checkpoint**: the first-launch note naming now differs from `screenshots/data.png` by one
octave. That is FR-011a, recorded in [plan.md](./plan.md) Complexity Tracking — not a defect.

---

## Phase 6: User Story 4 - Time the traffic rather than read it (Priority: P2)

**Goal**: the `Time` column can show the host clock in three forms, including for events captured
before the format was chosen.

**Independent Test**: capture events, switch `Time format` through all four options, and confirm
the column changes form each time with one event's three host values being consistent conversions
of each other.

> **This is the only story that modifies files earlier phases created** — `MidiEvent` gains a
> field, which propagates to both platform adapters. Sequence it after US1–US3 or accept the
> merge.

- [X] T042 [P] [US4] Add `HostTicks`, `TickRate`, `HostTime`, and `Arrival` to `crates/midi-core/src/domain/ids.rs` per [data-model.md](./data-model.md). `TickRate::new` returns `Result` because a zero rate would make both conversions divide by zero and `panic` is denied workspace-wide
- [X] T043 [US4] Extend the `Clock` port in `crates/midi-core/src/application/ports.rs` with `arrival() -> Arrival` and `tick_rate() -> TickRate`, keeping `now()` for the three send-record sites; document why the two readings are one call ([contracts/clock-port.md](./contracts/clock-port.md))
- [X] T044 [US4] Implement the extended port for `SystemClock` in `src-tauri/src/settings.rs`, `#[cfg]`-split: macOS uses `libc::mach_timebase_info` for the rate, Windows uses `QueryPerformanceFrequency`/`QueryPerformanceCounter`. No `unwrap`/`expect`/`panic` — both are `unsafe` FFI returning fallible results
- [X] T045 [US4] Change `MidiEvent.timestamp: Timestamp` to `arrival: Arrival` in `crates/midi-core/src/domain/event.rs` and update `MidiEvent::new`
- [X] T046 [US4] Give `Monitor` a `tick_rate: TickRate` supplied at construction from the `Clock` port in `crates/midi-core/src/application/monitor.rs`, so no renderer reaches for the platform
- [X] T047 [US4] Thread per-packet timestamps through the decode loop in `crates/midi-macos/src/source.rs` — `receive` currently flattens `packets.iter()` into one `outcomes` vector and loses which packet each came from. Map a non-zero packet timestamp to `HostTime::Stamped` and a zero one to `HostTime::ZeroMeaningNow { received }`, CoreMIDI defining zero as "now" ([research.md](./research.md) D3)
- [X] T048 [US4] Change the single arrival reading in `crates/midi-windows/src/receive.rs` from `now()` to `arrival()`, always producing `HostTime::Stamped`; keep the existing rule that the reading is taken in the callback and nowhere else. Do not use `dwParam2`
- [X] T049 [US4] Wire `TimeFormat` into the `Time` column in `crates/midi-core/src/domain/rendering.rs`: `ClockTime` renders byte-for-byte what `Timestamp::to_display` produces today; the three host forms convert one `HostTicks` through the `TickRate`. Compute nanoseconds via `u128` — `ticks * 1_000_000_000` overflows `u64` within an ordinary uptime
- [X] T050 [US4] Hand-verify Scenario 5 of [quickstart.md](./quickstart.md), especially that events captured **before** a host format was selected still show host time (FR-025a)

**Checkpoint**: all four time formats work, on retained events as well as new ones.

---

## Phase 7: User Story 5 - Turn off the application's helpfulness (Priority: P3)

**Goal**: `Expert mode` shows what arrived rather than what the application decided it meant.

**Independent Test**: send a Note On with velocity 0, confirm it reads `Note Off`, tick
`Expert mode`, and confirm the same retained row now reads `Note On`.

- [X] T051 [US5] Render the `Message` column from the retained `MidiEvent.raw` status byte when `ExpertMode::On` in `crates/midi-core/src/domain/rendering.rs`, so a `Note On` with velocity zero reports as `Note On`. **Do not change the decoder** — `decoder.rs:430-438` keeps converting; see [research.md](./research.md) D5 for why moving the conversion would break FR-027
- [X] T052 [US5] Handle running status in that path in `crates/midi-core/src/domain/rendering.rs`: `raw` carries no status byte, so it cannot contradict the interpretation and the decoded message stands
- [X] T053 [US5] Show data values raw rather than formatted per the five settings when `ExpertMode::On` in `crates/midi-core/src/domain/rendering.rs` (FR-022, first explanatory line), leaving the five stored choices untouched so unticking restores them without the user re-choosing (FR-023)
- [X] T054 [US5] Render `HostTime::ZeroMeaningNow` as `0` under `ExpertMode::On` and as `received` under `Off` in `crates/midi-core/src/domain/rendering.rs` (third explanatory line)
- [X] T055 [US5] Hand-verify Scenario 6 of [quickstart.md](./quickstart.md), including the filter check: toggling `Expert mode` must not change which rows are listed (FR-027)

**Checkpoint**: all six settings are functional.

---

## Phase 8: User Story 6 - Find out that Sources and Other are not built yet (Priority: P3)

**Goal**: two tabs that tell the truth instead of offering inert controls.

**Independent Test**: click `Sources`, then `Other`; each is selectable, each states its settings
are not yet available, and neither offers an operable control.

- [X] T056 [US6] Populate `PreferencesTabDto.available` and `unavailableNote` in `src-tauri/src/commands.rs` so the core — not the webview — decides which tabs are usable, making later implementation a flag flip rather than a restructure (Principle VII)
- [X] T057 [P] [US6] Create `src/components/settings/UnavailableTab.tsx` rendering the core-supplied note with no operable control (FR-005)
- [X] T058 [US6] Render `UnavailableTab` for any tab the model marks unavailable in `src/screens/SettingsScreen.tsx`, and confirm returning to `Display` leaves every setting as it was (FR-006)
- [X] T059 [US6] Hand-verify Scenario 7 of [quickstart.md](./quickstart.md)

**Checkpoint**: all six user stories are independently functional.

---

## Phase 9: Polish & Cross-Cutting Concerns

- [X] T060 [P] Confirm every new public item in `crates/midi-core/src/domain/` and `src-tauri/src/` carries rustdoc explaining **why**, and every new module a `//!` doc naming its responsibility and layer (Principle III)
- [X] T061 [P] Confirm every new component in `src/components/settings/` and `src/screens/SettingsScreen.tsx` carries TSDoc, and that `src/App.tsx` and `src/sendStore.ts` no longer describe two screens
- [X] T062 Audit `crates/midi-core/src/domain/rendering.rs` for exhaustive `match` over every settings enum with no `_ =>` arm, so a seventh setting is a compile error at each site (Principle VII)
- [X] T063 Audit for magic values: the note-name and controller-name tables are named consts in one place each, and no rendering literal is inline (Principle I)
- [X] T064 Run the gates: `cargo clippy --workspace --all-targets` zero warnings, `cargo fmt --check`, `npx tsc --noEmit` zero errors
- [X] T065 Confirm no test artifacts were introduced — no `tests/`, no `#[cfg(test)]`, no `*.test.ts`, no test dependency in `Cargo.toml` or `package.json` (Verification Standard)
- [X] T066 Perform the side-by-side comparison in Scenario 1 of [quickstart.md](./quickstart.md) against `screenshots/setting.jpg`, then against `screenshots/data.png`, `filters.png`, and `sources.png` to confirm the monitor's controls are untouched (Principle VI, FR-007)
- [X] T067 Run Scenario 8 of [quickstart.md](./quickstart.md) in full, especially steps 5–6: an unrecognised value in one display field must leave the other five and every non-display setting intact (FR-030)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies
- **Foundational (Phase 2)**: depends on Setup — **blocks every user story**
- **US1 (Phase 3)** and **US2 (Phase 4)**: depend only on Foundational; independent of each other
- **US3 (Phase 5)**: depends only on Foundational
- **US4 (Phase 6)**: depends only on Foundational, but **edits files US1–US3 touch** (`rendering.rs`, `event.rs`) — sequence it after them unless you are prepared to merge
- **US5 (Phase 7)**: depends on Foundational; T054 additionally needs T042 and T049 from US4, because a zero host timestamp cannot be rendered before host time exists
- **US6 (Phase 8)**: depends only on Foundational
- **Polish (Phase 9)**: depends on every story you intend to ship

### Within Each Story

Domain rules before IPC translation before webview. No story is complete until its quickstart
scenario has actually been run by hand — that is this project's only verification.

### Parallel Opportunities

- **Phase 1**: T001 and T002 are different manifests
- **Phase 2**: T004 (domain) ‖ T013, T014 (DTOs) ‖ T020, T021, T023 (webview) — three layers, no shared file. T013–T019 are sequential among themselves; T024–T026 depend on T021
- **Phase 5**: T035 (the name table) is a new file and can start immediately
- **Phase 8**: T057 is a new file
- **Phase 9**: T060 and T061 cover different languages

After Foundational, **US1, US2, US3, and US6 can proceed in parallel** on different files. US4
and US5 are better sequenced, since both concentrate in `rendering.rs`.

---

## Parallel Example: Phase 2 Foundational

```text
# Three layers at once — no shared file:
Task: "T004 Create domain/display.rs with the six setting enums and their labels"
Task: "T013 Add DisplaySettingsDto to src-tauri/src/dto.rs"
Task: "T020 Extend Screen to include settings in src/sendStore.ts"
Task: "T023 Create src/components/settings/RadioGroupRow.tsx"
```

## Parallel Example: after Foundational

```text
# Four stories, four sets of files:
Task: "US1 — wire DataFormat into rendering.rs"
Task: "US2 — confirm non-disturbance in commands.rs and App.tsx"
Task: "US3 — create domain/controller.rs and its name table"
Task: "US6 — create UnavailableTab.tsx"
```

---

## Implementation Strategy

### MVP scope

**Foundational + US1 + US2** (T001–T034). Both are P1, and the spec says so for different
reasons: US1 is the shortest path to the feature's value, and US2 is the promise that using the
feature costs nothing. A preferences surface that loses the user's session is worse than none, so
shipping US1 without US2 is not an MVP.

At that point a user can read the traffic in hexadecimal, the choice survives a restart, and
visiting the settings mode disturbs nothing. The other four settings are visible but inert —
which is honest surface, and the next increments fill them.

### Incremental delivery

1. Setup + Foundational → the tab renders, nothing in the monitor has changed yet
2. US1 + US2 → **MVP**, demonstrable
3. US3 → the three naming settings; note octaves shift per FR-011a
4. US4 → host time; the one story that edits earlier files
5. US5 → Expert mode
6. US6 → the two unimplemented tabs tell the truth
7. Polish → gates, docs audit, side-by-side comparison, full quickstart

### Notes

- **No tests.** Do not add a test file, a test dependency, a doctest, or a CI test step, and do
  not "helpfully" add one while implementing a task. Compile cleanly, then run the app.
- Commit after each task or logical group.
- Two things are easy to get wrong and expensive to find later: per-field serde tolerance (T011,
  verified by T067) and not discarding the pending pump batch (T031).
- `src/bindings.ts` is generated. If it needs changing, change the Rust declaration and rebuild.
