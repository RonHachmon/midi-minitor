---

description: "Task list for 005-midi-send-console"
---

# Tasks: A Send Screen With Built-In Requests and a Chosen Identity

**Input**: Design documents from `specs/005-midi-send-console/`

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
- **[Story]**: US1–US5 — maps to the user stories in [spec.md](./spec.md)
- Every task names the exact file it touches

## Path Conventions

Tauri desktop application, per [plan.md](./plan.md):

- **Core** (domain + application): `crates/midi-core/src/`
- **Adapters**: `crates/midi-macos/src/`, `crates/midi-windows/src/`
- **Shell** (IPC surface): `src-tauri/src/`
- **Webview**: `src/`

## The one thing that makes this feature different

This is the first feature to touch **both** platform adapters. A macOS developer never compiles
`midi-windows` and a Windows developer never compiles `midi-macos`, so a task that looks finished on
one machine can leave the other broken. Every adapter task below is therefore paired with the
cross-compiled check, and T003 exists to make that check available before it is needed.

---

## Phase 1: Setup

**Purpose**: Establish a clean baseline and the ability to check the platform you are not on.

- [X] T001 Run the four gates on an unchanged tree from the repository root — `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, `npm run typecheck`, `npm run tauri dev` — and record that all four pass, so any failure later in this feature is attributable to this feature
- [X] T002 Copy the settings document written by the current build to a safe location outside the repository, to verify in T085 that a pre-005 document still loads with every setting intact — macOS `~/Library/Application Support/com.midimonitor.mock/settings.json`, Windows `%APPDATA%\com.midimonitor.mock\settings.json`
- [X] T003 Install the cross-compilation target for the platform you are **not** developing on and confirm the check runs clean on the unchanged tree — from Windows `rustup target add x86_64-apple-darwin` then `cargo clippy -p midi-core -p midi-macos --target x86_64-apple-darwin -- -D warnings`; from macOS `rustup target add x86_64-pc-windows-msvc` then `cargo clippy -p midi-core -p midi-windows --target x86_64-pc-windows-msvc -- -D warnings`

**Checkpoint**: Baseline green on both adapters, and a pre-005 settings file is preserved.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The second direction — the port, the encoder, the service, and both adapters'
implementations. Nothing in this phase is visible to a user; everything after it depends on all of it.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete. Unlike feature 004, this
phase is genuinely required: every story sends something, and nothing can send until the port exists
and both adapters implement it.

### Domain types

- [X] T004 [P] Add the five newtypes to `crates/midi-core/src/domain/ids.rs` — `TargetId`, `TargetKey` (`Endpoint(i32)` / `DeviceName(String)` / `PublishedSource`), `SendRecordId`, `RequestName`, `PublishedName` — with `parse` returning `Result` for the two string-valued ones, following the shape `HexPrefix::parse` already uses. Document why `TargetKey` splits three ways, mirroring `SourceKey`
- [X] T005 [P] Add the new constants to `crates/midi-core/src/constants.rs` — `SEND_RECORD_LIMIT`, `MAX_REQUEST_NAME_LEN`, `MAX_PUBLISHED_NAME_LEN`, `DEFAULT_PUBLISHED_NAME`, `IDENTITY_REQUEST` — each with a doc comment giving the reason for the value, not restating it
- [X] T006 [P] Add the sixteen `CoreError` variants listed in [data-model.md](./data-model.md) to `crates/midi-core/src/application/error.rs`, each documenting the condition that produces it and what a caller can do about it, per Principle III
- [X] T007 Create `crates/midi-core/src/domain/encoder.rs` with `encode(&MidiMessage) -> Result<Vec<u8>, CoreError>`, matching exhaustively with **no catch-all arm**, returning `UnsendableMessage` for `Invalid` alone. Module doc must state that this is the byte-encoding boundary `constants.rs` refers to — the one place a 1-based display channel becomes a 0-based wire nibble — and why the unreachable `Err` still has to exist (depends on T004, T006)
- [X] T008 [P] Create `crates/midi-core/src/domain/sendable.rs` with `SendableKind` (18 payload-free variants), `FieldSpec`, `FieldId`, and `ALL` / `label` / `id` / `fields` / `default_message`. Ranges come from `CHANNEL_RANGE`, `MAX_DATA_BYTE`, and `MAX_DATA_14` — never repeated literals. Module doc must state why this is a third message-shaped enum and not duplication, per [research D6](./research.md) (depends on T005)
- [X] T009 Create `crates/midi-core/src/domain/composition.rs` with `Composition { Guided, Raw }`, `from_kind`, `parse_raw`, `bytes`, `fields`, `set_field`. `parse_raw` accepts an entry only when `MessageDecoder::feed` yields exactly one `Decoded::Message` with nothing pending. Module doc must state why the two variants differ in **which value is authoritative**, citing the same lossiness `MidiEvent` documents (depends on T007, T008)
- [X] T010 [P] Create `crates/midi-core/src/domain/target.rs` with `Target` and `TargetKind`, documenting why `Target` carries no `Availability` when `Source` does (depends on T004)
- [X] T011 [P] Create `crates/midi-core/src/domain/publication.rs` with `Publication { name, published }` (depends on T004)
- [X] T012 [P] Create `crates/midi-core/src/domain/send_record.rs` with `SendRecord` and `SendOutcome`, documenting why `target` is a copied name rather than a reference and why `SendOutcome` is an enum rather than `Option<String>` (depends on T004)
- [X] T013 Register the six new modules in `crates/midi-core/src/domain/mod.rs` (depends on T007–T012)

### The port

- [X] T014 Add `Transmitter`, the `MidiAccess` bundle with its blanket impl, and `PublicationSupport` to `crates/midi-core/src/application/ports.rs`, and add the `publication` field to `PlatformCapabilities`. Every method documents its obligations per [contracts/transmitter-port.md](./contracts/transmitter-port.md); the `MidiAccess` doc must name the shared-CoreMIDI-client constraint that forces it (depends on T010, T006)

### The service

- [X] T015 Create `crates/midi-core/src/application/sender.rs` with the `Sender` service and `Outgoing`, per [data-model.md](./data-model.md). Module doc must state why it is one service (its state is entangled, exactly as `Monitor`'s is) and why it does **not** hold the `Transmitter` — the `outgoing()`/`record()` two-step mirrors `AppState::sync_ports` (depends on T009, T010, T011, T012, T014)
- [X] T016 Add `SendSettings` and the `#[serde(default)] send` field to `crates/midi-core/src/application/settings.rs`, per [contracts/persisted-settings.md](./contracts/persisted-settings.md). Document why **no migration is written** here when 004 needed one (depends on T011)
- [X] T017 Register `sender` in `crates/midi-core/src/application/mod.rs` (depends on T015)

