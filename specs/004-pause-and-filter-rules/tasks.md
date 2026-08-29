---

description: "Task list for 004-pause-and-filter-rules"
---

# Tasks: Pause the View and Build Several Data Prefix Rules

**Input**: Design documents from `specs/004-pause-and-filter-rules/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/)

**Tests**: **None.** The constitution's Verification Standard prohibits a test suite in this
repository — no test files, no test dependencies, no doctests, no CI test steps. Per the Governance
clause, the task template's test categories are skipped and that skip is recorded here. Verification
is `cargo clippy` clean, `cargo fmt --check` clean, `tsc --noEmit` clean, and the manual pass in
[quickstart.md](./quickstart.md).

**Organization**: Tasks are grouped by user story so each can be implemented, verified by hand, and
demonstrated on its own.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel — different file, no dependency on an incomplete task
- **[Story]**: US1, US2, US3 — maps to the user stories in [spec.md](./spec.md)
- Every task names the exact file it touches

## Path Conventions

Tauri desktop application, per [plan.md](./plan.md):

- **Core** (domain + application): `crates/midi-core/src/`
- **Shell** (IPC surface): `src-tauri/src/`
- **Webview**: `src/`

---

## Phase 1: Setup

**Purpose**: Establish a clean baseline and preserve what the migration check needs.

- [X] T001 Run the four gates on an unchanged tree from the repository root — `cargo clippy --workspace --all-targets`, `cargo fmt --check`, `npm run typecheck`, and `npm run tauri dev` — and record that all four pass, so any failure later in this feature is attributable to this feature
- [X] T002 Copy the settings document written by the current build to a safe location outside the repository, for the migration check in [quickstart.md](./quickstart.md) §5 — macOS `~/Library/Application Support/com.midimonitor.mock/settings.json`, Windows `%APPDATA%\com.midimonitor.mock\settings.json`. If none exists, run the app first, enter `90 B007` in `Data starts with`, choose `Hide`, quit, then copy

**Checkpoint**: Baseline green, and a pre-004 settings file is preserved.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that must exist before any user story.

**None required.** This is a deliberate finding, not an omission: the two capabilities touch disjoint
types. Pause adds `CaptureState` and a gate in `Monitor::ingest`; the rule list rewrites
`DataPrefixFilter`. They share no type, no command, and no component, and the one file both
eventually edit — `src/ipc.ts` — is edited in different functions. The regenerated `src/bindings.ts`
is produced by a debug build and needs no preparatory task.

**Checkpoint**: User story work can begin immediately, and US1 and US2 can be worked in parallel.

---

## Phase 3: User Story 1 — Freeze the list to read what has already arrived (Priority: P1) 🎯 MVP

**Goal**: A `Pause` / `Resume` control that stops the list moving and guarantees every already-received
event survives, however long the pause and however dense the traffic.

**Independent Test**: With a source streaming continuously, pause; confirm the rows stop, then wait
past what would fill the retention cap several times and confirm the first visible row's timestamp
and the retained count are unchanged; resume and confirm the list is live again. Delivers the whole of
User Story 1 with no part of US2 or US3 built.

### Implementation for User Story 1

- [X] T003 [P] [US1] Create `crates/midi-core/src/application/capture.rs` with `CaptureState { Running, Paused }`, deriving `Debug, Clone, Copy, PartialEq, Eq` and **no** serde derives, with a module doc and a type doc stating why pausing stops retention rather than display (the cap would otherwise evict exactly the rows the user paused to read) and why it is never persisted
- [X] T004 [US1] Add `pub mod capture;` to `crates/midi-core/src/application/mod.rs` and mention the new module in that file's overview
- [X] T005 [US1] In `crates/midi-core/src/application/monitor.rs`, add the `capture: CaptureState` field defaulting to `Running` in `Monitor::new`, plus `capture_state()` and `set_capture_state()`; document that `set_capture_state` touches nothing else — no log change, no catalogue change, no port opened or closed (FR-008)
- [X] T006 [US1] In `crates/midi-core/src/application/monitor.rs`, gate `Monitor::ingest` on the capture state — return `None` before the selection check and before `log.push` when paused — and extend its doc comment with why the gate is first: nothing is retained, so the retention cap cannot evict a frozen row (FR-009, FR-010)
- [X] T007 [US1] Confirm `Monitor::persisted_settings` still excludes the capture state and that `crates/midi-core/src/application/settings.rs` is unchanged, so a relaunched monitor is running (FR-012)
- [X] T008 [US1] In `src-tauri/src/dto.rs`, add `CaptureStateDto` as a tagged union mirroring `CaptureState` one variant for one, with `From` conversions matched exhaustively and no catch-all arm
- [X] T009 [US1] In `src-tauri/src/dto.rs`, add `capture_state` to `SnapshotDto` and populate it in `SnapshotDto::from_monitor`; document why it is separate from `monitoring` — "no source selected" and "the user paused" call for different actions and must not be conflated
- [X] T010 [US1] In `src-tauri/src/commands.rs`, add the `set_capture_state` handler per [contracts/ipc-commands.md](./contracts/ipc-commands.md): delegate to the monitor, call `state.pump.discard_pending()` so a batch queued up to 16 ms earlier cannot land after the pause, and **do not** persist
- [X] T011 [US1] Register `commands::set_capture_state` in `collect_commands!` in `src-tauri/src/lib.rs`
- [X] T012 [US1] Run a debug build (`npm run tauri dev`, then quit) to regenerate `src/bindings.ts`, and confirm the file is regenerated rather than hand-edited
- [X] T013 [US1] In `src/store.ts`, add `captureState: CaptureStateDto` to `MonitorState` with a TSDoc line, initialise it to running, and set it in `applySnapshot`; leave `appendBatch` unchanged and note why — while paused nothing is streamed, and a webview-side guard would put a rule on the wrong side of the IPC boundary
- [X] T014 [US1] In `src/ipc.ts`, add the `setCaptureState` wrapper alongside the other mutation wrappers, using `mutate(..., false)` since the capture state changes no panel structure
- [X] T015 [P] [US1] Create `src/components/CaptureRow.tsx` — the `Pause` / `Resume` control showing which state the monitor is in (FR-002), stating while paused that arriving events are not being recorded, with a TSDoc block explaining why that sentence is there (the gap is deliberate and the user must be able to anticipate it)
- [X] T016 [US1] In `src/App.tsx`, render `<CaptureRow />` between `<RetentionRow />` and the error banner, and record in the component doc why it is a new row rather than a button beside `Clear` — that row is depicted in `screenshots/data.png` and constitution VII requires new capability to earn new surface
- [X] T017 [P] [US1] In `src/components/EventTable.tsx`, extend `emptyReason` with the paused case so a paused, empty list never reads `Waiting for events…`, keeping the "no sources selected" message ahead of it so a paused monitor with nothing selected still says the more actionable thing
- [ ] T018 [US1] Verify User Story 1 by hand: [quickstart.md](./quickstart.md) §1 (pause keeps what arrived, including the wait past the retention cap) and §2 (filters, `Clear`, and source selection all behave while paused)

**Checkpoint**: Pause is fully functional and demonstrable. This is the MVP — it can ship without any
part of User Story 2 or 3.

---

## Phase 4: User Story 2 — Keep a list of data prefix rules (Priority: P1)

**Goal**: `Data starts with` becomes a list of rules, each with its own kind, added and deleted one at
a time, surviving a restart and migrating a filter saved by an earlier build.

**Independent Test**: Add two `Hide` rules, confirm both are listed and both are in effect, delete one
and confirm the other still applies on that same interaction; restart and confirm the list returns
unchanged.

**Note on the split with User Story 3**: `add` refuses a **repeated prefix** in this phase, because
the prefix being unique is what lets deletion key on it (research D3) — the list would not be
well-defined without it. The **contradiction** case (overlapping prefixes of opposite kinds) and the
full quality of the explanation belong to User Story 3.

### Implementation for User Story 2

- [X] T019 [P] [US2] In `crates/midi-core/src/application/error.rs`, add the `DuplicatePrefixRule { prefix, existing_kind }` and `UnknownPrefixRule { prefix }` variants, each documenting the condition that produces it and what the caller should do — the same standard every existing variant meets
- [X] T020 [US2] In `crates/midi-core/src/domain/filter.rs`, add `DataPrefixRule { prefix: HexPrefix, kind: PrefixMode }` with `matches(&self, raw_hex: &str)` and a private `overlaps` helper, deriving `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`; document why the prefix is the rule's identity and why there is no rule id
- [X] T021 [US2] In `crates/midi-core/src/domain/filter.rs`, rewrite `DataPrefixFilter` to `{ rules: Vec<DataPrefixRule> }` and reimplement `admits` per [data-model.md](./data-model.md) §2 — empty list admits everything; any `Exclude` match hides; with no `Include` rule everything else shows; otherwise an `Include` rule must match
- [X] T022 [US2] In `crates/midi-core/src/domain/filter.rs`, implement `DataPrefixFilter::add` with the duplicate-prefix refusal only, and `remove` returning `UnknownPrefixRule` when the prefix is not listed; both must validate fully before mutating so a refusal leaves the list byte-identical (FR-031)
- [X] T023 [US2] In `crates/midi-core/src/domain/filter.rs`, add the private untagged `StoredDataPrefixFilter` enum, the `From` conversion that maps a legacy `{ mode, prefixes }` document to rules of that mode **deduplicating by prefix**, and `#[serde(from = "StoredDataPrefixFilter")]` on `DataPrefixFilter` — per [contracts/persisted-settings.md](./contracts/persisted-settings.md); document why the existing "unreadable means absent" fallback is not an acceptable substitute
- [X] T024 [US2] In `crates/midi-core/src/application/monitor.rs`, replace `set_data_prefix_filter` with `add_prefix_rule` and `remove_prefix_rule`, both returning `Result<(), CoreError>`; confirm `set_filter` still preserves the data filter when message kinds or the channel change
- [X] T025 [US2] In `src-tauri/src/error.rs`, add the `DuplicatePrefixRule` and `UnknownPrefixRule` wire variants and their arms in `From<CoreError> for IpcError`, keeping that match exhaustive with no catch-all
- [X] T026 [US2] In `src-tauri/src/dto.rs`, add `DataPrefixRuleDto { prefix, kind }`, replace `prefix_mode` and `prefixes` on `FilterViewDto` with `rules: Vec<DataPrefixRuleDto>`, and update the `filter_view` builder to emit them
- [X] T027 [US2] In `src-tauri/src/commands.rs`, add `add_data_prefix_rule` and `remove_data_prefix_rule` per [contracts/ipc-commands.md](./contracts/ipc-commands.md) — parse the prefix first, then delegate, then `discard_pending`, then persist — and **delete** `set_data_prefix_filter` rather than deprecating it
- [X] T028 [US2] In `src-tauri/src/lib.rs`, register the two new commands in `collect_commands!` and remove `commands::set_data_prefix_filter`
- [X] T029 [US2] Run a debug build to regenerate `src/bindings.ts`, and confirm `tsc --noEmit` now fails at the stale webview call sites — that failure is the contract doing its job
- [X] T030 [US2] In `src/ipc.ts`, replace the `setDataPrefixFilter` wrapper with `addDataPrefixRule` and `removeDataPrefixRule` (both `mutate(..., true)` so the panel refreshes), and add the `duplicatePrefixRule` and `unknownPrefixRule` arms to `describeError`
- [X] T031 [US2] Create `src/components/DataPrefixRules.tsx` — the `Data starts with` entry field, the `Show only` / `Hide` choice, an `Add` action, and the list of current rules each with a delete control; keep the labels `Data starts with`, `Show only`, and `Hide` verbatim, and document why the entry is committed on `Add` rather than applied per keystroke
- [X] T032 [US2] Delete `src/components/HexPrefixFilter.tsx` and render `<DataPrefixRules />` in its place in `src/components/FilterPanel.tsx` — no dead code left beside its replacement (Principle I)
- [ ] T033 [US2] Verify User Story 2 by hand: [quickstart.md](./quickstart.md) §3 (add, delete, normalisation, malformed and empty entries) and §5 (rules survive a restart, and the preserved pre-004 settings file migrates with its source selections, columns, and retention limit intact)

