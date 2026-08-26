---

description: "Task list for Mock MIDI Monitor implementation"
---

# Tasks: Mock MIDI Monitor

**Input**: Design documents from `/specs/001-mock-midi-monitor/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/ipc-contract.md](./contracts/ipc-contract.md), [quickstart.md](./quickstart.md)

**Tests**: **NONE — deliberately omitted.** The project constitution's Verification Standard
prohibits test files, test dependencies, doctests, and CI test steps. Per the constitution's
Governance section, the tasks template's test phases are skipped and this line is the required
record of that skip. Verification is `cargo clippy` clean, `tsc --noEmit` clean, and running the app
against [quickstart.md](./quickstart.md).

**Organization**: Tasks are grouped by user story. Unlike a typical feature, US2–US5 each require
US1's event list to exist before they can be exercised — see [Dependencies](#dependencies--execution-order).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1–US5)
- Include exact file paths in descriptions

## Path Conventions

Two-crate Cargo workspace with a Vite/React webview, per [plan.md](./plan.md):

- `crates/midi-core/src/` — Rust core, **no `tauri` dependency**
- `src-tauri/src/` — Tauri app shell, IPC surface only
- `src/` — React webview

## Standing obligations on every task

These apply to each task below and are not repeated as separate tasks:

- **Rustdoc/TSDoc on every public item, plus module docs, explaining *why*** (Constitution III).
- **Typed errors as values**; no `unwrap`/`expect`/`panic!` outside `main.rs`, no `any` (Constitution I).
- **No magic values** — every literal from the screenshots becomes a named constant.
- **Exhaustive `match` with no catch-all arm** over `MessageKind`, `Column`, and `MidiMessage`, so a
  later variant is a compile error (Constitution VII).
- **No test files, `#[cfg(test)]` modules, test dependencies, or runnable doc examples.** Rust doc
  examples use ` ```text ` fences.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Toolchain, scaffold, and build configuration

- [X] T001 Install the Rust toolchain (`winget install Rustlang.Rustup`, then `rustup default stable`) and confirm `cargo --version` in a fresh terminal — verified absent on this machine, blocks every later task
- [X] T002 Scaffold the Tauri 2 + React + TypeScript app at the repository root with `npm create tauri-app@latest`, targeting the existing directory
- [X] T003 Convert the repository root to a Cargo workspace in `Cargo.toml` with `members = ["crates/midi-core", "src-tauri"]`, and create the empty `crates/midi-core/` crate via `cargo new --lib crates/midi-core`
- [X] T004 Add core crate dependencies (`serde`, `thiserror`, `specta`, `rand`, `enumset`) to `crates/midi-core/Cargo.toml` — deliberately **no `tauri`**, which is what makes Principle IV compiler-enforced
- [X] T005 Add app shell dependencies (`tauri` 2.x, `tauri-specta` 2.0.0-rc, `specta`, `specta-typescript`, `tauri-plugin-store`, `midi-core` path dependency) to `src-tauri/Cargo.toml`
- [X] T006 [P] Install Tailwind CSS v4 (`npm install tailwindcss @tailwindcss/vite`) and register the plugin in `vite.config.ts`
- [X] T007 [P] Install webview dependencies `@tanstack/react-virtual` and `zustand` via npm
- [X] T008 [P] Configure `vite.config.ts` for Tauri per research D-06: `clearScreen: false`, `server.strictPort: true` on 5173, and `watch.ignored: ['**/src-tauri/**']`
- [X] T009 [P] Enable `strict: true` and `noUncheckedIndexedAccess` in `tsconfig.json`, and add an `npm run typecheck` script running `tsc --noEmit`
- [X] T010 [P] Add `#![warn(missing_docs)]` and workspace lint configuration in `Cargo.toml` so undocumented public items and clippy warnings fail the build
- [X] T011 [P] Configure the window (title `MIDI Monitor`, initial size matching the reference proportions, resizable) in `src-tauri/tauri.conf.json`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Domain primitives, the simulator behind its port, the typed IPC pipe, and the app shell — everything every user story needs

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Rust core — domain primitives

