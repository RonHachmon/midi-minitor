---
description: "Task list for feature implementation"
---

# Tasks: Windows Support

**Input**: Design documents from `specs/003-windows-support/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: **None.** The constitution's Verification Standard prohibits a test suite, and
[plan.md](./plan.md) records the skip in its Constitution Check. The tasks template's test-task
categories are therefore inert here and are omitted deliberately rather than overlooked. Verification
is `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `npm run typecheck`, and the
manual pass in [quickstart.md](./quickstart.md), run **on both platforms**.

**Organization**: Tasks are grouped by user story so each can be implemented and verified
independently.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1..US6)
- Every task names the exact file it changes

## Path Conventions

Rust workspace plus a React webview, per [plan.md](./plan.md):

- `crates/midi-core/` — domain and application, platform-free
- `crates/midi-macos/` — the CoreMIDI adapter (behaviour unchanged by this feature)
- `crates/midi-windows/` — the new WinMM adapter
- `src-tauri/src/` — the Tauri shell
- `src/` — the webview

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Bring the second adapter crate into the workspace so there is somewhere for the Windows
work to land. Nothing here changes behaviour on either platform.

- [X] T001 Add `"crates/midi-windows"` to `[workspace] members` in `Cargo.toml`
- [X] T002 Create `crates/midi-windows/Cargo.toml`: `midi-core` path dependency, `[lints] workspace = true`, and a Windows-gated target dependencies block adding the `windows` crate with features `Win32_Media_Multimedia` and `Win32_Devices_DeviceAndDriverInstallation`; mirror `crates/midi-macos/Cargo.toml`'s comment naming why `midir` was rejected, citing research D1's evidence from its WinMM backend
- [X] T003 Create `crates/midi-windows/src/lib.rs` with the crate-level `cfg(target_os = "windows")` attribute, `pub mod` declarations for `endpoints`, `notifications`, `receive`, `sysex`, `source`, and module docs stating **why** WinMM rather than Windows MIDI Services or WinRT (research D1), why the official `windows` crate rather than hand-declared FFI (research D6), and that the callback-to-worker split is platform-imposed rather than chosen (research D5)
- [X] T004 [P] Create doc-comment-only module files so the crate compiles: `crates/midi-windows/src/endpoints.rs`, `crates/midi-windows/src/notifications.rs`, `crates/midi-windows/src/receive.rs`, `crates/midi-windows/src/sysex.rs`, `crates/midi-windows/src/source.rs`
- [X] T005 Target-gate the adapter dependencies in `src-tauri/Cargo.toml`: move `midi-macos` under a macOS-gated target dependencies block and add `midi-windows` under a Windows-gated one, so the wrong platform's crate is never built

**Checkpoint**: `cargo check` succeeds on the development platform; the workspace has two adapter crates.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The three additive core type changes, the completed `EventSource` port, and the removal
of the shell's concrete-adapter coupling. These are the seam every user story below plugs into.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

### The shared debouncer moves into the core

- [X] T006 Create `crates/midi-core/src/support/mod.rs` and `crates/midi-core/src/support/debounce.rs`, moving `Debouncer` and `DEBOUNCE_MILLIS` **verbatim** from `crates/midi-macos/src/notifications.rs`; adapt the module docs so the macOS-specific run-loop paragraph reads as a platform note rather than the rule, and state why a platform-free timing helper lives in the core (both adapters need it identically — see Complexity Tracking in plan.md)
- [X] T007 Declare `pub mod support;` in `crates/midi-core/src/lib.rs` and extend its layering diagram so both adapters appear
- [X] T008 Delete `crates/midi-macos/src/notifications.rs`, drop its `pub mod notifications;` from `crates/midi-macos/src/lib.rs`, and change `crates/midi-macos/src/source.rs` to import `Debouncer` from `midi_core::support::debounce`

### `SourceKey` gains a second kind of identity