### The macOS adapter

- [X] T018 [P] Add destination enumeration to `crates/midi-macos/src/endpoints.rs` using the `Destinations` iterator, keying each by `TargetKey::Endpoint(unique id)` and using the OS-supplied name verbatim (depends on T010)
- [X] T019 Implement `Transmitter` for `CoreMidiSource` in `crates/midi-macos/src/source.rs` — an `OutputPort` for destinations, `Client::virtual_source` plus `VirtualSource::received` for the published source, `PacketBuffer::new(0, bytes)` for both — and return `PublicationSupport::Supported` from `capabilities`. Document that publishing reuses the one client, and that the published source will appear in this application's own Sources list as an expected consequence (depends on T014, T018)
- [X] T020 Verify the macOS adapter compiles clean from a Windows machine — `cargo clippy -p midi-core -p midi-macos --target x86_64-apple-darwin -- -D warnings` — or natively if developing on macOS (depends on T019)

### The Windows adapter

- [X] T021 [P] Add output enumeration to `crates/midi-windows/src/endpoints.rs` using `midiOutGetNumDevs` and `midiOutGetDevCapsW`, keying each by `TargetKey::DeviceName`. **Exclude the MIDI Mapper pseudo-device** and document why (depends on T010)
- [X] T022 [P] Create `crates/midi-windows/src/send.rs` with the output path — `midiOutShortMsg` for messages of three bytes or fewer, and `midiOutPrepareHeader` / `midiOutLongMsg` / `midiOutUnprepareHeader` over a `MIDIHDR` for System Exclusive. Document the `MIDIHDR` lifetime and why the handle is held for the chosen target rather than opened per send
- [X] T023 Implement `Transmitter` for `WindowsMidiSource` in `crates/midi-windows/src/source.rs` — delegating to `send.rs`, returning `PublicationUnsupported` from `publish`, and returning `PublicationSupport::Unsupported` from `capabilities` with detail naming the loopback-utility route in the adapter's own words, reading as a sibling of the existing `Act as a destination for other programs` message (depends on T014, T021, T022)
- [X] T024 Register `send` in `crates/midi-windows/src/lib.rs` and verify the Windows adapter compiles clean from a macOS machine — `cargo clippy -p midi-core -p midi-windows --target x86_64-pc-windows-msvc -- -D warnings` — or natively if developing on Windows (depends on T023)