**Checkpoint**: The rule list is fully functional. Repeated prefixes are already refused; overlapping
opposite-kind rules are not yet.

---

## Phase 5: User Story 3 — Be told, and told why, when a rule clashes (Priority: P2)

**Goal**: An addition that contradicts a listed rule is refused with a message naming both the entered
prefix and the existing rule, leaving the list and the visible rows exactly as they were.

**Independent Test**: With `Show only 90` listed, attempt `Hide 9`, `Hide 9012`, and `Hide 90`; each is
refused with an explanation naming both rules, and the table does not change by a single row. Confirm
`Show only 9` alongside `Show only 90` is still **accepted**.

### Implementation for User Story 3

- [X] T034 [P] [US3] In `crates/midi-core/src/application/error.rs`, add `ContradictoryPrefixRule { prefix, kind, existing_prefix, existing_kind }`, documenting why it is separate from `DuplicatePrefixRule` — the remedies differ, delete the duplicate versus narrow one of the overlapping pair
- [X] T035 [US3] In `crates/midi-core/src/domain/filter.rs`, extend `DataPrefixFilter::add` with the overlap-across-kinds refusal, documenting that same-kind overlaps stay permitted because they are redundant rather than contradictory (FR-028) and that the rule is symmetric by construction (FR-029)
- [X] T036 [US3] In `crates/midi-core/src/domain/filter.rs`, document on `admits` the invariant this phase completes — two prefixes match the same bytes only when one is a prefix of the other, and `add` now refuses that pair across kinds, so the `Exclude` and `Include` tests can never disagree and no precedence clause is needed (FR-023) — together with the consequence at FR-024, that `Hide` rules are inert while any `Show only` rule is listed, stated as intended behaviour so it is not "fixed" later
- [X] T037 [US3] In `src-tauri/src/error.rs`, add the `ContradictoryPrefixRule` wire variant and its `From<CoreError>` arm
- [X] T038 [US3] Run a debug build to regenerate `src/bindings.ts`, and confirm `describeError`'s exhaustive switch now fails to compile until the new variant is handled
- [X] T039 [US3] In `src/ipc.ts`, add the `contradictoryPrefixRule` arm to `describeError`, and check both clash sentences name the entered prefix **and** the existing rule with its kind, so the user can act without going to read the list (FR-030, SC-006)
- [X] T040 [US3] In `src/components/DataPrefixRules.tsx`, keep the entry text and the chosen kind in the field after a refusal so the user can correct and retry, and confirm the shared error banner is dismissible and blocks nothing else in the window (FR-032)
- [ ] T041 [US3] Verify User Story 3 by hand: [quickstart.md](./quickstart.md) §4 — every row of the clash table, plus the three checks on each refusal and the delete-then-re-add recovery