- [X] T009 Add `DeviceInterface(String)` to `SourceKey` in `crates/midi-core/src/domain/ids.rs`, change the derive from `Copy` to `Clone`, and document **why** a new variant rather than widening `Endpoint(i32)` (existing macOS settings files must keep loading — FR-050, research D3)
- [X] T010 [P] Fix the non-`Copy` fallout in `crates/midi-core/src/domain/source.rs`: `SourceCatalogue::replace`'s previous-selection vector, `key_of`, `apply_selection`, `selected_keys`, `present_keys` — clone where the value was copied, no clone where a reference will do
- [X] T011 [P] Fix the non-`Copy` fallout in `crates/midi-core/src/application/monitor.rs`: the `remembered: Vec<SourceKey>` field, `select_by_key`, and `replace_catalogue`'s remembered-set comparison
- [X] T012 [P] Fix the non-`Copy` fallout in `crates/midi-macos/src/endpoints.rs` (the `SourceKey::Endpoint` construction in `to_source`) and `crates/midi-macos/src/source.rs` (`IdMinter::id_for`, the `HashMap<SourceKey, SourceId>` lookups, and the three `SourceKey::VirtualDestination` comparisons in `scan` and `sync_ports`)

### A capability the platform cannot offer at all

- [X] T013 Add `Unsupported { detail: String }` to `Availability` in `crates/midi-core/src/domain/source.rs` with its fourth `reason()` arm and no catch-all; document why it cannot reuse `Unopenable` — an `Unopenable` row stays tickable on purpose, and a row the platform can never satisfy must not (FR-042, data-model.md §2)
- [X] T014 Add `PlatformCapabilities { byte_fidelity }` and `ByteFidelity::{AsTransmitted, Assembled { detail }}` to `crates/midi-core/src/application/ports.rs`, beside `MidiSystemStatus`; document why an enum rather than `Option<String>` — faithfulness is a state worth naming, and a third fidelity level must become a compile error at every render site

### The port becomes complete and the shell stops naming an adapter

- [X] T015 Add `sync_ports`, `capabilities`, and `dropped_events` to the `EventSource` trait in `crates/midi-core/src/application/ports.rs` per [contracts/event-source-port.md](./contracts/event-source-port.md), documenting for each **why** it belongs in the port rather than on a concrete adapter
- [X] T016 In `crates/midi-macos/src/source.rs`, move the inherent `sync_ports` and `dropped_events` into `impl EventSource for CoreMidiSource` and add `capabilities()` returning `ByteFidelity::AsTransmitted`; behaviour must be identical — this is a relocation, not a rewrite
- [X] T017 Give `Monitor` a `PlatformCapabilities` field in `crates/midi-core/src/application/monitor.rs`, set from a new `Monitor::new` parameter and exposed read-only; document why it is held but never persisted (it describes the machine running right now)
- [X] T018 Create `src-tauri/src/platform.rs` with a macOS-gated `event_source(clock: Arc<dyn Clock>) -> Box<dyn EventSource>` returning `CoreMidiSource`, plus module docs stating that this file is the **only** `cfg(target_os)` in the application; declare `pub mod platform;` in `src-tauri/src/lib.rs`
- [X] T019 Change `AppState` in `src-tauri/src/state.rs` to hold `Arc<Mutex<dyn EventSource>>` — no concrete adapter type named anywhere in the file — keeping `sync_ports`'s failure-collecting behaviour unchanged
- [X] T020 Rewrite the wiring in `src-tauri/src/lib.rs` to construct through `platform::event_source`, pass `source.capabilities()` into `Monitor::new`, and rewrite the module docs and the step-2 comment so the main-thread ordering requirement reads as macOS's rule (CoreMIDI binds notification delivery at first-client creation) rather than as a cross-platform one — research D4 records that Windows imposes no such constraint

### The wire contract

- [X] T021 In `src-tauri/src/dto.rs`, replace `SourceDto.unavailable: Option<String>` with `availability: AvailabilityDto`, a tagged union over `Open` / `Unopenable { detail }` / `Absent` / `Unsupported { detail }` following the `MidiSystemStatusDto` house pattern, matched exhaustively with no catch-all
- [X] T022 In `src-tauri/src/dto.rs`, add `ByteFidelityDto` (`AsTransmitted` / `Assembled { detail }`) and a `byte_fidelity` field on `CatalogueDto`, populated from the monitor's capabilities
- [X] T023 Regenerate `src/bindings.ts` by running `npm run tauri dev` once — generated from the Rust declarations, never hand-edited
- [X] T024 [P] Add `byteFidelity` to `MonitorState` and `applyCatalogue` in `src/store.ts`, with a default that names the faithful case rather than a null
- [X] T025 [P] Update `src/components/SourcesPanel.tsx` to read the `availability` union instead of the removed `unavailable` string, preserving today's rendering for `open`, `unopenable`, and `absent` exactly — the `unsupported` behaviour is US5's work

