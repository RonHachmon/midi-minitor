# Implementation Plan: A Send Screen With Built-In Requests and a Chosen Identity

**Branch**: `005-midi-send-console` | **Date**: 2026-08-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/005-midi-send-console/spec.md`

## Summary

This is the first feature that makes the application **emit** MIDI. Until now every port, type, and
screen assumed one direction, so the shape of the work is set by one question: where does the second
direction attach without disturbing the first?

The answer is a **second application port, `Transmitter`**, sitting beside `EventSource` and
implemented by the two adapters that already exist. `EventSource` is not widened and not renamed; its
contract, which two platforms and four features depend on, is untouched. The two ports are bundled by
a one-line supertrait, `MidiAccess`, for a reason that is forced rather than chosen: on macOS both
directions must share a single CoreMIDI client, and that client is created at one exact moment on one
exact thread or hot-plug notification silently stops working.

Four decisions carry the rest of the design.

1. **Encoding is domain code, and the webview never computes bytes.** A new
   `domain/encoder.rs` mirrors the existing `decoder.rs` and becomes the byte-encoding boundary that
   `constants.rs` has referred to since feature 001 — the one place a 1-based display channel becomes a
   0-based wire nibble. The byte preview *is* the encoding, carried to the screen as data, which makes
   "what you see is what is sent" (SC-004) one value rather than two that must agree.

2. **The composition lives in the core.** A new `Sender` service holds the message being built, the
   chosen target, the publication, the request library, and the session's send records — the
   counterpart of `Monitor`, and one service for the same reason `Monitor` is one. That places value
   ranges (FR-016) and the table of which message carries which fields (FR-013) in the layer that owns
   them, and makes "what you were composing survives moving between screens" (FR-019) free rather than
   a component-lifetime problem.

3. **Hand-typed bytes are validated by the decoder that already exists.** `MessageDecoder` was
   written for this project, already handles running status and truncated System Exclusive, and
   already names what is wrong with bad input. Writing a second validator would produce a send screen
   that refuses to transmit things the monitor happily displays.

4. **The platform difference travels as data, through the mechanism built for it.**
   `PlatformCapabilities` gains a `publication` field, exactly as it carries `byte_fidelity` today.
   macOS reports `Supported`; Windows reports `Unsupported` with the reason and the loopback-utility
   route. No layer above the adapter learns which platform it is on — `src-tauri/src/platform.rs`
   remains the only file in the application that names an operating system.

**No new dependency.** Every call this feature needs was verified present in the vendored sources of
crates already in the workspace: `coremidi` 0.9.2 for `virtual_source`, `output_port`, and
`MIDIReceived`; `windows` 0.62.2 for the `midiOut*` family, under a feature the manifest already
enables.

**No new pattern.** `Command` for the handlers, `Ports & Adapters` for the seam, and `Newtype` for the
identifiers are all already in use. The `outgoing()`/`record()` two-step that keeps the port out of
`midi-core` is the shape `AppState::sync_ports` established in feature 002.

**The one thing that grows**: the window gains a screen switcher, and the monitoring screen moves into
a component of its own. No control the screenshots depict is relabelled, reordered, restyled, or
repurposed — this feature adds surface and edits none.

## Technical Context

**Language/Version**: Rust (workspace edition 2021, MSRV 1.77); TypeScript 5 strict in the webview

**Primary Dependencies**: existing only — `tauri` 2, `tauri-specta`/`specta`, `tauri-plugin-store`,
`serde`, `thiserror`, `coremidi` 0.9 (macOS), `windows` 0.62 (Windows), `zustand`,
`@tanstack/react-virtual`. **Nothing new** — see [research.md](./research.md) D16

**Storage**: `tauri-plugin-store` JSON through the existing `SettingsRepository` port. One additive
`#[serde(default)]` field; documents from features 001–004 load unchanged and **no migration is
written** — see [contracts/persisted-settings.md](./contracts/persisted-settings.md)

**Testing**: None. The constitution's Verification Standard prohibits a test suite, and the Spec Kit
templates' test phases are skipped under Governance. Verification is `cargo clippy` clean,
`cargo fmt --check` clean, `tsc --noEmit` clean, and the manual pass in [quickstart.md](./quickstart.md)

**Target Platform**: macOS and Windows 10 1809+ / Windows 11. Sending works identically on both.
Publishing a source is macOS-only, reported as an adapter capability and surfaced on the control it
affects — never as a conditional above the adapter