**Checkpoint**: All three user stories are independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T042 [P] Update `README.md` — that the list can be paused without losing what has arrived (and that traffic during a pause is not recorded), and that the data filter is now a list of `Show only` / `Hide` rules added and deleted one at a time
- [X] T043 Audit the seven doc comments named in [data-model.md](./data-model.md) §7 against Principle III — each must state a *why* the signature does not, and every new `pub` item and both new modules must carry one
- [X] T044 Sweep the diff for Principle I violations: no `unwrap`/`expect`/`panic` outside `main`, no `any` or unchecked casts in the webview, no unnamed magic values, no commented-out or dead code left from the removed prefix filter
- [X] T045 Run the four gates from the repository root — `cargo clippy --workspace --all-targets` (zero warnings), `cargo fmt --check`, `npm run typecheck`, and a successful `npm run tauri dev`
- [ ] T046 Compare the running window side by side with `screenshots/data.png` and `screenshots/filters.png` per [quickstart.md](./quickstart.md) §6 — no depicted control relabelled, reordered, restyled, or given a neighbour inside its row (FR-035, SC-008)
- [X] T047 Confirm no test artifacts were introduced — no `tests/` directory, no `#[cfg(test)]`, no `*.test.ts` or `*.spec.ts`, no test dependency in `Cargo.toml` or `package.json`, no runnable doctest fences (Verification Standard)
- [ ] T048 Run the second-platform pass per [quickstart.md](./quickstart.md) §7 — build, launch, and repeat §1, §3, and §4 briefly, confirming nothing in this feature behaves differently and the `Data` column's fidelity note on Windows is unaffected

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies. T002 must happen **before** any code change, or the pre-004 settings file is gone
- **Foundational (Phase 2)**: empty — see the phase note. Nothing blocks the user stories
- **User Story 1 (Phase 3)**: independent of US2 and US3 entirely
- **User Story 2 (Phase 4)**: independent of US1. Can be built first if the rule list is wanted before pause
- **User Story 3 (Phase 5)**: **depends on User Story 2** — it extends `DataPrefixFilter::add`, which US2 creates. This is the one real cross-story dependency and it is inherent, not accidental
- **Polish (Phase 6)**: after whichever stories are being shipped