### The shell

- [X] T025 Rename `event_source` to `midi_access` in `src-tauri/src/platform.rs`, returning `Box<dyn MidiAccess>`, and update the module doc — this file remains the only one in the application that names an operating system (depends on T014, T019, T023)
- [X] T026 [P] Generalise `CataloguePump` to `PushPump<T>` in `src-tauri/src/stream.rs` and add the `CataloguePump` and `TargetPump` aliases. Keep the existing doc explaining why this is separate from `EventPump`, and add why one generic serves both
- [X] T027 Add `SendViewDto` and its members — `TargetDto`, `PublicationDto`, `PublicationSupportDto`, `CompositionDto`, `SendableKindDto`, `FieldDto`, `RequestDto`, `SendRecordDto` — plus the standalone `TargetsDto` to `src-tauri/src/dto.rs`. Spaced uppercase hex formatting (`90 3C 64`) lives here, not in the domain (depends on T015)
- [X] T028 Add the sixteen `IpcError` variants and their `From<CoreError>` arms to `src-tauri/src/error.rs` (depends on T006)
- [X] T029 Add `Sender` to managed state in `src-tauri/src/state.rs`, with `sender()` accessor, `transmit(&mut Sender)` performing the `outgoing()` → port → `record()` sequence, `refresh_targets()`, and `publish_targets()` (depends on T015, T025, T026, T027)
- [X] T030 Wire the send side into `src-tauri/src/lib.rs` — construct `Sender` from the adapter's targets, saved settings, and capabilities; call `state.refresh_targets()` inside the existing catalogue callback beside `publish_catalogue`; restore the saved publication at startup, reporting failure without disturbing anything else (depends on T029)

### The webview shell

- [X] T031 Extract the monitor body from `src/App.tsx` into `src/screens/MonitorScreen.tsx` **unchanged** — same components, same order, same props — and add the screen switcher to `src/App.tsx`, keeping `startStream` and `subscribeCatalogue` at `App` level so switching screens cannot unsubscribe them
- [X] T032 [P] Create `src/sendStore.ts` — a zustand store holding the mirrored `SendViewDto` plus the presentation-only in-flight field text, with a note that it never computes bytes
- [X] T033 Add the `get_send_view` and `subscribe_send_targets` wrappers and the shared `describeError` arms for `UnknownField`, `UnknownSendableKind`, and `UnknownSendRecord` to `src/ipc.ts` (depends on T032)

**Checkpoint**: The application builds and runs on both platforms, sends nothing yet, and every gate
passes. `src/bindings.ts` regenerates with the new types. User story work can now begin, and US1–US3
can be worked in parallel by different people.

---

## Phase 3: User Story 1 — Send a ready-made request (Priority: P1) 🎯 MVP

**Goal**: Open the send screen, pick a destination, pick a named request, press send, and have it
arrive — without knowing anything about MIDI.

**Independent Test**: With a destination selected, choose any built-in request, send it, and confirm
the receiving end shows exactly the message the request's name promised — [quickstart.md](./quickstart.md) §1.

### Core

