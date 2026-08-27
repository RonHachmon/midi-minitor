# Implementation Plan: Real MIDI Input

**Branch**: `002-real-midi-input` | **Date**: 2026-08-27 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/002-real-midi-input/spec.md`

---

## Summary

Replace the simulator with macOS CoreMIDI: real ports in the Sources list, real bytes in the table,
hot-plug tracked live, and a virtual destination other applications can send to. The simulator is
deleted, not disabled.

**The technical approach turns on one finding.** `midir` — the popular, cross-platform Rust MIDI crate,
and the obvious answer to "use popular crates" — was investigated first and is **rejected**. Its
CoreMIDI backend does one thing this feature cannot tolerate: on a byte that does not begin a valid
message, its decode loop executes `break`, abandoning the rest of the packet with no callback and no
error. That silently discards running status (FR-016) and malformed input (FR-020), the two things a
*monitor* exists to show. `midir` is built for applications that consume MIDI, where discarding garbage
is correct; a monitor is the one application where the garbage is the payload.

So the design uses [`coremidi`](https://crates.io/crates/coremidi) `0.9` directly — the very crate
`midir` wraps on macOS, actively maintained (0.9.2, August 2026), MIT, ~816k downloads — for device
access, hot-plug notifications, endpoint properties, and the virtual destination, and writes the
byte-stream decoder in the domain where `MidiMessage::Invalid` already exists. Full reasoning and
sources: [research.md](./research.md).

**One promise breaks, and the plan says so rather than glossing it.** Constitution VII stated that
substituting real MIDI would touch "no domain type, no use case, and no webview code". The event path
holds — `EventDto` is byte-identical, and filters, log, retention, and columns are untouched. Hot-plug
does not: `SourceCatalogue` is documented as fixed at startup with "no add or remove", and FR-009
contradicts that directly. Tracked in [Complexity Tracking](#complexity-tracking).

---

## Technical Context

**Language/Version**: Rust 1.98 (workspace `rust-version = 1.77`, edition 2021) · TypeScript 5.9 strict

**Primary Dependencies**: `coremidi` 0.9 (**new**, macOS-only) · Tauri 2 · `tauri-specta` 2.0-rc ·
`specta` · `serde` · `thiserror` · `chrono` · React 19 · Zustand 5 · `@tanstack/react-virtual` ·
Tailwind 4. **Removed**: `rand` (existed only to seed the simulator)

**Storage**: `tauri-plugin-store` — settings only; retained events stay session-only

**Testing**: **None.** The constitution's Verification Standard prohibits test files, test
dependencies, doctests, and CI test steps. Verification is zero-warning compilation plus the manual pass
in [quickstart.md](./quickstart.md)

**Target Platform**: macOS only (FR-039). CoreMIDI via `#[cfg(target_os = "macos")]` on the new crate

**Project Type**: Desktop application — Rust core + Tauri shell + React webview

**Performance Goals**: ≥ 500 messages/second sustained across ≥ 2 sources with every control responding
within 250 ms (SC-009); a note visible within 250 ms of the key (SC-001); catalogue reflecting a
plug/unplug within 2 s (SC-005)

**Constraints**: Bytes reported exactly as received (FR-014) · nothing dropped without a trace (FR-023)
· no fabricated events on any code path (FR-001) · the reference surface frozen (FR-036)

**Scale/Scope**: Single window, single event list. Typically < 10 endpoints; retention 1–100 000 events.
~1 000 lines of Rust across one new crate and edits to two existing ones, plus four webview files

---

## Constitution Check

*GATE: evaluated before Phase 0, re-evaluated after Phase 1 design.*

