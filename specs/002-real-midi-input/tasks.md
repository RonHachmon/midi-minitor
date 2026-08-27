---

description: "Task list for Real MIDI Input"
---

# Tasks: Real MIDI Input

**Input**: Design documents from `/specs/002-real-midi-input/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/ipc-contract.md](./contracts/ipc-contract.md),
[quickstart.md](./quickstart.md)

**Tests**: **None.** The constitution's Verification Standard prohibits test files, test dependencies,
doctests, and CI test steps in this repository. No test tasks appear below, and none may be added.
Verification is zero-warning compilation plus the manual pass in [quickstart.md](./quickstart.md).

**Organization**: Grouped by user story so each is independently implementable and verifiable by hand.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story the task serves (US1–US5)
- Exact file paths are given in every task

## Path Conventions

Rust workspace + Tauri shell + React webview, per [plan.md](./plan.md#project-structure):

- `crates/midi-core/` — domain + application. **Never** gains a platform dependency
- `crates/midi-macos/` — **new**, the only crate naming `coremidi`
- `src-tauri/src/` — IPC translation only, no logic
- `src/` — webview

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Stand up the new infrastructure crate so later phases have somewhere to land.

- [X] T001 Add `"crates/midi-macos"` to `[workspace] members` in `Cargo.toml`
- [X] T002 [P] Create `crates/midi-macos/Cargo.toml` — depend on `midi-core` (path) and `coremidi = "0.9"` under `[target.'cfg(target_os = "macos")'.dependencies]`, inherit `version`/`edition`/`rust-version` and `[lints] workspace = true`
- [X] T003 [P] Create `crates/midi-macos/src/lib.rs` with a module doc stating why this is a separate crate rather than code in `src-tauri` — layering (Constitution IV forbids logic in the IPC layer), containment of the macOS-only dependency, and that it occupies the seam `simulator/` did

**Checkpoint**: `cargo build` succeeds with an empty new crate in the workspace.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Domain and application changes every user story depends on. This phase contains the
Constitution VII deviations tracked in [plan.md](./plan.md#complexity-tracking).

**⚠️ CRITICAL**: No user story work can begin until this phase completes.

- [X] T004 [P] Add `SourceKey` enum (`Endpoint(i32)`, `VirtualDestination`) to `crates/midi-core/src/domain/ids.rs` with `Serialize`/`Deserialize`/`Copy`/`Hash`; document why it is **not** `specta::Type` (never crosses IPC, which is what lets `SourceId` stay `u32`) and why `VirtualDestination` is a named variant (our endpoint's OS id is not stable across launches)
- [X] T005 [P] Add `MAX_SYSEX_BYTES = 65_536` to `crates/midi-core/src/domain/constants.rs`, documenting it as CoreMIDI's practical packet-buffer ceiling and the point past which a transfer is reported truncated (FR-018)
- [X] T006 [P] Add `CoreError` variants for port-open failure and MIDI-system unavailability in `crates/midi-core/src/application/error.rs`, each documenting its producing condition and what a caller can do (Constitution III)
- [X] T007 Add `key: SourceKey` and `availability: Availability` fields to `Source`, and the `Availability` enum (`Open`, `Unopenable { detail }`, `Absent`) in `crates/midi-core/src/domain/source.rs` (depends on T004)
- [X] T008 Rework `SourceCatalogue` in `crates/midi-core/src/domain/source.rs`: add `replace(Vec<Source>)` preserving `selected` for known keys, `selected_keys()`, `key_of()`; change `apply_selection` to take `&[SourceKey]`; **rewrite the type doc**, which currently claims the catalogue is fixed at startup with "no add or remove" and is about to become false (depends on T007)
- [X] T009 Create `crates/midi-core/src/domain/decoder.rs` — `MessageDecoder`, `SysexAccumulator`, and the `Decoded` enum (`Message`, `Invalid`, `SysexTruncated`, `SysexIncomplete`). Module doc **must** carry the Principle II justification: `wmidi`, `midi-msg`, and `midly` each return `Err` for exactly the input FR-020 requires as a row (depends on T005, T006)
- [X] T010 Implement the decoder rules in `crates/midi-core/src/domain/decoder.rs`: running status (FR-016); a data byte with no status **and** no running status is `Invalid`, never discarded; Real-Time bytes `0xF8`–`0xFF` interleave inside SysEx without ending it; a non-Real-Time status mid-SysEx emits `SysexIncomplete` and starts the new message; running status cleared by any System message; `abandon()` flushes on disconnect (depends on T009)
- [X] T011 Register `pub mod decoder;` in `crates/midi-core/src/domain/mod.rs` (depends on T009)
- [X] T012 Change the `EventSource` port in `crates/midi-core/src/application/ports.rs`: add `pub type CatalogueSink`, change `start` to `start(&mut self, events: EventSink, catalogue: CatalogueSink)`, add `MidiSystemStatus`. **Delete the `catalogue()` doc claiming "Fixed for the lifetime of the process… callers may cache the result"** and replace it with the Observer rationale from [research.md D8](./research.md#d8-the-eventsource-port-grows-a-catalogue-channel--and-that-is-a-real-edit) (depends on T007)
- [X] T013 [P] Change `PersistedSettings.selected_sources` to `Vec<SourceKey>` in `crates/midi-core/src/application/settings.rs` (depends on T004)
- [X] T014 Extend `Monitor` in `crates/midi-core/src/application/monitor.rs`: add `remembered: Vec<SourceKey>` and `status: MidiSystemStatus`; add `replace_catalogue()` re-applying `remembered` to arriving ports and leaving the `EventLog` untouched; make `persisted_settings()` union live selections with `remembered`; change `new()` so a genuine first run selects everything but unknown ports otherwise start unselected (depends on T008, T012, T013)
- [X] T015 Make `StoreSettingsRepository::load` in `src-tauri/src/settings.rs` treat a malformed or previous-format store as `Ok(None)` rather than an error, documenting that the `selected_sources` shape changed and old simulator ids carry no meaning for real hardware
- [X] T016 Adapt `SimulatedSource` in `crates/midi-core/src/simulator/mod.rs` to the new `start` signature, ignoring the `CatalogueSink`. **Throwaway scaffolding deleted in T045** — annotate it as such so it is never mistaken for permanent work. It exists only to keep the application runnable through Phases 2–4, which the Verification Standard depends on

**Checkpoint**: `cargo clippy -- -D warnings` clean, `npm run typecheck` clean, and the app still runs
on the simulator. Domain foundations are ready.

---

## Phase 3: User Story 2 — Real MIDI ports in the Sources list (Priority: P1) 🎯 MVP

**Goal**: The Sources list shows the machine's actual MIDI input ports, named as macOS names them.

**Independent Test**: Compare the Sources list against Audio MIDI Setup — name for name, count for
count. Disconnect everything and confirm the list is empty and says so.

> **Sequenced before US1 despite equal priority.** Enumeration is a hard prerequisite for opening a
> port, so events cannot arrive before the catalogue exists. Both are P1 and together form the MVP.

- [X] T017 [P] [US2] Create `crates/midi-macos/src/endpoints.rs` — enumerate `coremidi::Sources`, read `Properties::unique_id()` (`i32`), `display_name()`, and `offline()` into an `EndpointSnapshot`
- [X] T018 [US2] Map `EndpointSnapshot` → domain `Source` in `crates/midi-macos/src/endpoints.rs`: `SourceKey::Endpoint(unique_id)`, `group = Some(SourceGroupId::MidiSources)`, name used **verbatim** with no cleanup or deduplication (FR-004), one entry per port not per device (FR-005) (depends on T017)
- [X] T019 [US2] Create `crates/midi-macos/src/source.rs` — `CoreMidiSource` holding the `Client`, `ports`, `decoders`, and `next_id`; implement `EventSource::catalogue()` from T018 and a `stop()` that closes ports (depends on T018)
- [X] T020 [US2] Report `MidiSystemStatus::Unavailable { detail }` when client creation fails, in `crates/midi-macos/src/source.rs` — distinct from an empty catalogue (FR-008, SC-016) (depends on T019)
- [X] T021 [US2] Add `MidiSystemStatusDto` (tagged union), `SourceDto.unavailable: Option<String>`, and `CatalogueDto { groups, midi_system }` to `src-tauri/src/dto.rs`; update `source_groups()` to populate availability (depends on T020)
- [X] T022 [US2] Change `get_catalogue` in `src-tauri/src/commands.rs` to return `CatalogueDto` (depends on T021)
- [X] T023 [US2] Rewire `src-tauri/src/lib.rs` to construct `CoreMidiSource` in place of `SimulatedSource`, and **rewrite the module doc** — its claim that swapping in real MIDI "is a change to that one line" is about to be false (depends on T019, T022)
- [X] T024 [P] [US2] Update `getCatalogue` types and calls in `src/ipc.ts` for `CatalogueDto` (depends on T022)
- [X] T025 [US2] Hold `midiSystem` in `src/store.ts` and render the real list in `src/components/SourcesPanel.tsx`, including the "no MIDI devices found" empty state (FR-007) and the system-unavailable state (FR-008) — new text only, **no existing control relabelled, reordered, or restyled** (Constitution VI) (depends on T024)

**Checkpoint**: Sources list matches Audio MIDI Setup. Quickstart Scenario 2 passes.

---

## Phase 4: User Story 1 — Watch messages from a real MIDI device (Priority: P1) 🎯 MVP

**Goal**: Real bytes from real devices in the event table, including the malformed ones.

**Independent Test**: Play a device; rows appear within a quarter second and stop when you stop.
Compare raw hex byte-for-byte against a known transmitted sequence.

- [X] T026 [US1] Open **one `InputPort` per selected source** in `crates/midi-macos/src/source.rs`, each closure capturing its own `SourceId` for attribution — the cost of the MIDI 1.0 `PacketList` path chosen for byte fidelity ([D2](./research.md#d2-midi-10-packetlist-not-midi-20-eventlist)) (depends on T019)
- [X] T027 [US1] Hold one `MessageDecoder` per source in `crates/midi-macos/src/source.rs`, feeding each `Packet::data()` through it and emitting `Decoded` outcomes in arrival order (depends on T026, T010)
- [X] T028 [US1] Convert `Decoded` → `MidiEvent` in `crates/midi-macos/src/source.rs`, carrying raw bytes exactly as received (FR-014) and timestamping via the existing `Clock` port ([D7](./research.md#d7-arrival-time-from-the-existing-clock-port)) (depends on T027)
- [X] T029 [US1] Map `Decoded::Invalid` to `MidiMessage::Invalid` so malformed bytes reach the table with their raw content (FR-020) — the requirement that disqualified `midir` (depends on T028)
- [X] T030 [US1] Handle SysEx outcomes in `crates/midi-macos/src/source.rs`: `SysexTruncated` reports the **true** size (FR-018), and `abandon()` is called on port disconnect so an incomplete transfer is reported rather than held forever (FR-019) (depends on T028)
- [X] T031 [US1] Set `Availability::Unopenable { detail }` when a port fails to open, continuing to monitor every port that did open (FR-013), in `crates/midi-macos/src/source.rs` (depends on T026)
- [X] T032 [US1] Count and surface dropped events in `src-tauri/src/stream.rs` — `EventPump::push` currently drops silently on lock contention, which FR-023 no longer permits

**Checkpoint**: 🎯 **MVP complete** once Phase 8 also runs. Quickstart Scenarios 3 and 7 pass.

---

## Phase 5: User Story 3 — Keep monitoring across plug and unplug (Priority: P2)

**Goal**: The Sources list follows the hardware, and selections survive a replug.

**Independent Test**: Unplug a selected, transmitting device — the list updates within two seconds, the
app keeps running, and its existing rows stay. Plug it back in: still selected, events resume.

- [X] T033 [US3] Create `crates/midi-macos/src/notifications.rs` — `Client::new_with_notifications`, translating setup-change notifications into a re-enumeration
- [X] T034 [US3] Debounce the notification burst in `crates/midi-macos/src/notifications.rs` — macOS emits several notifications per physical plug event, and one push per notification would rebuild the Sources panel repeatedly (depends on T033)
- [X] T035 [US3] **Order client creation first, on the main thread**, in `src-tauri/src/lib.rs`'s `setup`, before any other MIDI call. Document that this ordering is load-bearing: macOS fixes the notification thread at first client creation, so a later refactor moving a port creation above it breaks hot-plug silently ([D4](./research.md#d4-hot-plug-via-clientnew_with_notifications-created-on-the-main-thread-first)) (depends on T033)
- [X] T036 [US3] Wire the `CatalogueSink` to `Monitor::replace_catalogue` in `src-tauri/src/lib.rs`, opening ports for arrivals and closing them for departures while leaving retained events intact (FR-011) (depends on T035, T014)
- [X] T037 [US3] Add a catalogue pump to `src-tauri/src/stream.rs` — a separate `Channel<CatalogueDto>` from the event stream, since the two have unrelated rates and payloads (depends on T021)
- [X] T038 [US3] Add `subscribe_catalogue` command to `src-tauri/src/commands.rs`, returning the current `CatalogueDto` in the same round trip so the webview is never subscribed-but-unrendered; register it in `specta_builder()` in `src-tauri/src/lib.rs` (depends on T037)
- [X] T039 [US3] Hold the catalogue pump in `src-tauri/src/state.rs` (depends on T037)
- [X] T040 [US3] Add `MutationResultDto { snapshot, catalogue }` in `src-tauri/src/dto.rs` and return it from `set_source_selected` and `set_group_selected` in `src-tauri/src/commands.rs` — selecting a source can now surface an open failure, which `SnapshotDto` has no field for. Leave the filter, prefix, retention, clear, and column commands returning `SnapshotDto` unchanged (depends on T021)
- [X] T041 [P] [US3] Add `subscribeCatalogue` and the changed mutation return types to `src/ipc.ts` (depends on T038, T040)
- [X] T042 [US3] Subscribe to the catalogue channel in `src/App.tsx` alongside the event channel, and apply pushed catalogues in `src/store.ts` (depends on T041)

**Checkpoint**: Quickstart Scenarios 4 and 5 pass. **Validate here, not at the end** — this is the
plan's top risk.

---

## Phase 6: User Story 4 — Receive MIDI from other applications (Priority: P3)

**Goal**: The monitor appears as a destination other apps can send to.

**Independent Test**: Enable the entry, select the monitor as a destination in another MIDI app, send
messages, and see them listed.

- [X] T043 [US4] Create the virtual destination via `Client::virtual_destination` (MIDI 1.0 `PacketList`) in `crates/midi-macos/src/source.rs`, exposed as the standalone `Act as a destination for other programs` source with `SourceKey::VirtualDestination` and `group: None` (depends on T019)
- [X] T044 [US4] Feed its packets through their own `MessageDecoder` and attribute events to that source, in `crates/midi-macos/src/source.rs` (depends on T043, T027)
- [X] T045 [US4] Create and tear down the destination as the entry is checked and unchecked (FR-026), in `crates/midi-macos/src/source.rs` (depends on T043)
- [X] T046 [US4] Report creation failure as `Availability::Unopenable { detail }` without preventing anything else from working (FR-027), in `crates/midi-macos/src/source.rs` (depends on T043)

**Checkpoint**: Quickstart Scenario 6 passes.

---

## Phase 7: User Story 5 — The spy group explains itself (Priority: P4)

**Goal**: `Spy on output to destinations` stays on screen, empty, and states that it is unavailable.

**Independent Test**: Expand Sources — the group is present with its verbatim label, has no children,
and carries an explanation. No event is ever attributed to it.

> Deferred capability, delivered honestly. The group is **retained** because Constitution VI forbids
> deleting a control the screenshots depict; FR-029 forbids putting placeholder sources in it.

- [X] T047 [US5] Add `unavailable_reason: Option<String>` to `SourceGroupDto` in `src-tauri/src/dto.rs` (depends on T021)
- [X] T048 [US5] Emit the `spyOnOutput` group from `source_groups()` in `src-tauri/src/dto.rs` — always present, always empty `sources`, always carrying `"Observing output to destinations is not available in this version."`, keeping its verbatim label and its position after the standalone row (depends on T047)
- [X] T049 [US5] Render the group heading with its explanation where children would be, in `src/components/SourcesPanel.tsx` — no placeholder rows (FR-029) (depends on T048)

**Checkpoint**: Quickstart Scenario 2's spy-group check passes.

---

## Phase 8: Remove All Simulation (MANDATORY — the feature's defining requirement)

**Purpose**: FR-001, FR-002, and SC-014. **Not polish and not optional.** This phase is what makes the
feature true; everything before it only makes real input work alongside the simulator.

**Sequenced last deliberately** so every earlier phase had a working application to compare against, and
so deletion is one irreversible commit rather than a rewrite performed blind.

- [X] T050 Delete the directory `crates/midi-core/src/simulator/` entirely — `mod.rs`, `catalogue.rs`, and `traffic.rs`. This also removes the throwaway adapter from T016
- [X] T051 Remove `pub mod simulator;` from `crates/midi-core/src/lib.rs` and update its module doc, which currently describes the simulator as the crate's event source (depends on T050)
- [X] T052 Remove the `rand` workspace dependency and its explanatory comment from `Cargo.toml`, and from `crates/midi-core/Cargo.toml` — it existed solely to seed the traffic generator (depends on T050)
- [X] T053 Verify no fabricated path survives: run `rg -i 'simul|mock|fake|dummy|synthetic' crates/ src-tauri/src/ src/` and `rg 'rand' Cargo.toml crates/*/Cargo.toml src-tauri/Cargo.toml`. Matches in prose explaining the removal are fine; **matches in code are not** (SC-014) (depends on T052)
- [X] T054 Run Quickstart Scenario 1 — no devices attached, IAC disabled, application idle for five minutes. **A single row appearing is a failure**, and means a generated path survived (depends on T053)

**Checkpoint**: 🎯 **MVP complete.** Nothing in the product can produce an event it did not receive.

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: The constitution's Definition of Done. Documentation is a completion criterion here, not a
follow-up — with no test suite it is the primary record of intent (Constitution III).

- [X] T055 [P] Confirm the three stale doc comments identified in [plan.md](./plan.md#documentation-debt) were rewritten, not merely extended: `SourceCatalogue` (T008), `EventSource::catalogue` (T012), and the `src-tauri/src/lib.rs` module doc (T023). A doc that lies is a defect of the same kind as a broken test
- [X] T056 [P] Audit rustdoc coverage across `crates/midi-core/`, `crates/midi-macos/`, and `src-tauri/src/` — every new or modified public item and every module states **why**, not what (Constitution III)
- [X] T057 [P] Audit TSDoc coverage across `src/ipc.ts`, `src/store.ts`, `src/App.tsx`, and `src/components/SourcesPanel.tsx`
- [X] T058 Verify no `unwrap`/`expect`/`panic!`/`todo!` outside `main`, no `any` or unchecked casts at the invoke boundary, no dead code, and no unnamed magic values (Constitution I)
- [X] T059 Confirm `src/bindings.ts` regenerated from the Rust declarations and that no hand-maintained TypeScript mirrors the contract (Constitution IV)
- [X] T060 Run the full gate: `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `npm run typecheck`
- [ ] T061 Run Quickstart Scenario 10 — side-by-side against `screenshots/main-screen.png`, `sources.png`, and `filters.png`. A relabelled, reordered, restyled, or repurposed control fails the gate regardless of how well it works (Constitution VI)
- [ ] T062 Run Quickstart Scenario 8 — every behaviour from feature 001 still works against real traffic, including every Filter checkbox demonstrably changing the list (SC-012)
- [ ] T063 Run Quickstart Scenario 9 — ≥ 500 messages/second across ≥ 2 sources; controls responsive within 250 ms, arrival order preserved, retention cap exact, nothing dropped without a trace
- [ ] T064 Run Quickstart Scenario 11 — deny MIDI permission and confirm the app says access was refused, distinctly from "no devices found" (SC-016)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies
- **Foundational (Phase 2)**: depends on Setup — **blocks every user story**
- **US2 (Phase 3)**: depends on Foundational
- **US1 (Phase 4)**: depends on US2 — enumeration is a hard prerequisite for opening a port
- **US3 (Phase 5)**: depends on US2 (catalogue) and US1 (ports to reopen on reconnect)
- **US4 (Phase 6)**: depends on Foundational + T019/T027; independent of US3
- **US5 (Phase 7)**: depends only on T021; **can be done any time after Phase 3**
- **Simulation removal (Phase 8)**: depends on US1 + US2 delivering real events
- **Polish (Phase 9)**: depends on all desired stories

### Critical path

```text
T001–T003 → T004–T016 → T017–T025 (US2) → T026–T032 (US1) → T050–T054 → MVP
```

### Parallel Opportunities

- **Phase 1**: T002, T003 together
- **Phase 2**: T004, T005, T006 together (three different files); then T013 alongside T009/T010
- **Phase 3**: T017 starts immediately; T024 parallel with backend work once T022 lands
- **Phase 5**: T041 parallel with T039/T040
- **Phase 7 (US5)**: T047–T049 are independent of US3 and US4 — a second person can take the whole story after Phase 3
- **Phase 9**: T055, T056, T057 together

### Story-level parallelism

Genuinely limited here, and worth saying plainly: US1 and US2 are two halves of one mechanism, and US3
needs both. Only **US4 and US5 can be worked independently** once Phase 3 lands. This is a
mostly-sequential feature, not a fan-out one.

---

## Parallel Example: Phase 2 Foundational

```bash
# Three independent domain files — launch together:
Task: "Add SourceKey enum to crates/midi-core/src/domain/ids.rs"
Task: "Add MAX_SYSEX_BYTES to crates/midi-core/src/domain/constants.rs"
Task: "Add port-open and system-unavailable variants to crates/midi-core/src/application/error.rs"

# Then, once T007 lands:
Task: "Change PersistedSettings.selected_sources to Vec<SourceKey> in crates/midi-core/src/application/settings.rs"
Task: "Create the decoder in crates/midi-core/src/domain/decoder.rs"
```

---

## Implementation Strategy

### MVP scope

**Phases 1, 2, 3, 4, and 8** — Setup, Foundational, US2, US1, and simulation removal.

Phase 8 is **inside** the MVP, not after it. US1 and US2 alone give an application that shows real
traffic while still carrying a simulator; the feature's premise is "nothing is mocked", so the MVP is
not reached until the generator is gone (FR-002, SC-014).

1. Phase 1 → Phase 2 → **checkpoint: app still runs on the simulator**
2. Phase 3 → **checkpoint: Sources list matches Audio MIDI Setup**
3. Phase 4 → **checkpoint: real messages, real bytes, malformed input visible**
4. Phase 8 → **checkpoint: five idle minutes, zero rows**
5. Stop and validate against Quickstart Scenarios 1, 2, 3, 7

### Incremental delivery after MVP

1. **US3 (hot-plug)** — the largest remaining value, and the one carrying the plan's top risk. Validate
   notifications at T035, not at the end
2. **US4 (virtual destination)** — self-contained
3. **US5 (spy group)** — three small tasks; can be pulled forward at any point after Phase 3
4. **Phase 9** — the Definition of Done gates

### Risk-first sequencing

Two tasks are worth attempting early even out of order, because both can invalidate the design rather
than merely delay it:

- **T035** (notification client ordering) — if notifications never fire under Tauri's run loop, US3
  needs the polling fallback. Contained to `notifications.rs`, but better known in week one
- **T026** (one `InputPort` per source) — validates the byte-fidelity choice from
  [D2](./research.md#d2-midi-10-packetlist-not-midi-20-eventlist) against real hardware

---

## Notes

- **No test tasks exist and none may be added** — the Verification Standard prohibits test files, test
  dependencies, doctests, and CI test steps. [quickstart.md](./quickstart.md) carries verification as 11
  manual scenarios
- `[P]` = different files, no dependency on incomplete work
- Commit after each task or logical group; every checkpoint should compile clean and run
- **T016 is throwaway scaffolding** deleted by T050 — it keeps the app runnable through Phases 2–4,
  which the Verification Standard depends on
- Phase 2 contains this feature's Constitution VII deviations. They are tracked in
  [plan.md](./plan.md#complexity-tracking), not to be re-litigated per task
- Every task touching the Sources panel is subject to Constitution VI: new text is confined to states
  the reference screenshots could not depict, and no existing control changes