- [X] T034 [US1] Create `crates/midi-core/src/domain/request.rs` with `Request`, `RequestOrigin`, and `RequestLibrary` (read paths only: `all`, `get`), plus the fifteen built-ins exactly as tabulated in [research D15](./research.md). Names follow the reference vocabulary where one exists — `Aftertouch (Poly)`, `Active Sense`, `Reset`. Document why a request's name is its identity (depends on T009, T013)
- [X] T035 [US1] Register `request` in `crates/midi-core/src/domain/mod.rs` and hold the library on `Sender` in `crates/midi-core/src/application/sender.rs`, adding `select_request`, `set_target`, and `chosen_target` (depends on T034)

### Shell

- [X] T036 [US1] Add the `get_send_view` and `subscribe_send_targets` handlers to `src-tauri/src/commands.rs` (depends on T029, T035)
- [X] T037 [US1] Add the `set_send_target` handler to `src-tauri/src/commands.rs`, persisting the choice by `TargetKey` (depends on T036)
- [X] T038 [US1] Add the `select_request` and `set_composition_field` handlers to `src-tauri/src/commands.rs`. `set_composition_field` belongs to this story because a built-in's values are adjustable (FR-009), and it is reused unchanged by US2 (depends on T036)
- [X] T039 [US1] Add the `send` handler to `src-tauri/src/commands.rs`, delegating to `AppState::transmit`, returning the updated view on both success and failure, and returning `NoSendTarget` when nothing is chosen (depends on T029, T036)
- [X] T040 [US1] Register the five new commands in `specta_builder` in `src-tauri/src/lib.rs` and run `npm run tauri dev` once so `src/bindings.ts` regenerates (depends on T036–T039)

### Webview

- [X] T041 [US1] Create `src/screens/SendScreen.tsx` — the layout, its mount-time `get_send_view` call, and its target subscription (depends on T031, T032)
- [X] T042 [P] [US1] Create `src/components/send/TargetPicker.tsx`, including the plain statement shown when no target is available at all, rather than an empty control (depends on T041)
- [X] T043 [P] [US1] Create `src/components/send/BytePreview.tsx`, rendering the preview string the DTO supplies and computing nothing (depends on T041)
- [X] T044 [P] [US1] Create `src/components/send/RequestLibrary.tsx`, listing each request with its name and plain-language description (depends on T041)
- [X] T045 [US1] Create `src/components/send/Composer.tsx` rendering the value fields the DTO supplies for the loaded request, each labelled by meaning. US2 extends this file with the kind picker (depends on T041)
- [X] T046 [US1] Add the send control and its outcome reporting to `src/screens/SendScreen.tsx` — every send produces a visible confirmation or a failure naming the target and the reason, and the message must say the bytes were *transmitted*, never that the far end received or acted on them (depends on T041)
- [X] T047 [US1] Add the `set_send_target`, `select_request`, `set_composition_field`, and `send` wrappers plus their `describeError` arms — `NoSendTarget`, `UnknownTarget`, `TransmitFailed`, `ValueOutOfRange` — to `src/ipc.ts` (depends on T040)
- [ ] T048 [US1] Verify [quickstart.md](./quickstart.md) §1 end to end on the platform you are developing on, including the two-interaction path, value adjustment, repeat sending, and the no-target refusal

**Checkpoint**: US1 is complete and shippable. The whole path — port, adapter, encoder, IPC, screen —
is exercised by a real device. This is the MVP.

---

## Phase 4: User Story 2 — Compose from named controls (Priority: P1)

**Goal**: Build any message the application can name, from controls labelled by meaning, with a live
byte preview and no hexadecimal required — plus a raw entry for those who prefer it.

**Independent Test**: Compose a control change on a chosen channel with a chosen value using only the
named controls, send it, and confirm the receiving end shows exactly that message —
[quickstart.md](./quickstart.md) §2.