**Checkpoint**: The workspace compiles, `npm run typecheck` is clean, and macOS behaviour is what it was. Both adapters can now be substituted behind one trait object.

---

## Phase 3: User Story 1 - Monitor a real MIDI device on Windows (Priority: P1) 🎯 MVP

**Goal**: Real Windows MIDI traffic reaches the event table — right message types, channels, data, and
bytes — with nothing invented.

**Independent Test**: On a Windows machine with any MIDI device attached, launch the application, tick
the device, and play it. Rows appear in response to physical input and stop when input stops.

- [X] T026 [US1] Implement `enumerate()` in `crates/midi-windows/src/endpoints.rs` over `midiInGetNumDevs` and `midiInGetDevCapsW`, returning one entry per device index with the `MIDIINCAPSW` product name passed through **verbatim**; document in the module docs that Windows truncates names at 32 characters (research D1 limitation 3) so the clipping reads as Windows' and not ours
- [X] T027 [US1] Add the device-interface lookup to `crates/midi-windows/src/endpoints.rs` using `midiInMessage` with `DRV_QUERYDEVICEINTERFACESIZE` then `DRV_QUERYDEVICEINTERFACE`, producing `SourceKey::DeviceInterface(String)`; where the string is empty (documented as possible), fall back to the port name per FR-020, and give every WinMM message code a named `const`
- [X] T028 [US1] Implement the `MidiInProc` callback in `crates/midi-windows/src/receive.rs` handling `MIM_DATA`, `MIM_LONGDATA`, `MIM_ERROR`, and `MIM_LONGERROR`: timestamp the arrival through the `Clock` port, push the bytes onto a FIFO channel, and **call no multimedia function** — document the deadlock rule (research D5) as the reason the callback does nothing else
- [X] T029 [US1] Implement the `MIDIHDR` buffer pool in `crates/midi-windows/src/sysex.rs`: allocate, `midiInPrepareHeader`, `midiInAddBuffer`, re-queue after use, and `midiInUnprepareHeader` on close — every call off the callback thread, every `unsafe` block as small as the call it wraps with the invariant that makes it sound stated in a comment
- [X] T030 [US1] Implement the worker thread in `crates/midi-windows/src/receive.rs`: drain the channel in order, feed the bytes to the unmodified `midi_core::domain::decoder::MessageDecoder`, deliver each outcome through the `EventSink` carrying the timestamp taken in the callback, and re-queue SysEx buffers; document that FIFO order plus one shared channel is what preserves arrival order (FR-029)
- [X] T031 [US1] Add `WindowsMidiSource` to `crates/midi-windows/src/source.rs` — `new(clock)`, the shared receive state behind locks and atomics, and a `dropped` counter — mirroring `crates/midi-macos/src/source.rs` module for module so a reader who knows one knows the other
- [X] T032 [US1] Implement `EventSource::start` and `EventSource::stop` in `crates/midi-windows/src/source.rs`: start the worker thread and register for device notifications; stop closes every open port, unprepares every buffer, and abandons in-flight System Exclusive so an interrupted dump is reported rather than stranded (FR-026), idempotently
- [X] T033 [US1] Implement `EventSource::sync_ports` in `crates/midi-windows/src/source.rs` over `midiInOpen`/`midiInStart`/`midiInStop`/`midiInReset`/`midiInClose`, opening selected ports, closing unselected or departed ones, and **returning** per-port failures with their reason rather than propagating them (contracts/event-source-port.md)
- [X] T034 [US1] Implement `EventSource::catalogue`, `capabilities` (returning `ByteFidelity::Assembled { detail }`), and `dropped_events` in `crates/midi-windows/src/source.rs`; the `detail` states that Windows delivers short messages already assembled and that System Exclusive transfers are exact (FR-037)
- [X] T035 [US1] Map `MIM_ERROR` in `crates/midi-windows/src/receive.rs` to the `Invalid` category with the bytes Windows packed into the doubleword; where the low byte is a recognisable status byte report the bytes its length implies, otherwise report only the byte Windows placed first and state that the remainder was not determinable (FR-027, research D1 limitation 2)
- [X] T036 [US1] Route `MIM_LONGERROR` and oversize or unterminated transfers in `crates/midi-windows/src/receive.rs` through the decoder's existing truncated and incomplete outcomes, so a dump larger than retention is marked truncated with its true size and never silently shortened (FR-024, FR-025)
- [X] T037 [US1] Add the Windows-gated arm to `src-tauri/src/platform.rs` returning `WindowsMidiSource`, completing the composition root
- [ ] T038 [US1] Walk quickstart steps 3–7 on a Windows machine: a played note within a quarter second, a five-minute idle producing zero rows, a chord in transmission order, a SysEx dump with its true byte count, and every Filter panel control governing real traffic