**Project Type**: Tauri desktop application — Rust workspace (domain/application core, per-platform
adapters, Tauri shell) plus a React webview

**Performance Goals**: a send completes within one user-visible interaction and reports its outcome
(SC-003); the byte preview updates on the same interaction as the value change that caused it
(FR-015); the target list reacts to a device change as quickly as the Sources list does, since both
ride the same notification; opening or using the send screen costs the ingest path nothing (SC-006)

**Constraints**: bytes transmitted are byte-for-byte the bytes previewed (SC-004); a message is
transmitted whole or not at all (FR-035); a refused command mutates nothing; no control is
operable-but-inert on any platform (SC-007); no depicted control is relabelled, reordered, or restyled
(FR-003, SC-008); zero `unwrap`/`expect`/`panic` outside `main`; zero clippy warnings; no `any` in the
webview

**Scale/Scope**: 6 new modules and 4 changed files in `midi-core`; 2 changed files per platform
adapter; 6 changed files in `src-tauri`; 11 files in the webview (9 new, 2 changed), plus README. 13
IPC commands added, 1 subscription added, **none removed or changed**. 16 `CoreError` variants added

## Constitution Check

*GATE: evaluated before Phase 0, re-evaluated after Phase 1 design. Result: **PASS**, with one entry
in Complexity Tracking.*

| Principle | Assessment | Verdict |
|---|---|---|
| **I. Code quality** | Purpose-built types throughout: `Target`, `Composition`, `SendRecord`, `Publication`, and five newtypes rather than strings and tuples. `SendOutcome` is an enum, not `Option<String>`, so "it worked" is a named fact. Sixteen typed `CoreError` variants, each carrying what its message must quote. Every range comes from the existing constants, never a repeated literal. `Composition`'s two variants exist because *which value is authoritative* differs between them — a distinction a single struct would leave unstated. | PASS |
| **II. Reuse libraries** | No new dependency, and the one thing that could have been hand-rolled — validating hand-typed bytes — reuses this project's own `MessageDecoder` instead (research D4). Both adapters use the vendored bindings already in the manifest; `midir` is rejected for output for the reason it was rejected twice before, plus a new one (two MIDI stacks in one process). `CataloguePump` is generalised rather than copied (D13). | PASS |
| **III. Documentation explains why** | Six new modules, each with a module-level *why*. The load-bearing ones are pre-identified in [data-model.md](./data-model.md): why encoding is the channel-offset boundary; why `Composition` has two variants; why `SendableKind` is a third message-shaped enum and not duplication; why `Sender` does not hold the port; why the encoder's error is unreachable yet must exist; why a published source appears in this application's own Sources list. | PASS |
| **IV. Layered architecture + typed IPC** | Every rule lands in `midi-core`: encoding, ranges, field tables, clash-free naming, target resolution. The twelve handlers validate, delegate, persist, and map errors. The webview computes no bytes and holds no MIDI knowledge — it renders `SendViewDto`, the same way `FilterPanel.tsx` renders `FilterViewDto`. All DTOs are declared in Rust and regenerate `src/bindings.ts`; no hand-written parallel types, no `any`, no `serde_json::Value` in a signature. Errors cross as typed variants. | PASS |
| **V. Named patterns, never ceremony** | One new port, justified by two real implementations on day one — the same pressure that justified `EventSource`. The `MidiAccess` bundle is one line answering a stated constraint (the shared CoreMIDI client), not an anticipated one. Rejected as ceremony and recorded in research: a `SendService`/`TargetService` split, a `RequestId` newtype, a `send_request` fused command, a `preview_bytes` query, a scheduling parameter, and a fan-out API. | PASS |
| **VI. Screenshots are the design authority** | The monitoring screen is moved into `MonitorScreen.tsx` **unchanged** — same components, same order, same labels, same defaults. `Sources`, the `Filter` columns, `All Channels` / `One Channel`, `Remember up to … events`, `Clear`, `Pause`, and the five table columns are untouched. The send screen appears in no reference image, so it is designed for usability under the spec. Verification is the side-by-side comparison in [quickstart.md](./quickstart.md) scenario 7. | PASS |
| **VII. Extend rather than edit** | New capability earns new surface: a new screen and a switcher, with nothing depicted altered. Every new enum is matched exhaustively with no catch-all, so a nineteenth message type is a compile error in the encoder, in `SendableKind`, and in the decoder at once. `PlatformCapabilities` grows by one field rather than sprouting a parallel mechanism. The mock-containment rule is unaffected: nothing here invents data, and every byte sent is a byte the user composed. | PASS |
| **Verification Standard** | No tests added; no test files, dependencies, or CI test steps. The templates' test phases are skipped, recorded here per Governance. Manual verification is specified in [quickstart.md](./quickstart.md), including the cross-compiled check of the adapter the developer is not on — which this feature makes newly important, because it touches **both** adapters. | PASS (skip recorded) |