- [X] T049 [US2] Add `set_composition_kind` and `compose_raw` to `crates/midi-core/src/application/sender.rs` (depends on T035)
- [X] T050 [US2] Add the `set_composition_kind` and `compose_raw` handlers to `src-tauri/src/commands.rs` and register them in `src-tauri/src/lib.rs` (depends on T049)
- [X] T051 [US2] Add the message-type picker to `src/components/send/Composer.tsx`, driven by the `kinds` the DTO supplies, so that switching type replaces the field set with exactly the values that message carries (depends on T045, T050)
- [X] T052 [P] [US2] Create `src/components/send/RawEntry.tsx` — a byte entry whose text is presentation state, submitted to `compose_raw` for validation, never parsed in the webview (depends on T050)
- [X] T053 [US2] Enforce ranges at entry in `src/components/send/Composer.tsx` — an out-of-range value is prevented or corrected and the permitted range is stated, using the `min` and `max` the DTO supplies rather than any number written in TypeScript (depends on T051)
- [X] T054 [US2] Add the `set_composition_kind` and `compose_raw` wrappers and the `MalformedSendBytes` `describeError` arm to `src/ipc.ts` (depends on T050)
- [ ] T055 [US2] Verify [quickstart.md](./quickstart.md) §2, including the field-list change on switching type, the range refusal, the three raw-entry refusals, and — the one that proves the design — that hand-typed `90 3C 00` transmits as `90 3C 00` and **not** as `80 3C 00`

**Checkpoint**: US1 and US2 both work independently. Anything the application can name can now be sent.

---

## Phase 5: User Story 3 — Publish a source under a chosen name (Priority: P2)

**Goal**: Other programs on the machine list a MIDI input under the user's chosen name and receive
from it. On Windows, the control says why it cannot and what to use instead.

**Independent Test**: Publish with a chosen name, confirm it appears in another application's MIDI
input list, send to it, and confirm that application receives it —
[quickstart.md](./quickstart.md) §3. On Windows, §4.

- [X] T056 [US3] Add `set_publication_name` and `set_published` to `crates/midi-core/src/application/sender.rs`, including clearing the chosen target when the published source stops being published and was the target (depends on T035)
- [X] T057 [US3] Include the published source in `Transmitter::targets()` when and only when it is published — `crates/midi-macos/src/source.rs` and `crates/midi-windows/src/source.rs` (which never publishes, so never lists it) (depends on T019, T023)
- [X] T058 [US3] Add the `set_publication_name` and `set_publication_enabled` handlers to `src-tauri/src/commands.rs`, pushing the updated target list through the target pump when publication state changes, and register them in `src-tauri/src/lib.rs` (depends on T056, T057)
- [X] T059 [US3] Persist the publication and restore it at launch in `src-tauri/src/lib.rs` and `crates/midi-core/src/application/settings.rs` — a saved `published: true` is a request, not a guarantee; failure is reported with its reason and disturbs nothing else (depends on T016, T058)
- [X] T060 [US3] Create `src/components/send/PublishRow.tsx` — the name field, the publish control, the name currently in force, and the platform message rendered from `PublicationSupportDto`. Where unsupported, the control **must not be operable**; the component holds no platform knowledge of its own (depends on T041, T058)
- [X] T061 [US3] Add the rename warning to `src/components/send/PublishRow.tsx` — the user is told before the change takes effect that a receiving program keeps its settings against the name it saw (depends on T060)
- [X] T062 [US3] Add the publication wrappers and the `PublicationUnsupported`, `PublicationFailed`, and `MalformedPublicationName` `describeError` arms to `src/ipc.ts` (depends on T058)
- [ ] T063 [US3] Verify [quickstart.md](./quickstart.md) §3 on macOS — including that the published source appears in this application's own Sources list and that sending to it while watching it produces rows in the event table
- [ ] T064 [US3] Verify [quickstart.md](./quickstart.md) §4 on Windows — the control states its unavailability and cannot be switched on, loopMIDI's port works as a target, the MIDI Mapper is absent, and every other capability of the screen works in full
- [ ] T065 [US3] Verify the honest-reporting case from §3: with the receiving program listing the device but nothing mapped, a send reports *transmitted* and does not claim the far end acted

**Checkpoint**: The disguise works on macOS, and Windows loses nothing but the disguise.

---