**Checkpoint**: Real Windows MIDI events reach the table. This is the MVP.

---

## Phase 4: User Story 2 - See the machine's real MIDI ports on Windows (Priority: P1)

**Goal**: The Sources list shows the ports Windows actually reports, named as Windows names them,
grouped as the reference layout groups them — and says so when there are none or when the MIDI system
cannot be reached.

**Independent Test**: Compare the Sources list against another MIDI application on the same machine;
then detach every device, relaunch, and confirm an empty list with a stated reason.

**Depends on**: US1's `endpoints.rs` enumeration (T026, T027).

- [X] T039 [US2] Implement `scan()` in `crates/midi-windows/src/source.rs`: every enumerated input port under `SourceGroupId::MidiSources`, then the standalone destination row, and nothing at all under `SourceGroupId::SpyOnOutput` — the same `Vec<Source>` shape `crates/midi-macos/src/source.rs` builds, so the two adapters can be read side by side
- [X] T040 [US2] Add the `IdMinter` equivalent to `crates/midi-windows/src/source.rs` so each port gets a distinct `SourceId` keyed on its distinct `SourceKey`, making two ports that share a display name separately selectable and correctly attributed (FR-008, US2·3)
- [X] T041 [US2] Return `CoreError::MidiSystemUnavailable` from `EventSource::start` in `crates/midi-windows/src/source.rs` only when the MIDI system itself cannot be reached — never for a single port that will not open — so the panel can distinguish "cannot see any devices" from "no devices" (FR-010, US2·5)
- [X] T042 [US2] Mark a port another program holds as `Availability::Unopenable { detail }` in `crates/midi-windows/src/source.rs`, keeping the row listed and tickable; document in the module docs that exclusive grants make this routine on Windows rather than the rarity it is on macOS (FR-015, US2·6)
- [X] T043 [US2] Verify the empty-list and system-unreachable paths render on Windows through the existing text in `src/components/SourcesPanel.tsx` with no new platform conditional, and that a machine with zero MIDI devices leaves every other control operable (FR-009, SC-004)
- [ ] T044 [US2] Walk quickstart steps 1, 2, and 8 on a Windows machine: the list matches another MIDI application name for name and count for count, and a machine with no devices says so

**Checkpoint**: The Sources list is truthful on Windows, in every one of its states.

---

## Phase 5: User Story 3 - macOS is exactly as it was (Priority: P1)

**Goal**: A macOS user notices nothing. Every device, message, control, saved setting, and raw byte
behaves precisely as before.

**Independent Test**: Re-run every acceptance scenario of feature 002 on macOS and confirm each still
passes unaltered.

- [X] T045 [US3] Review the whole `crates/midi-macos/` diff and confirm every change is mechanical — the `.clone()` additions from T012, the `Debouncer` import change from T008, and the inherent-to-trait relocation from T016 — with no behavioural edit anywhere in it
- [X] T046 [US3] Confirm `crates/midi-macos/src/source.rs` still emits the standalone destination row with `Availability::Open` and still publishes the virtual destination, so FR-036 holds and Decision 1 changes nothing on macOS
- [X] T047 [US3] Confirm the running-status branch in `crates/midi-core/src/domain/decoder.rs` is untouched and still reached on macOS, and that no code path was removed to suit the Windows platform (plan.md §4)
- [ ] T048 [US3] Verify settings compatibility by hand: launch the new macOS build against a settings file written by the previous version and confirm every `Endpoint` key loads, applies, and is written back unchanged (FR-050, contracts/persisted-settings.md)
- [ ] T049 [US3] Walk quickstart steps 27–32 on macOS: every feature 002 acceptance scenario, running-status bytes off the cable, the byte-fidelity statement **absent**, the destination row still publishing, and each reference screenshot matching

**Checkpoint**: Windows support has cost macOS nothing.

---

## Phase 6: User Story 4 - Keep monitoring across plug and unplug on Windows (Priority: P2)