**Re-evaluation after Phase 1**: unchanged. The design added no type, port, or indirection beyond the
one tracked item, and that item is forced by a documented platform constraint rather than by
anticipation.

## Project Structure

### Documentation (this feature)

```text
specs/005-midi-send-console/
├── plan.md              # This file
├── research.md          # Phase 0 — D1..D16, each with the alternatives rejected
├── data-model.md        # Phase 1 — new and changed types, and their layer
├── quickstart.md        # Phase 1 — gates, then eight manual scenarios
├── contracts/
│   ├── transmitter-port.md    # The new port both adapters must satisfy
│   ├── ipc-commands.md        # 12 commands and 1 subscription added
│   └── persisted-settings.md  # The additive settings field; why no migration
├── checklists/
│   └── requirements.md
└── tasks.md             # Phase 2 — created by /speckit-tasks, not here
```

### Source Code (repository root)

```text
crates/midi-core/src/
├── domain/
│   ├── encoder.rs       # NEW — MidiMessage -> bytes; the channel-offset boundary
│   ├── sendable.rs      # NEW — SendableKind, FieldSpec, FieldId
│   ├── composition.rs   # NEW — Composition { Guided, Raw }; raw parsed by the decoder
│   ├── target.rs        # NEW — Target, TargetKind
│   ├── request.rs       # NEW — Request, RequestLibrary, the 15 built-ins
│   ├── publication.rs   # NEW — Publication { name, published }
│   ├── send_record.rs   # NEW — SendRecord, SendOutcome
│   ├── ids.rs           # + TargetId, TargetKey, SendRecordId, RequestName, PublishedName
│   └── mod.rs           # + the six new modules
├── application/
│   ├── sender.rs        # NEW — the Sender service; Outgoing
│   ├── ports.rs         # + Transmitter, MidiAccess, PublicationSupport; capabilities gains a field
│   ├── settings.rs      # + SendSettings on PersistedSettings (serde default)
│   ├── error.rs         # + 16 CoreError variants
│   └── mod.rs           # + pub mod sender;
└── constants.rs         # + SEND_RECORD_LIMIT, name lengths, DEFAULT_PUBLISHED_NAME, IDENTITY_REQUEST

crates/midi-macos/src/
├── source.rs            # + OutputPort, VirtualSource; impl Transmitter; capabilities gains Supported
└── endpoints.rs         # + destination enumeration and keying

crates/midi-windows/src/
├── source.rs            # + midiOut handle; impl Transmitter; capabilities gains Unsupported
├── endpoints.rs         # + midiOutGetNumDevs / midiOutGetDevCapsW enumeration
└── send.rs              # NEW — short and long (SysEx) output, MIDIHDR lifetime

src-tauri/src/
├── platform.rs          # event_source -> midi_access, returning Box<dyn MidiAccess>
├── state.rs             # + Sender in managed state; + transmit(); + refresh_targets()
├── commands.rs          # + 13 command handlers and 1 subscription
├── dto.rs               # + SendViewDto and its members; + TargetsDto
├── error.rs             # + 16 IpcError variants and their From arms
├── stream.rs            # CataloguePump generalised to PushPump<T>; + TargetPump alias
└── lib.rs               # command registration; Sender construction; targets on catalogue change

src/
├── App.tsx              # + the screen switcher; monitor body moves out
├── bindings.ts          # REGENERATED — never edited by hand
├── ipc.ts               # + 14 call wrappers, + 16 describeError arms
├── sendStore.ts         # NEW — the send screen's mirrored model and presentation state
├── screens/
│   ├── MonitorScreen.tsx   # NEW — the existing monitor body, moved unchanged
│   └── SendScreen.tsx      # NEW — the send screen's layout
└── components/send/
    ├── TargetPicker.tsx    # NEW — where traffic goes
    ├── PublishRow.tsx      # NEW — the disguise, and the platform message
    ├── RequestLibrary.tsx  # NEW — built-ins and saved requests
    ├── Composer.tsx        # NEW — the kind picker and the value fields
    ├── BytePreview.tsx     # NEW — what will be sent
    ├── RawEntry.tsx        # NEW — hand-typed bytes
    └── SendLog.tsx         # NEW — what was sent, and re-send

README.md                # what the app does — sending, and the platform difference
```