## Phase 6: User Story 4 — See what was sent (Priority: P2)

**Goal**: A record of what this session sent — time, what it was, its bytes, and whether it
succeeded — with any entry re-sendable.

**Independent Test**: Send several messages, confirm each is listed with its time and bytes, and that
a failed send is listed as failed rather than omitted — [quickstart.md](./quickstart.md) §5.

**Note**: US1 already calls `Sender::record` on every send, so the records exist by the time this
story starts. This story exposes them and adds re-sending.

- [X] T066 [US4] Add `recorded(id)` to `crates/midi-core/src/application/sender.rs` and enforce the `SEND_RECORD_LIMIT` cap, oldest evicted first (depends on T035)
- [X] T067 [US4] Add the `resend` handler to `src-tauri/src/commands.rs`, transmitting the record's stored bytes rather than re-encoding them, and register it in `src-tauri/src/lib.rs` (depends on T066)
- [X] T068 [US4] Create `src/components/send/SendLog.tsx` — each entry with its time, what it was, its bytes, and an outcome that makes a failure visibly different from a success, plus the re-send control (depends on T041, T067)
- [X] T069 [US4] Add the `resend` wrapper and the `UnknownSendRecord` `describeError` arm to `src/ipc.ts` (depends on T067)
- [ ] T070 [US4] Verify [quickstart.md](./quickstart.md) §5, including a genuine failure produced by unplugging the target mid-session, and a re-send of a raw-typed record sending what was typed

**Checkpoint**: A silent failure and an ignored message are now distinguishable, which is the whole
point of the story.

---

## Phase 7: User Story 5 — Keep the requests used often (Priority: P3)

**Goal**: Save a composed message under a name of your own, beside the built-ins, surviving restarts.

**Independent Test**: Save a composed message, relaunch, and confirm it is still listed and still
sends the same bytes — [quickstart.md](./quickstart.md) §6.

- [X] T071 [US5] Add `save`, `rename`, and `delete` to `RequestLibrary` in `crates/midi-core/src/domain/request.rs`, refusing rather than overwriting and mutating nothing when they refuse — `DuplicateRequestName` against saved names **and** built-in names, `BuiltInRequestImmutable`, `UnknownRequest` (depends on T034)
- [X] T072 [US5] Add `library_mut` and saved-request persistence to `crates/midi-core/src/application/sender.rs` and `PersistedComposition` to `crates/midi-core/src/application/settings.rs`, storing a raw request's bytes as typed so a restart sends identical bytes (depends on T016, T071)
- [X] T073 [US5] Add the `save_request`, `rename_request`, and `delete_request` handlers to `src-tauri/src/commands.rs` and register them in `src-tauri/src/lib.rs` (depends on T072)
- [X] T074 [US5] Add saving, renaming, and deleting to `src/components/send/RequestLibrary.tsx`, with saved requests visibly distinguishable from built-ins and no control offering to delete or overwrite a built-in (depends on T044, T073)
- [X] T075 [US5] Add the three request wrappers and the `DuplicateRequestName`, `BuiltInRequestImmutable`, `UnknownRequest`, and `MalformedRequestName` `describeError` arms to `src/ipc.ts` (depends on T073)
- [X] T076 [US5] Handle the duplicate-name-on-load case in `crates/midi-core/src/application/settings.rs` — first occurrence kept, the rest dropped, rather than refusing the whole document (depends on T072)
- [ ] T077 [US5] Verify [quickstart.md](./quickstart.md) §6, including the restart, the refusal to touch a built-in, and the duplicate-name refusal against both a saved name and a built-in name

**Checkpoint**: All five stories are independently functional.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: The audits the constitution makes part of done, and the verification that spans stories.