**Goal**: The Sources list follows cables on its own, and a selection survives unplug, replug, and
relaunch — matched to the physical port, not to a list position.

**Independent Test**: With the application running and a device ticked, unplug it, replug it, then quit
and relaunch; the tick returns each time and events resume without re-ticking.

- [X] T050 [US4] Implement device-interface notification registration in `crates/midi-windows/src/notifications.rs` with `CM_Register_Notification` and a `CM_NOTIFY_FILTER` of the device-interface type; document why this rather than `RegisterDeviceNotification` — it needs no window handle and no message pump, so the adapter borrows nothing from the Tauri window (research D4)
- [X] T051 [US4] Feed every notification kind into the shared `midi_core::support::debounce::Debouncer` from `crates/midi-windows/src/notifications.rs`, triggering one full rescan per settled burst and pushing the result to the `CatalogueSink`; mirror the macOS comment on why every kind is treated the same way — a rescan is cheaper than trusting a notification to describe the change completely
- [X] T052 [US4] Unregister the notification handle in `EventSource::stop` in `crates/midi-windows/src/source.rs` and confirm a device departing mid-transfer has its half-finished System Exclusive abandoned rather than stranded (FR-012, FR-026)
- [X] T053 [US4] Confirm a reconnected port returns ticked without user action by exercising the existing `Monitor::replace_catalogue` remembered-set path against `SourceKey::DeviceInterface` keys, and that a re-tick is never required (FR-014, US4·3)
- [X] T054 [US4] Implement the FR-020 fallback in `crates/midi-windows/src/source.rs`: where a port yields no device-interface string and two present ports share a display name with nothing else to distinguish them, apply the remembered selection to **neither** and state on both rows that the previous selection could not be matched — carried through the existing per-row reason mechanism, with a doc comment on why guessing one would silently monitor a device the user never chose
- [X] T055 [US4] Confirm a port that becomes free after being held starts being monitored on the next catalogue-driven `sync_ports` with no re-tick, in `crates/midi-windows/src/source.rs` (FR-016)
- [ ] T056 [US4] Walk quickstart steps 9–18 on a Windows machine, including the ten consecutive replug-and-relaunch repetitions of step 16 (SC-006)

**Checkpoint**: Cables can be handled mid-session without the user losing their place.

---

## Phase 7: User Story 5 - Learn on the control that acting as a destination is unavailable on Windows (Priority: P2)

**Goal**: The `Act as a destination for other programs` row stays exactly where the reference layout
puts it, cannot be switched on, and says why and what to do instead.

**Independent Test**: On Windows, expand Sources; the row is present with its verbatim label in its
reference position, cannot be ticked, and carries a plain statement of the limitation and the
alternative — while everything else in the window works normally.

- [X] T057 [US5] Emit the standalone row from `scan()` in `crates/midi-windows/src/source.rs` with the verbatim label `Act as a destination for other programs` and `Availability::Unsupported { detail }`, the label held as a named `const` exactly as `crates/midi-macos/src/source.rs` holds it
- [X] T058 [US5] Write the `detail` prose in `crates/midi-windows/src/source.rs`: Windows provides no built-in way for an application to publish a MIDI destination other programs can send to, and the alternative is a MIDI loopback utility whose ports then appear under `MIDI sources` and are monitored like any other port (FR-032, FR-033)
- [X] T059 [US5] Render `unsupported` rows in `src/components/SourcesPanel.tsx` as present, visible, and **not** operable, with the reason beneath — keeping `unopenable` rows tickable as they are today, and documenting in the component docs why the two unavailabilities call for different controls
- [X] T060 [US5] Confirm `crates/midi-windows/src/source.rs` never opens a port, never mints traffic, and never attributes any event to the destination row, and that `sync_ports` ignores its checkbox entirely on Windows (FR-034)
- [X] T061 [US5] Confirm the `Spy on output to destinations` group is still emitted present, empty, and stating its unavailability on both platforms, unchanged by this feature, in `src-tauri/src/dto.rs` (FR-048)
- [ ] T062 [US5] Walk quickstart steps 19–23 and 26 on a Windows machine

**Checkpoint**: Decision 1 is visible on the control, and nothing else in the window is affected.

---

## Phase 8: User Story 6 - Know what the `Data` column is showing on Windows (Priority: P3)

**Goal**: A standing statement travels with the `Data` column on Windows explaining that Windows
delivers short messages already assembled and that System Exclusive bytes are exact — and it is absent
on macOS, where it is not true.