**Structure Decision**: no structural change to the workspace. The feature fits the existing four-layer
shape (domain → application → adapters → IPC surface → webview) and needs no new crate. Two new
directories appear in the webview — `src/screens/` and `src/components/send/` — because there are now
two screens, and leaving eight send components loose beside the seven monitor ones would make the flat
directory the thing a reader has to decode.

## Key Design Decisions

### 1. A second port, not a wider one

```rust
pub trait Transmitter: Send {
    fn targets(&self) -> Vec<Target>;
    fn transmit(&mut self, target: TargetId, bytes: &[u8]) -> Result<(), CoreError>;
    fn publish(&mut self, name: &PublishedName) -> Result<(), CoreError>;
    fn unpublish(&mut self);
}

pub trait MidiAccess: EventSource + Transmitter {}
impl<T: EventSource + Transmitter> MidiAccess for T {}
```

`EventSource` is about *arrival*: sinks, catalogue push, and a deliberate inability to tell
implementations apart. Transmission is caller-driven, synchronous, and returns a result per call.
Folding the two together would give one trait two reasons to change.

The bundle is the part that needs justifying, and the justification is a platform constraint the
codebase already documents as fragile. A `MIDIClientRef` parents every port and virtual endpoint an
application owns, and this application's client is created inside `EventSource::start`, on the main
thread, because macOS binds hot-plug notification delivery to the thread and run loop current at that
instant — wrongly placed, it fails silently. A separate transmitting object would need either a second
client (an avoidable risk) or the first one handed through the shell (a `coremidi::Client` in
`src-tauri`, which is the layering violation the whole arrangement exists to prevent).

Full contract, including what the port deliberately does not expose, in
[contracts/transmitter-port.md](./contracts/transmitter-port.md).

### 2. The preview is the encoding, not a second opinion

```rust
// domain/encoder.rs
pub fn encode(message: &MidiMessage) -> Result<Vec<u8>, CoreError>;
```

SC-004 promises that the bytes shown before sending are the bytes transmitted. Two implementations that
must agree is a promise with a hole in it; one value used twice has none. So the core encodes, the DTO
formats (`90 3C 64`), and the screen renders — and the same `Vec<u8>` is what reaches the adapter.

This is also where `constants.rs` has been pointing since feature 001: *"conversion happens once at the
byte-encoding boundary so that no display or filtering code can forget the offset."* That boundary did
not exist until this feature. It does now, and the 1-based-to-0-based channel conversion happens there
and nowhere else.

The `Result` is unreachable in practice — `MidiMessage::Invalid` cannot be reached through the
composition API — and it stays anyway, because `MidiMessage` models what a *monitor* can observe.
Narrowing the monitor's message type to what a sender can emit would damage the older feature to serve
the newer one.

### 3. Raw bytes are authoritative for raw compositions

```rust
pub enum Composition {
    Guided { message: MidiMessage },              // bytes derived from the message
    Raw { bytes: Vec<u8>, message: MidiMessage }, // bytes are what was typed
}
```

Type `90 3C 00` — a Note On with velocity zero — and it must go out as `90 3C 00`. Re-encoding the
decoded message would produce `80 3C 00`, because a zero-velocity Note On *means* a release. That is
precisely the lossiness `MidiEvent` already documents for received data, running in the opposite
direction, and it is why the two variants differ in which field is the authority rather than merely in
how they were created.

`fields()` returns an empty slice for `Raw`, so there is nothing to edit and no error to invent.

### 4. `Sender` decides what to send; the shell performs it

```rust
let outgoing = sender.outgoing()?;                 // what, and where — a core decision
let outcome  = transmitter.transmit(outgoing.target, &outgoing.bytes);
sender.record(&outgoing, outcome.into(), clock.now());
```

`Sender` does not hold the `Transmitter`, exactly as `Monitor` does not hold the `EventSource`. The
shell coordinates the two and decides nothing — the same shape `AppState::sync_ports` has used since
feature 002, where the monitor's state drives the adapter and the adapter's failures are fed back.