- [X] T012 Create `crates/midi-core/src/lib.rs` with crate-level docs stating the layering contract and **why this crate has no `tauri` dependency**, declaring the `constants`, `domain`, `application`, and `simulator` modules
- [X] T013 [P] Define screenshot-derived constants (default retention `1000`, retention range `1..=100_000`, channel range `1..=16`, batch interval ~16 ms) in `crates/midi-core/src/constants.rs`
- [X] T014 [P] Implement the newtypes `SourceId`, `SourceGroupId`, `EventId`, `ChannelNumber` (1-based, validated `1..=16`), `RetentionLimit`, and `Timestamp` in `crates/midi-core/src/domain/ids.rs`
- [X] T015 [P] Define the `CoreError` enum with `thiserror`, documenting each variant's trigger and the caller's recourse, in `crates/midi-core/src/application/error.rs`
- [X] T016 Implement `MidiMessage`, `MessageKind`, `MessageCategory`, and `InvalidReason` with exhaustive `kind()`, `channel() -> Option<ChannelNumber>`, `display_name()`, and Data-column rendering, in `crates/midi-core/src/domain/message.rs` — module docs MUST carry the Principle II justification for hand-rolling the MIDI model (research D-05)
- [X] T017 Implement `MidiEvent`, `RawBytes`, and the uppercase no-separator hex rendering of raw bytes in `crates/midi-core/src/domain/event.rs`
- [X] T018 [P] Implement `Source`, `SourceGroup`, and derived `CheckState` (Checked/Unchecked/Mixed) in `crates/midi-core/src/domain/source.rs`

### Rust core — ports and simulator

- [X] T019 Define the `EventSource`, `SettingsRepository`, and `Clock` traits in `crates/midi-core/src/application/ports.rs`, naming **Ports & Adapters** and the problem each port solves (Constitution V)
- [X] T020 [P] Define the fixed virtual source catalogue matching `screenshots/sources.png` exactly — `MIDI sources` over `IAC Driver Bus 1` and `MidiKeys`, standalone `Act as a destination for other programs`, `Spy on output to destinations` over `IAC Driver Bus 1` (distinct `SourceId`s despite the shared display name) — in `crates/midi-core/src/simulator/catalogue.rs`
- [X] T021 Implement plausible traffic generation (paired Note On/Off with velocities, periodic Clock and Active Sense, occasional Control/Program/Pitch Wheel/Channel Pressure/Aftertouch/System Common/SysEx/Invalid, across multiple channels and sources) covering **every** `MessageKind` per FR-007, in `crates/midi-core/src/simulator/traffic.rs`
- [X] T022 Implement `SimulatedSource` as the `EventSource` adapter with its background generation thread in `crates/midi-core/src/simulator/mod.rs` — the **only** module permitted to know the data is simulated (Constitution VII)
- [X] T023 Implement the `Monitor` application service holding source selection and exposing `ingest` and `snapshot` (returning all events from selected sources at this stage) in `crates/midi-core/src/application/monitor.rs`

### Tauri app shell — the typed IPC surface

- [X] T024 [P] Define `IpcError` as a `specta::Type` enum with `From<CoreError>`, so errors cross as discriminated unions rather than strings, in `src-tauri/src/error.rs`
- [X] T025 Define the wire DTOs `EventDto`, `SourceDto`, `SourceGroupDto`, `SnapshotDto`, and `EventBatchDto` with `Serialize + specta::Type` in `src-tauri/src/dto.rs`, with the Data column and `rawHex` pre-rendered in Rust per the contract
- [X] T026 Implement the `getCatalogue`, `snapshot`, and `subscribeEvents` command handlers — validate, delegate to `Monitor`, map errors, no business rules — in `src-tauri/src/commands.rs`
- [X] T027 Implement the `Channel<EventBatchDto>` pump coalescing admitted events into at most one batch per ~16 ms in `src-tauri/src/stream.rs`, using the named interval constant
- [X] T028 Wire the Tauri builder in `src-tauri/src/lib.rs`: managed `Monitor` state, the `tauri-specta` `Builder` with `ErrorHandlingMode::Result` and `Casing::CamelCase`, and the debug-only `export(Typescript::default(), "../src/bindings.ts")`
- [X] T029 Reduce `src-tauri/src/main.rs` to a thin delegation to `lib.rs::run()`
- [X] T030 Verify `src/bindings.ts` is generated on a debug build and add it to `.gitignore` review — confirm it is never hand-edited (Constitution IV)