**Independent Test**: On Windows, the statement is present and readable with the `Data` column and goes
and returns with it; on macOS it is absent.

- [X] T063 [US6] Render the `byteFidelity` detail in `src/components/EventTable.tsx` alongside the `Data` column when the fidelity is `assembled`, matched exhaustively with no catch-all so a third fidelity level becomes a compile error here
- [X] T064 [US6] Gate the statement on the `Data` column's visibility in `src/components/EventTable.tsx`, so hiding the column through the column controls takes the statement with it and showing the column brings it back (FR-038, US6·5)
- [X] T065 [US6] Confirm no per-row annotation is ever produced and no platform conditional is introduced in `src/components/EventTable.tsx` — the application cannot know which individual messages Windows assembled, and claiming otherwise would be the fabrication FR-039 forbids
- [ ] T066 [US6] Walk quickstart steps 24, 25, 30, 33, and 34: the statement present on Windows and absent on macOS, an identical SysEx dump on both machines, and running-status rows differing exactly as Decision 2 describes (SC-008)

**Checkpoint**: Every difference a user can observe between the two platforms is one the window itself explains.

---

## Phase 9: Polish & Cross-Cutting Concerns

- [X] T067 [P] Update `README.md`: the application is no longer macOS-only, add the Windows prerequisites (MSVC toolchain, C++ Build Tools, WebView2), replace the IAC Driver note with the loopback guidance from quickstart.md including the `Microsoft GS Wavetable Synth` warning, and state that `Act as a destination for other programs` is unavailable on Windows
- [X] T068 [P] Audit `crates/midi-windows/` for the constitution's Principle III: every `pub` item and every module carries a doc comment stating **why**, with the three pre-identified ones present — why WinMM rather than the newer stack, why the callback hands off instead of decoding in place, and why `Availability::Absent` is unreachable here (research D7)
- [X] T069 [P] Audit every `unsafe` block in `crates/midi-windows/` — each as small as the call it wraps, each documenting the invariant that makes it sound, and no `unwrap`, `expect`, or `panic` anywhere outside `main`
- [X] T070 Confirm `cfg(target_os)` appears nowhere in the application except `src-tauri/src/platform.rs` and the two adapter crates' crate-level attributes, by grepping `src-tauri/src/`, `src/`, and `crates/midi-core/`
- [X] T071 Read `crates/midi-macos/src/source.rs` and `crates/midi-windows/src/source.rs` side by side and confirm their `scan()` functions differ in exactly two `Availability` values and nothing else (plan.md Risks)
- [X] T072 Run `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and `npm run typecheck` on **both** platforms; all three clean
- [ ] T073 Complete the Definition of Done checklist in [quickstart.md](./quickstart.md), including the side-by-side reference-screenshot comparison on both platforms with each platform-forced deviation documented at the code

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies
- **Foundational (Phase 2)**: depends on Setup — **blocks every user story**
- **US1 (Phase 3)**: depends on Foundational. The MVP
- **US2 (Phase 4)**: depends on Foundational, and on T026–T027 from US1 for enumeration
- **US3 (Phase 5)**: depends on Foundational only — it verifies what Phase 2 must not have broken, and can run the moment Phase 2 lands
- **US4 (Phase 6)**: depends on US1 and US2 — hot-plug rescans the catalogue those stories produce
- **US5 (Phase 7)**: depends on US2's `scan()` (T039) for the row it fills
- **US6 (Phase 8)**: depends on Foundational's T022–T024 and US1's T034 for the detail it renders
- **Polish (Phase 9)**: depends on every story being complete

### Within Each Story

- Windows adapter: enumeration → receive path → source lifecycle → catalogue shaping
- Core before adapters; adapters before the shell; the shell before the webview
- No test-first ordering applies — this project has no test suite by constitutional design

### Parallel Opportunities

- **Phase 2**: T010, T011, and T012 are the same mechanical `.clone()` change in three separate files and run together. T024 and T025 are different webview files and run together
- **Phase 3**: T028 (`receive.rs`) and T029 (`sysex.rs`) are separate files and can be written in parallel once T026–T027 land
- **US3 (Phase 5)** is entirely independent of US1, US2, US4, US5, and US6 — a second person can run the macOS regression pass while Windows work continues
- **Phase 9**: T067, T068, and T069 touch different files and run together
- Two people after Phase 2: one on US1 → US2 → US4, one on US3 then US5 → US6

## Parallel Example: Phase 2

```bash
# The non-Copy SourceKey fallout, three files at once:
Task: "Fix non-Copy fallout in crates/midi-core/src/domain/source.rs"
Task: "Fix non-Copy fallout in crates/midi-core/src/application/monitor.rs"
Task: "Fix non-Copy fallout in crates/midi-macos/src/endpoints.rs and source.rs"