This is what keeps `midi-core` free of the port object while leaving every rule inside it: which target,
which bytes, whether a target is chosen at all, and what the record says are all `Sender`'s answers.

### 5. Targets ride the notification that already exists

One physical event — a cable moved — changes the source list and the target list together, and both
platforms report it through the single notification the application already subscribes to. So the
catalogue callback in `lib.rs` gains a sibling call:

```rust
monitor.replace_catalogue(sources);
state.sync_ports(&mut monitor);
state.publish_catalogue(&monitor);
state.refresh_targets();          // re-read Transmitter::targets(), update Sender, push
```

A second sink on the port was rejected: it would mean two callbacks for one notification and would make
each adapter classify which list a given change affects — a judgement neither can make reliably.

### 6. The platform difference is a capability, not a check

```rust
pub enum PublicationSupport {
    Supported,
    Unsupported { detail: String },
}
```

Windows returns `Unsupported`, with prose naming the loopback-utility route, written by the adapter in
its own words — the pattern `Availability::Unsupported` and `MidiSystemStatus::Unavailable` already
set. The screen renders what it is told and holds no platform knowledge, so `platform.rs` stays the
only file in the application that names an operating system.

`publish()` still returns `PublicationUnsupported` on Windows, but no user is expected to reach it: the
control is not operable, because a control that looks live and does nothing is what SC-007 forbids.
That error is a backstop against a stale webview.

### 7. What the screens share, and what they do not

The window gains a switcher; the monitoring body moves into `MonitorScreen.tsx` **unchanged**. The
event subscription stays where it is, at `App` level, so switching screens cannot unsubscribe it —
which is most of FR-005 and SC-006 handled by placement rather than by logic.

Monitoring state already lives in Rust, so nothing about the monitor depends on its components staying
mounted. Retained events, filters, selections, and the paused state survive a screen switch because
they were never in the webview to begin with.

## Phase 2 note for `/speckit-tasks`

The spec's five user stories are already independently shippable, and the priorities give the slicing:

| Slice | Delivers | Depends on |
|---|---|---|
| Foundation | `Transmitter`, `MidiAccess`, encoder, `Sender`, both adapters' `transmit` | — |
| US1 (P1) | Built-in requests, target picker, send, outcome | Foundation |
| US2 (P1) | Composer, fields, byte preview, raw entry | Foundation |
| US3 (P2) | Publishing, the name, the Windows message | Foundation |
| US4 (P2) | Send records and re-send | US1 |
| US5 (P3) | Saved requests and their persistence | US2 |

US1 alone is a usable feature: pick a destination, pick `Note On`, send it. That is the natural first
milestone and the one worth getting in front of a device early, because it exercises the whole path —
port, adapter, encoder, IPC, screen — with the smallest possible surface above it.

## Complexity Tracking

> Filled only where the Constitution Check found something that must be justified.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| `MidiAccess` supertrait bundling two ports into one object | On macOS, receiving and transmitting must share one `MIDIClientRef`, and that client is created at one specific moment on one specific thread — `EventSource::start`, on the main thread — because CoreMIDI binds hot-plug notification delivery to whatever run loop is current at that instant, silently and with no error when it is wrong. The shell must therefore hold **one** object that does both. | *Two independent adapter objects* would need either a second CoreMIDI client (an avoidable risk against a constraint this codebase already documents as fragile and silent when broken) or the first client passed through `src-tauri`, which would put `coremidi::Client` in the layer forbidden to name a platform. *`EventSource::transmitter(&mut self)`* would make one port responsible for vending the other, and its borrow would conflict with the `&mut self` the shell needs for `sync_ports` in the same scope. The bundle is one line, one blanket impl, and it names the constraint it answers. |

Two items considered for this table and deliberately left out, because on inspection neither departs
from the constitution:

- **A third message-shaped enum (`SendableKind`).** It looks like duplication and is not: the three
  types answer three different questions and change for three different reasons — what was observed,
  which filter checkbox it falls under, and what can be composed. A payload-free discriminant cannot
  drift the way a second payload enum would, and every `match` over it is exhaustive. See
  [research D6](./research.md#d6--sendablekind-is-a-third-message-shaped-enum-and-that-is-correct).
- **Six new domain modules.** Each holds one concept and would have to exist somewhere regardless;
  putting them in one `send.rs` would produce a file with six reasons to change, which is the thing
  Principle I actually prohibits.