### Within Each User Story

Core before shell before webview, because the contract flows that way: a DTO cannot be written before
the type it mirrors, and the webview cannot compile against bindings that have not been regenerated.
Each story's regeneration task (T012, T029, T038) is the hinge — everything before it is Rust,
everything after it is TypeScript.

### Parallel Opportunities

- **US1 and US2 can be built simultaneously by two people.** They share no file. The only care needed
  is that both edit `src/ipc.ts` and `src/store.ts` in different places, and both regenerate
  `src/bindings.ts` — so those three files want a coordinated merge rather than a simultaneous edit
- Within US1: T003, T015, and T017 are `[P]` — three different files with no dependency between them
- Within US2: T019 is `[P]` against T020 — different files (`error.rs` and `filter.rs`) — but T020
  through T023 all edit `crates/midi-core/src/domain/filter.rs` and must be sequential
- Within US3: T034 is `[P]` against nothing else in its phase; T035 and T036 share `filter.rs`
- Polish: T042 is `[P]`; the remaining tasks are audits and manual passes over the whole tree

## Parallel Example: User Story 1

```text
# Three independent files, no dependencies between them:
Task T003: "Create crates/midi-core/src/application/capture.rs with CaptureState"
Task T015: "Create src/components/CaptureRow.tsx"
Task T017: "Extend emptyReason in src/components/EventTable.tsx with the paused case"
```