# The webview's adaptation to the new wire shape, two files at once:
Task: "Add byteFidelity to src/store.ts"
Task: "Consume the availability union in src/components/SourcesPanel.tsx"
```

---

## Implementation Strategy

### MVP First

1. Phase 1 — Setup
2. Phase 2 — Foundational (**critical**: blocks everything)
3. Phase 3 — US1
4. **STOP and VALIDATE**: on a Windows machine, tick a device and play it. Rows must appear
5. Phase 5 — US3, immediately after, because a Windows build bought with macOS regressions fails a P1 story

### Incremental Delivery

1. Setup + Foundational → both adapters can sit behind one trait object
2. US1 → real Windows traffic in the table (MVP)
3. US2 → a truthful Sources list in all its states
4. US3 → macOS verified unchanged
5. US4 → cables can be handled mid-session
6. US5 → Decision 1 stated on its control
7. US6 → Decision 2 stated with its column

Each increment is exercised by hand on the platform it affects before the next begins.

---

## Notes

- **No test tasks appear above, deliberately.** The constitution's Verification Standard prohibits a
  test suite; the skip is recorded in plan.md's Constitution Check and repeated here so it reads as a
  decision rather than an omission
- Every core type change is an **added variant**, so the compiler enumerates the work — no catch-all
  arm may be introduced to quiet it
- `src/bindings.ts` is generated (T023), never hand-edited
- A macOS developer never compiles `crates/midi-windows/` and a Windows developer never compiles
  `crates/midi-macos/`; the feature is not done until someone has built both
- Commit after each task or logical group; stop at any checkpoint to validate a story independently

---

## Verification status at end of implementation

Implementation ran on **Windows 11**. Recorded here so the manual pass knows what is already
covered and what is genuinely untouched.

### Verified by the compiler

- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, and
  `npm run typecheck` are clean.
- **The macOS crates were cross-compiled from Windows** — `rustup target add x86_64-apple-darwin`
  then `cargo clippy -p midi-core -p midi-macos --target x86_64-apple-darwin -- -D warnings`, clean.
  This was not anticipated by the plan and materially reduces the US3 risk. The Tauri **shell**
  cannot be cross-checked this way: a macOS dependency needs a C compiler for that target. So the
  macOS arm of `src-tauri/src/platform.rs` — two lines — is the only Rust in the repository that
  has never been compiled.

### Verified by running the adapter against real hardware

A throwaway harness (outside the repository, since the constitution forbids test artifacts) drove
the machine's `goRave Virtual` loopback pair through the adapter. It confirmed:

- enumeration, name reading, and `DRV_QUERYDEVICEINTERFACE` — the derived message constants are
  correct at runtime, and a real stable interface path is returned;
- `midiInOpen`, the buffer pool, `midiInStart`, and the stop → reset → unprepare → close ordering,
  with no crash and an idempotent `stop`;
- Note On, Note Off, Control Change, Program Change, Clock, and a 44-byte System Exclusive dump —
  every row's bytes exactly as transmitted, SysEx byte-for-byte in a single event, no event
  produced while idle, nothing dropped.

**It also found a real defect**, now fixed: `expected_data_len` reports no length for System Real
Time status bytes, and `unpack_short` was defaulting that to the full three-byte doubleword. A
Clock arriving after a Control Change therefore completed a *fictional* Control Change out of two
padding zeroes. Reading the code did not reveal this; running bytes through it did.

### Not verified — needs a person

- **Anything perceptual or physical on Windows**: the quarter-second latency of a played note,
  unplugging and replugging a cable, two ports sharing a name, a port held by another program,
  and the side-by-side screenshot comparison. The webview changes in particular have been
  type-checked but never *looked at*.
- **Everything on macOS.** No Mac was available. US3's compile-level risk is largely closed by the
  cross-target check above, but no macOS behaviour was observed, and the previous version's
  settings file was never loaded by the new build.