### Webview shell

- [X] T031 [P] Define the screenshot-derived design tokens (row height, column widths, header tint, list border, system font stack) as `@theme` tokens beside `@import "tailwindcss"` in `src/index.css`
- [X] T032 [P] Create the zustand store holding settings and the event buffer in `src/store.ts` — presentation state only, no filtering logic
- [X] T033 Implement `src/ipc.ts`: open the typed `Channel` from the generated bindings, buffer incoming batches in a ref, and flush on `requestAnimationFrame`; drop batches whose ids precede the latest snapshot's high-water mark
- [X] T034 Build the window shell in `src/App.tsx` — Sources section, Filter section, retention row, event table region — matching the vertical order in `screenshots/main-screen.png`
- [X] T035 [P] Implement the reusable `DisclosureSection` component with `▶`/`▼` triangle, collapsed by default, in `src/components/DisclosureSection.tsx`

**Checkpoint**: `npm run tauri dev` launches a window with the correct regions and a live event channel delivering batches — the table itself arrives in US1

---

## Phase 3: User Story 1 - Watch a live stream of MIDI events (Priority: P1) 🎯 MVP

**Goal**: A live, scrolling event list with the five columns, a working retention cap, and Clear.

**Independent Test**: Launch with nothing else implemented — events appear continuously with correct Time/Source/Message/Chan/Data; changing the retention count trims the list; Clear empties it and it refills.

- [X] T036 [US1] Implement the bounded `EventLog` ring buffer with `push`, `set_limit` (evicting immediately), `clear`, and oldest-first eviction in `crates/midi-core/src/domain/event_log.rs`
- [X] T037 [US1] Extend `Monitor` in `crates/midi-core/src/application/monitor.rs` with `set_retention(limit)` and `clear()`, and have `ingest` push into the `EventLog`
- [X] T038 [US1] Add `setRetentionLimit` and `clearEvents` command handlers returning a fresh `SnapshotDto` in `src-tauri/src/commands.rs`, rejecting out-of-range limits as `IpcError::RetentionOutOfRange`
- [X] T039 [P] [US1] Implement the virtualized event table with `useVirtualizer`, a fixed row height, and the header row `Time | Source | Message | Chan | Data` in `src/components/EventTable.tsx` — appending oldest-to-newest so timestamps ascend downward (FR-014)
- [X] T040 [P] [US1] Implement the retention row reading `Remember up to` · numeric field · `events` with `Clear` at the opposite edge, in `src/components/RetentionRow.tsx` — labels verbatim from `screenshots/main-screen.png`
- [X] T041 [US1] Validate retention entry in `src/components/RetentionRow.tsx`: reject or normalise zero, blank, negative, non-numeric, and over-maximum values, surfacing the typed error at the field (FR-019)
- [X] T042 [US1] Wire the table and retention row into `src/App.tsx` against the store and `src/ipc.ts`, replacing the local buffer whenever a command returns a snapshot
- [X] T043 [US1] Run scenario **S1** in [quickstart.md](./quickstart.md) and confirm SC-002 (events within 10 s) and SC-006 (count never exceeds the limit)

**Checkpoint**: A complete, demonstrable MIDI monitor — the MVP

---

## Phase 4: User Story 2 - Choose which sources to monitor by name (Priority: P2)

**Goal**: A bordered checkbox tree of named sources and groups; only checked sources contribute events.

**Independent Test**: With several sources emitting, uncheck all but one and confirm the Source column shows only that name; re-check a group and its members' events return.