| Principle | Verdict | Evidence |
|---|---|---|
| **I. Code Quality** | **PASS** | Typed errors throughout; `Availability` and `Decoded` are exhaustive enums with no catch-all; `MAX_SYSEX_BYTES` named rather than literal; simulator and `rand` deleted rather than left dead |
| **II. Reuse Libraries** | **PASS with a recorded exception** | All OS integration comes from `coremidi`. The byte-stream decoder is hand-written; `wmidi`, `midi-msg`, and `midly` were evaluated and each returns `Err` for exactly the input FR-020 requires as a *row*. Justification goes in `decoder.rs`'s module docs, extending the argument [`message.rs`](../../crates/midi-core/src/domain/message.rs) already makes for the model ([D3](./research.md#d3-the-byte-stream-decoder-is-written-here-and-that-is-a-principle-ii-exception)) |
| **III. Documentation Why** | **PASS (obligation)** | Every new public item and module carries rustdoc/TSDoc stating *why*. Three sites carry load-bearing reasoning: the notification-client ordering ([D4](./research.md#d4-hot-plug-via-clientnew_with_notifications-created-on-the-main-thread-first)), the `SourceId`/`SourceKey` split ([D5](./research.md#d5-two-identities--a-session-handle-and-a-persistence-key)), and the decoder exception. Three existing doc comments become **false** and must be rewritten — see [Documentation debt](#documentation-debt) |
| **IV. Layered Architecture** | **PASS** | `midi-core` gains no platform dependency; `coremidi` is confined to the new `midi-macos` crate; `src-tauri` stays translation-only; the contract is generated into `src/bindings.ts`, never hand-written; no `serde_json::Value` or `any` at the boundary |
| **V. Named Patterns** | **PASS** | **Observer** for `CatalogueSink` — pressure named: the OS decides when the catalogue changes, so the application must be told rather than ask. **Adapter** for `CoreMidiSource`. No new abstraction without a second concrete pressure |
| **VI. Screenshots Authority** | **PASS** | No control relabelled, reordered, restyled, or repurposed. New text is confined to states the reference could not depict: no devices found, system unreachable, port unopenable, device absent, spying unavailable. The `Spy on output to destinations` group is **retained** — deleting it would violate this principle |
| **VII. Mock Foundation** | **PARTIAL — see Complexity Tracking** | Simulation removal is total (FR-002, SC-014) and the reference surface is undisturbed. But the substitution *does* touch domain types, so the "one-file change" promise does not hold |
| **Verification Standard** | **PASS** | No test artifacts. [quickstart.md](./quickstart.md) is the manual pass; the plan template's test-first phases are skipped per Governance, recorded [below](#skipped-template-phases) |

**Post-Phase-1 re-evaluation**: unchanged. The design surfaced no new violation. Phase 1 *narrowed* the
Constitution VII deviation — the IPC contract delta confirmed `EventDto`, `SnapshotDto`, the filter
types, and every filter/column/retention command are untouched, so the breach is bounded to source
identity and catalogue mutability.

---

## Project Structure

### Documentation (this feature)

```text
specs/002-real-midi-input/
├── spec.md                     # Complete (40 FRs, 16 SCs)
├── plan.md                     # This file
├── research.md                 # Phase 0 — 9 decisions, all unknowns resolved
├── data-model.md               # Phase 1 — entities by layer
├── contracts/ipc-contract.md   # Phase 1 — delta against feature 001
├── quickstart.md               # Phase 1 — 11 manual scenarios
├── checklists/requirements.md  # 16/16 pass
└── tasks.md                    # Phase 2 — NOT created here (/speckit-tasks)
```

### Source Code (repository root)

```text
crates/
├── midi-core/                       # No platform dependency. Ever.
│   └── src/
│       ├── domain/
│       │   ├── decoder.rs           # NEW — byte-stream state machine
│       │   ├── ids.rs               # CHANGED — SourceKey added
│       │   ├── source.rs            # CHANGED — key, Availability, replace()
│       │   ├── constants.rs         # CHANGED — MAX_SYSEX_BYTES
│       │   ├── message.rs           # unchanged (model already fits)
│       │   ├── event.rs, event_log.rs, filter.rs, column.rs   # unchanged
│       ├── application/
│       │   ├── ports.rs             # CHANGED — CatalogueSink, MidiSystemStatus
│       │   ├── monitor.rs           # CHANGED — remembered, replace_catalogue, status
│       │   ├── settings.rs          # CHANGED — selected_sources: Vec<SourceKey>
│       │   └── error.rs             # CHANGED — decode/port error variants
│       └── simulator/               # DELETED ENTIRELY
│
└── midi-macos/                      # NEW CRATE — the only place naming coremidi
    ├── Cargo.toml
    └── src/
        ├── lib.rs                   # module docs: why a separate crate
        ├── source.rs                # CoreMidiSource: EventSource impl
        ├── endpoints.rs             # enumeration → EndpointSnapshot → Source
        └── notifications.rs         # setup-change → CatalogueSink, debounced

src-tauri/src/
├── lib.rs                           # CHANGED — wiring; notification client FIRST
├── dto.rs                           # CHANGED — CatalogueDto, availability fields
├── commands.rs                      # CHANGED — subscribe_catalogue; two return types
├── state.rs                         # CHANGED — catalogue pump
├── stream.rs                        # CHANGED — catalogue channel; count dropped events
├── settings.rs, error.rs            # minor

src/                                 # webview
├── ipc.ts                           # CHANGED — new/changed signatures
├── store.ts                         # CHANGED — midiSystem, pushed catalogues
├── App.tsx                          # CHANGED — subscribe to catalogue channel
└── components/SourcesPanel.tsx      # CHANGED — availability + empty states
```

**Structure Decision**: A **new workspace crate `crates/midi-macos`** rather than putting the adapter in
`src-tauri`.

Three reasons. **Layering** — Constitution IV puts infrastructure in its own layer; folding CoreMIDI
into `src-tauri` would merge the infrastructure and IPC layers, and `src-tauri` is required to hold no
logic. **Containment** — a macOS-only dependency lives behind one crate boundary, so a future
cross-platform feature adds a sibling crate rather than editing a `cfg`-riddled file. **Symmetry** — it
occupies exactly the position `simulator/` did: one implementation of `EventSource`, swappable at one
line in `lib.rs`. That is the seam working as designed, even though the port's *shape* had to change.

---

## Implementation Phasing

Ordered so each step compiles and the application runs. `cargo clippy` clean and `tsc --noEmit` clean
are gates between steps, not a final sweep.

| # | Step | Delivers | Requirements |
|---|---|---|---|
| 1 | Domain foundations — `SourceKey`, `Availability`, `MAX_SYSEX_BYTES`, `SourceCatalogue::replace` | Compiles; simulator still running | FR-031–034 |
| 2 | `decoder.rs` — running status, SysEx accumulation, `Invalid` | Pure, no I/O; the Principle II exception | FR-014–020 |
| 3 | Port + `Monitor` — `CatalogueSink`, `MidiSystemStatus`, `remembered`, `replace_catalogue` | Simulator adapted to the new port shape | FR-009–012 |
| 4 | `midi-macos` — client, enumeration, one `InputPort` per source, decoding | **Real events reach the table** | US1, US2 |
| 5 | Hot-plug — notifications, debounce, `CatalogueSink` | Live catalogue | US3, FR-009 |
| 6 | Virtual destination | Inbound from other apps | US4, FR-024–027 |
| 7 | IPC delta — `CatalogueDto`, `subscribe_catalogue`, availability fields | Bindings regenerate | contract |
| 8 | Webview — availability, empty states, spy group, catalogue subscription | US5, honest empty states | FR-003, FR-028–030 |
| 9 | **Delete the simulator**, remove `rand`, rewrite the three stale doc comments | FR-002, SC-014 | FR-001–002 |
| 10 | Manual pass — all 11 quickstart scenarios | Definition of Done | all SCs |

**Step 9 is deliberately last.** Keeping the simulator until real input works end to end means every
earlier step has a working application to compare against — and the deletion becomes a single
irreversible commit rather than a rewrite performed blind. It is not optional and it is not deferred:
FR-002 and SC-014 are gates on this feature, not the next one.

---

## Documentation debt

Three existing doc comments become **false** and must be rewritten, not merely extended. Under
Constitution III, documentation is the primary substitute for a test suite here, so a doc that lies is a
defect of the same kind as a broken test:

| Location | Currently claims | Reality after this feature |
|---|---|---|
| [`source.rs`](../../crates/midi-core/src/domain/source.rs) — `SourceCatalogue` | "established once at startup… no add or remove" so ids never stop resolving | The catalogue changes at runtime; a `SourceId` **can** stop resolving |
| [`ports.rs`](../../crates/midi-core/src/application/ports.rs) — `catalogue()` | "Fixed for the lifetime of the process… callers may cache the result" | Only an initial snapshot; caching it is now a bug |
| [`lib.rs`](../../src-tauri/src/lib.rs) — module doc | "Step 4 is the only place that names the simulator. Swapping in real MIDI input is a change to that one line." | Honest at the time; the swap also changed the port and the domain. Rewrite to say what actually held and what did not |

---

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| **Notifications never fire under Tauri's run loop** | FR-009, US3 dead | The client must be created **first**, on the main thread, in `setup` — macOS fixes the notification thread at first client creation ([D4](./research.md#d4-hot-plug-via-clientnew_with_notifications-created-on-the-main-thread-first)). Fallback: poll `Sources` on a timer, contained to `notifications.rs`. Validate early — step 5, not step 10 |
| **Old settings file fails to deserialize** | Startup failure | `selected_sources` changes from `Vec<SourceId>` to `Vec<SourceKey>`. `load()` must treat a malformed store as `None` (first run), extending the existing "missing store is `None`" rule. Old simulator ids carry no meaning for real hardware |
| **One `InputPort` per source exhausts something** | US1 at scale | Real machines have single-digit port counts; CoreMIDI imposes no meaningful limit. The alternative costs byte fidelity ([D2](./research.md#d2-midi-10-packetlist-not-midi-20-eventlist)). Watch during Scenario 9 |
| **`EventPump` drops events silently under load** | FR-023, SC-009 | Pre-existing: `push` drops when the lock is contended. Acceptable for a simulator, reportable now. Must count drops and surface them |
| **Decoder mishandles interleaved Real Time inside SysEx** | FR-015, FR-020 | Real Time bytes may legally appear mid-SysEx and must not end the transfer. Called out explicitly in [data-model.md](./data-model.md); exercised by Scenario 7 |
| **No hardware available to verify** | Scenarios 3, 4, 7 unvalidated | IAC Driver covers most cases with real OS plumbing; hot-plug (Scenario 4) genuinely requires a cable |

---

## Complexity Tracking

> Constitution VII deviation. Recorded here rather than argued away.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| **`EventSource` port changes shape** (`CatalogueSink` added; `catalogue()` demoted to a snapshot) | FR-009 requires the OS to *tell* the application when ports change. A port that can only be asked cannot express this | Polling `catalogue()` from the Tauri layer leaves the port untouched — but puts refresh policy in the IPC layer, which Constitution IV forbids from holding logic, and trades a correct push for a latency/CPU tradeoff SC-005's 2 s bound would expose |
| **`SourceCatalogue` becomes mutable** (`replace`), contradicting its own "fixed at startup, no add or remove" doc | Hot-plug is the feature. The immutability was an assumption true of a simulator and false of hardware | Rebuilding the whole `Monitor` on every change would discard the `EventLog`, violating FR-011 (events from a departed device must remain listed) |
| **Two identities: `SourceId` + `SourceKey`** | FR-031 needs an identity surviving replug and reboot; the IPC contract needs a dense `u32`. CoreMIDI's `kMIDIPropertyUniqueID` is a sparse `i32` | One type forces either a negative number through the contract, or persistence keyed on a value that changes between sessions — which FR-031 forbids outright |
| **`Monitor` gains `remembered`** | FR-033: quitting while a device is unplugged must not erase its selection. `selected_ids()` returns only *present* sources, so today's save would silently drop it | Persisting the raw catalogue would restore stale device rows as if attached, misrepresenting what is connected |
| **Hand-written decoder** (Principle II exception) | `MidiMessage::Invalid`, running status, and truncated SysEx must be **values**, not errors. Every candidate crate returns `Err` for precisely these | `wmidi` for valid messages plus a fallback for the rest means two decoders disagreeing about message boundaries — a worse failure mode, and the fallback needs the same state machine anyway |

**What Constitution VII got right, and it is most of it**: the event path never moved. `EventDto`,
`EventLog`, `FilterSettings`, `ColumnVisibility`, and every filter, retention, and column command are
untouched, and the adapter still slots in at one line in `lib.rs`. The seam absorbed the substitution it
was built for. What it could not absorb was an assumption *underneath* it — that the set of sources
never changes — which no amount of port discipline would have hidden.

### Skipped template phases

Per Governance, the plan template's test-oriented phases (contract tests, integration tests,
test-first ordering) are **skipped**: the Verification Standard prohibits test artifacts in this
repository. [quickstart.md](./quickstart.md) carries the equivalent verification as 11 manual scenarios.