- [X] T078 [P] Update `README.md` — what the app does now that it can send, the built-in library, and the platform difference stated where it applies, matching the honest tone of the existing `What differs between the two platforms` section
- [X] T079 [P] Documentation audit across every file this feature touched: every new `pub` item and every new module carries a doc comment saying **why**, every new `CoreError` variant documents its condition and what a caller can do, and no doc restates a signature
- [X] T080 [P] Dead-code and magic-value audit: no `#[allow(dead_code)]`, no commented-out code, no literal that is not `0`, `1`, or `""` outside a named `const`, no `unwrap`/`expect`/`panic` outside `main`, no `any` or unchecked cast in the webview
- [X] T081 [P] Exhaustiveness audit: every `match` over `MidiMessage`, `SendableKind`, `FieldId`, `TargetKind`, `SendOutcome`, `PublicationSupport`, and `RequestOrigin` lists its variants with **no catch-all arm**, so a nineteenth message type is a compile error at every site
- [X] T082 Confirm `src/bindings.ts` is generated and was never hand-edited — `git diff` it against a fresh `npm run tauri dev` run and confirm no manual change survives
- [X] T083 Run the four gates on both platforms, plus the cross-compiled adapter check from T003 — `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, `npm run typecheck`, `npm run tauri dev`
- [ ] T084 Verify [quickstart.md](./quickstart.md) §7 — the two screens do not disturb each other: retained events, selections, filters, and the paused state all survive a screen switch; sending works while paused; and the **side-by-side comparison** of the monitoring screen against `screenshots/sources.png`, `screenshots/filters.png`, and `screenshots/data.png` shows nothing relabelled, reordered, restyled, or moved
- [ ] T085 Verify the settings document preserved in T002 loads with every setting intact — sources, filter, columns, retention — and gains an empty send section, per [contracts/persisted-settings.md](./contracts/persisted-settings.md)
- [ ] T086 Verify [quickstart.md](./quickstart.md) §8 — every edge in the table, including the long System Exclusive whole-or-nothing case, rapid repeated sending, publishing under a name a real device already has, and publishing with nothing subscribed
- [X] T087 Confirm no test artifact was introduced anywhere — no `tests/` directory, no `#[cfg(test)]`, no `*.test.ts`, no test dependency in any `Cargo.toml` or `package.json`, no runnable doctest

---

## Dependencies & Execution Order

### Phase dependencies

- **Setup (Phase 1)** — no dependencies
- **Foundational (Phase 2)** — depends on Setup. **Blocks every user story.** This is real here: nothing can send until the port exists and both adapters implement it
- **US1 (Phase 3)** — depends on Foundational
- **US2 (Phase 4)** — depends on Foundational. Extends `Composer.tsx`, created in US1, so it merges most cleanly after US1
- **US3 (Phase 5)** — depends on Foundational only. Fully independent of US1, US2, US4, US5
- **US4 (Phase 6)** — depends on US1, which is what produces the records
- **US5 (Phase 7)** — depends on US1 (`RequestLibrary`) and is most useful after US2, which is what produces something worth saving
- **Polish (Phase 8)** — depends on every story you intend to ship

### Within each story

Core types → service → command handlers → command registration and bindings regeneration → webview
components → `ipc.ts` wrappers → manual verification. The bindings regeneration is the gate: a
webview task cannot compile before the command it calls exists in `src/bindings.ts`.

### Parallel opportunities

**Phase 2** — three independent tracks after T004–T006:

- Domain: T007 → T008 → T009, with T010, T011, T012 alongside
- macOS adapter: T018 → T019 → T020
- Windows adapter: T021, T022 → T023 → T024

T014 gates both adapter tracks and should be done first by whoever starts.

**Phase 3+** — US1, US2, and US3 can be built in parallel by three people once Phase 2 is done. US4
and US5 must wait for US1.

**Marked `[P]` within phases**: T004/T005/T006; T008/T010/T011/T012; T018/T021/T022; T026/T032;
T042/T043/T044; T052; T078/T079/T080/T081.

### Merge points

`src/ipc.ts`, `src-tauri/src/commands.rs`, and `src-tauri/src/lib.rs` are touched by every story, in
different functions. `src/bindings.ts` is **generated** — after merging two stories, regenerate it
once rather than reconciling two generated copies.

---

## Parallel Example: Phase 2