- [X] T044 [US2] Add `retain_sources` to `crates/midi-core/src/domain/event_log.rs`, dropping retained events whose source was deselected (FR-026)
- [X] T045 [US2] Add `set_source_selected` and `set_group_selected` to `crates/midi-core/src/application/monitor.rs`, applying a group's state to all children and recomputing `CheckState`
- [X] T046 [US2] Expose `monitoring: bool` on `SnapshotDto` in `src-tauri/src/dto.rs`, false when no source is selected (FR-027)
- [X] T047 [US2] Add `setSourceSelected` and `setGroupSelected` command handlers returning a fresh snapshot in `src-tauri/src/commands.rs`
- [X] T048 [P] [US2] Implement the tri-state checkbox (checked / unchecked / indeterminate) in `src/components/TriStateCheckbox.tsx`
- [X] T049 [US2] Implement the bordered, scrollable source tree with expandable groups and indented children in `src/components/SourcesPanel.tsx`, reproducing `screenshots/sources.png` exactly — groups expanded by default
- [X] T050 [US2] Render the "nothing is being monitored" state in `src/components/EventTable.tsx` when `monitoring` is false, so an empty list is explained rather than looking frozen (FR-027)
- [X] T051 [US2] Run scenario **S2** in [quickstart.md](./quickstart.md) and confirm SC-003 (isolate one named source in under 15 s)

**Checkpoint**: US1 and US2 both work

---

## Phase 5: User Story 3 - Filter by message type and channel (Priority: P3)

**Goal**: Three columns of message-category checkboxes plus an All Channels / One Channel radio pair.

**Independent Test**: Uncheck `Real Time` and confirm Clock and Active Sense disappear while Note events continue; select One Channel = 5 and confirm only channel-5 events remain.

- [X] T052 [US3] Implement `FilterSettings`, `ChannelMode` (as a sum type so "One Channel with no channel" is unrepresentable), and the kind/channel matching rules in `crates/midi-core/src/domain/filter.rs`
- [X] T053 [US3] Add `view(&FilterSettings)` to `crates/midi-core/src/domain/event_log.rs`, iterating retained events that pass the filter, oldest first — a view over the log, never an admission gate (research D-04)
- [X] T054 [US3] Add `set_filter` to `crates/midi-core/src/application/monitor.rs` and make `snapshot` and `ingest` apply the active filter, so changing a filter re-evaluates already-retained events (FR-034)
- [X] T055 [US3] Add `MessageKindDto` and the `getFilterModel` command in `src-tauri/src/dto.rs` and `src-tauri/src/commands.rs`, so the panel's structure and labels come from Rust rather than being invented in the UI
- [X] T056 [US3] Add the `setFilter` command handler with channel-range validation returning `IpcError::ChannelOutOfRange` in `src-tauri/src/commands.rs`
- [X] T057 [US3] Restrict the stream pump in `src-tauri/src/stream.rs` to filter-passing events only, so suppressed high-rate traffic never crosses the IPC boundary
- [X] T058 [US3] Implement the three-column filter grid with parent/child checkboxes and standalone `System Exclusive` and `Invalid` in `src/components/FilterPanel.tsx`, reproducing `screenshots/filters.png` entry-for-entry in order, all checked by default
- [X] T059 [US3] Implement the `All Channels` / `One Channel` radio pair with the channel field active only under `One Channel`, retaining its value when switching back, in `src/components/FilterPanel.tsx`
- [ ] T060 [US3] Run scenario **S3** in [quickstart.md](./quickstart.md) and confirm SC-004 (≥90% rate reduction) and SC-007 (every filter control demonstrably changes the list)

**Checkpoint**: US1–US3 all work

---

## Phase 6: User Story 4 - Include or exclude events by hexadecimal data prefix (Priority: P4)

**Goal**: Match hex prefixes against raw MIDI bytes, showing only matches or hiding only matches.

**Independent Test**: Enter `B0 07` in include mode and confirm only channel-1 controller 7 events remain; switch to exclude mode and confirm exactly those are the only ones missing.