T015 and T017 can be written against the contract before the bindings exist; they will not typecheck
until T012 regenerates `src/bindings.ts`, which is expected.

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. Phase 1 — Setup, including preserving the old settings file
2. Phase 3 — User Story 1 (T003–T018)
3. **STOP and VALIDATE**: [quickstart.md](./quickstart.md) §1 and §2
4. Ship. Pause is the primary request and stands entirely on its own

### Incremental Delivery

1. Setup → baseline green
2. US1 → verify §1, §2 → **MVP, shippable**
3. US2 → verify §3, §5 → shippable; the rule list works, repeats are refused
4. US3 → verify §4 → shippable; contradictions are refused and explained
5. Polish → §6, §7, the gates, and the audits

Each increment leaves the application in a state that builds clean and can be demonstrated.

### Two-Person Strategy

- Person A: Phase 3 (US1) end to end
- Person B: Phase 4 (US2), then Phase 5 (US3)
- Merge point: `src/ipc.ts`, `src/store.ts`, and `src/bindings.ts`. Regenerate the bindings once after
  merging rather than reconciling two generated copies

---

## Notes

- `[P]` means a different file with no dependency on an incomplete task
- `src/bindings.ts` is **generated**. If a task ever asks you to edit it by hand, the task is wrong
- Commit after each task or logical group; every commit should leave the gates passing
- The one deliberate behaviour that looks like a bug and is not: with any `Show only` rule listed,
  `Hide` rules change nothing (FR-024). It is documented at `admits` in T036 for exactly this reason
- No test task appears anywhere in this list, and none should be added — see the Tests note at the top