```bash
# Three people, after T004, T005, T006 are merged:
Person A: "Create encoder.rs, sendable.rs, composition.rs in crates/midi-core/src/domain/"
Person B: "Add destination enumeration and implement Transmitter in crates/midi-macos/src/"
Person C: "Add output enumeration, create send.rs, implement Transmitter in crates/midi-windows/src/"

# T014 (the port) must land before B and C can start.
```

## Parallel Example: User Story 1

```bash
# After T041 creates the screen, three components are independent:
Task: "Create src/components/send/TargetPicker.tsx"
Task: "Create src/components/send/BytePreview.tsx"
Task: "Create src/components/send/RequestLibrary.tsx"
```

---

## Implementation Strategy

### MVP first (User Story 1 only)

1. Phase 1 — Setup, including the cross-compilation target
2. Phase 2 — Foundational (T004–T033). Unavoidable, and the bulk of the risk
3. Phase 3 — User Story 1 (T034–T048)
4. **STOP and VALIDATE**: [quickstart.md](./quickstart.md) §1 against a real device
5. Ship. Pick a destination, pick `Note On`, send it — the whole path proven with the smallest surface above it

### Incremental delivery

1. Setup + Foundational → both platforms build, nothing user-visible yet
2. US1 → verify §1 → **MVP, shippable**
3. US2 → verify §2 → shippable; anything nameable can be sent
4. US3 → verify §3 and §4 → shippable; the disguise works on macOS and Windows says why it cannot
5. US4 → verify §5 → shippable; failures are legible
6. US5 → verify §6 → shippable; the library is yours to extend
7. Polish → §7, §8, the gates, the audits

Each increment leaves the application building clean on both platforms and demonstrable.

### Three-person strategy

- All three: Phase 2, split along the three tracks above
- Then — Person A: US1, then US4. Person B: US2, then US5. Person C: US3, then Phase 8 audits
- Person C's story is the only one needing a second application to verify, and the only one that
  differs by platform, so it is worth giving to whoever has both machines

---

## Notes

- `[P]` means a different file with no dependency on an incomplete task
- `src/bindings.ts` is **generated**. If a task ever asks you to edit it by hand, the task is wrong
- Commit after each task or logical group; every commit should leave the gates passing
- **The cross-compile check is not optional.** Half of Phase 2 is per-platform, and the platform you
  are not on will not tell you it is broken until someone builds it
- Two behaviours that look like defects and are not, both documented where they occur: the published
  source appears in this application's **own** Sources list (T019), and a send reports *transmitted*
  rather than *received*, because what a receiving program does with it is unknowable from here (T046)
- No test task appears anywhere in this list, and none should be added — see the Tests note at the top

---

## Implementation status — 2026-08-29

**77 of 87 tasks complete.** Every task that produces code is done, and the four gates pass:

- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo fmt --check` — clean
- `npm run typecheck` — clean
- `cargo clippy -p midi-core -p midi-macos --target x86_64-apple-darwin -- -D warnings` — clean
- The application builds and launches on Windows with both screens wired in

**The ten open tasks are all manual verification**, and none of them can be completed from here:

| Task | What it needs |
|---|---|
| T048, T055, T070, T077, T086 | Driving the window by hand, with a MIDI device or a loopback utility attached |
| T063, T065 | **macOS**, plus a second application that lists MIDI inputs |
| T064 | A Windows machine with loopMIDI installed |
| T084 | A side-by-side comparison of the running window against the three reference images |
| T085 | Driving the window until a settings write occurs, then reading the document back |

They are left unchecked deliberately rather than marked done on the strength of the code compiling.
The gates the compiler can answer have been answered; the ones a person has to answer have not.

**One deviation from the plan, made during implementation and recorded here:**
`set_composition_field` takes its value as a **string** rather than a `u16`, which
[contracts/ipc-commands.md](./contracts/ipc-commands.md) specifies. The reason is System Exclusive:
its payload is a byte string with no numeric bounds, so a `u16` parameter could not carry it and the
screen would have needed a second command for one field. Taking the entry as text keeps one command,
and keeps parsing *and* range-checking in the core — where a `u16` would have forced the webview to
coerce first and therefore to hold a rule about what a valid value looks like.