- [X] T061 [US4] Implement the `HexPrefix` newtype with `parse` normalising case and whitespace and rejecting non-hex characters, in `crates/midi-core/src/domain/ids.rs`
- [X] T062 [US4] Implement `DataPrefixFilter` and `PrefixMode` with nibble-string prefix matching — so odd-length prefixes need no special case — and the empty-entry rule admitting everything in both modes, in `crates/midi-core/src/domain/filter.rs`
- [X] T063 [US4] Add `set_data_prefix_filter` to `crates/midi-core/src/application/monitor.rs` and include the prefix filter in the log view
- [X] T064 [US4] Add the `setDataPrefixFilter` command handler in `src-tauri/src/commands.rs`, failing with `IpcError::MalformedHexPrefix` **without mutating state**, so the previous valid filter stays in effect (FR-041)
- [X] T065 [P] [US4] Implement the prefix entry with include/exclude mode selection and inline invalid-entry reporting in `src/components/HexPrefixFilter.tsx`
- [X] T066 [P] [US4] Surface each event's raw bytes in hexadecimal from `EventDto.rawHex` in `src/components/EventTable.tsx`, so users can see what to type (FR-015)
- [ ] T067 [US4] Run scenario **S4** in [quickstart.md](./quickstart.md) and confirm SC-008 (isolate a message kind in under 20 s; exclude mode yields the exact complement)

**Checkpoint**: US1–US4 all work

---

## Phase 7: User Story 5 - Hide columns that are not of interest (Priority: P5)

**Goal**: Choose which of the five columns are shown, with at least one always visible.

**Independent Test**: Hide Source and Chan and confirm header and rows show only Time, Message, and Data with no leftover gap.

- [X] T068 [US5] Implement `Column` (declaration order = display order) and `ColumnVisibility`, refusing to hide the last visible column with `CoreError::LastColumnVisible`, in `crates/midi-core/src/domain/column.rs`
- [X] T069 [US5] Add `set_columns` to `crates/midi-core/src/application/monitor.rs`, kept out of the log view so visibility can never affect which events are listed (FR-046)
- [X] T070 [US5] Add the `setColumnVisibility` command handler returning `()` rather than a snapshot in `src-tauri/src/commands.rs`
- [X] T071 [P] [US5] Implement the column visibility menu on the table header in `src/components/ColumnMenu.tsx` — additive surface that must not alter the header at rest (Constitution VII)
- [X] T072 [US5] Make `src/components/EventTable.tsx` render only visible columns, reflowing the remainder and restoring a re-shown column to its original position
- [ ] T073 [US5] Run scenario **S5** in [quickstart.md](./quickstart.md)

**Checkpoint**: All five user stories independently functional

---

## Phase 8: Polish & Cross-Cutting Concerns

- [X] T074 Implement `SettingsRepository` over `tauri-plugin-store` writing `settings.json` in `src-tauri/src/settings.rs`, naming the **Repository** pattern and the problem it solves — keeping the plugin and `serde_json::Value` out of the Tauri-free core
- [X] T075 Persist sources, filters, prefix filter and mode, column visibility, and retention limit after each successful mutating command, and restore them at startup via `getSettings` in `src-tauri/src/lib.rs` — retained events are never persisted (FR-047)
- [X] T076 [P] Handle the remaining edge cases from [spec.md](./spec.md): all categories unchecked, window resized very narrow, hex prefix longer than a message, Clear pressed mid-stream
- [X] T077 [P] Audit documentation coverage across `crates/midi-core/src/`, `src-tauri/src/`, and `src/` — every public item and module documented, explaining *why*, with the Principle II hand-roll justification present in `domain/message.rs`
- [X] T078 [P] Remove dead code, replace any remaining magic literals with named constants, and confirm no `unwrap`/`expect` outside `main.rs` and no `any` in the webview
- [X] T079 Run `cargo clippy --workspace --all-targets` and `cargo fmt --check` and resolve every warning — warnings are errors here
- [X] T080 Run `npx tsc --noEmit` under strict mode and resolve every error
- [X] T081 Confirm no test artifacts exist anywhere: no `tests/` directory, `#[cfg(test)]` module, test `[dev-dependencies]`, `*.test.ts`/`*.spec.ts`, or runnable Rust doc example (Verification Standard)
- [ ] T082 Perform the full screenshot fidelity pass from **S6** in [quickstart.md](./quickstart.md) against all three reference images — labels verbatim, control types, order, indentation, first-launch defaults, nothing added — documenting any platform-forced deviation in the relevant module's docs
- [ ] T083 Run scenario **S7** in [quickstart.md](./quickstart.md) and confirm SC-005: ≥500 events/s sustained with every control responding within a quarter second and smooth scrolling at maximum retention
- [ ] T084 Walk the complete [quickstart.md](./quickstart.md) end to end on a machine with no MIDI devices attached, confirming SC-010 (settings persist) and SC-011 (runs with no MIDI hardware)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: T001 blocks everything — the Rust toolchain is absent on this machine
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **User Stories (Phases 3–7)**: All depend on Foundational
- **Polish (Phase 8)**: Depends on all desired stories

### User Story Dependencies — read this before parallelising

Unlike the template's default assumption, these stories are **not** mutually independent:

- **US1 (P1)**: Depends only on Foundational. The MVP.
- **US2 (P2)**: **Depends on US1.** Source selection is verified by watching the Source column change,
  and T044 extends the `EventLog` that T036 creates.
- **US3 (P3)**: **Depends on US1.** T053 extends the same `EventLog`.
- **US4 (P4)**: **Depends on US3** — `DataPrefixFilter` composes into the `FilterSettings` that T052
  introduces.
- **US5 (P5)**: **Depends on US1** — the table it hides columns in.

This is inherent to the feature rather than a design flaw: every story is a lens on one event list.
Each story remains independently *verifiable* once US1 exists, which is what the checkpoints assert.
The honest completion order is **US1 → US2 → US3 → US4 → US5**, with US2 and US5 available to run in
parallel with US3/US4 by a second developer once US1 lands.

### Within each user story

- Domain types → application service → IPC command → React component → scenario walkthrough
- The scenario task (T043, T051, T060, T067, T073) closes each story and MUST actually be performed —
  it is the only verification this project has

### Parallel Opportunities

- Setup: T006–T011 all `[P]`
- Foundational: T013, T014, T015, T018 (domain primitives, distinct files); T020 alongside T019;
  T024 alongside T025; T031, T032, T035 (webview shell, distinct files)
- US1: T039 and T040 (distinct components)
- US2: T048 before T049 consumes it
- US4: T065 and T066 (distinct components)
- Polish: T076, T077, T078 `[P]`; T079–T084 are sequential gates

---

## Parallel Example: Foundational domain primitives

```bash
# After T012 establishes the module tree, launch these together:
Task: "Define screenshot-derived constants in crates/midi-core/src/constants.rs"
Task: "Implement newtypes in crates/midi-core/src/domain/ids.rs"
Task: "Define CoreError in crates/midi-core/src/application/error.rs"
Task: "Implement Source, SourceGroup, CheckState in crates/midi-core/src/domain/source.rs"
```

## Parallel Example: User Story 1

```bash
# Distinct components, no shared file:
Task: "Implement virtualized event table in src/components/EventTable.tsx"
Task: "Implement retention row in src/components/RetentionRow.tsx"
```

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. Phase 1 Setup — **starting with T001, installing Rust**
2. Phase 2 Foundational (blocks everything)
3. Phase 3 US1
4. **STOP and VALIDATE**: run quickstart S1
5. A working MIDI monitor with a live list, retention, and Clear — demonstrable on its own

### Incremental Delivery

1. Setup + Foundational → window launches, events flow
2. US1 → **MVP**, validate with S1
3. US2 → validate with S2
4. US3 → validate with S3
5. US4 → validate with S4
6. US5 → validate with S5
7. Polish → S6 fidelity pass, S7 throughput, full quickstart

### Parallel Team Strategy

1. Everyone on Setup + Foundational
2. One developer takes US1 alone — it blocks the rest
3. Once US1 lands: Developer A on US3 → US4 (the filter chain), Developer B on US2 and US5
4. Polish together

---

## Notes

- `[P]` = different files, no dependencies on incomplete tasks
- `src/bindings.ts` is **generated output** — regenerate it whenever a command, DTO, or error changes
  (Definition of Done gate 9); never hand-edit it
- Commit after each task or logical group
- Every checkpoint is a real stopping point — the app compiles and runs there
- There is no test suite by design: `cargo clippy`, `tsc --noEmit`, and the quickstart scenarios are
  the entire verification story, so skipping a scenario task means shipping unverified work
