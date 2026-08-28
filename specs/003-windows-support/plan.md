# Implementation Plan: Windows Support

**Branch**: `003-windows-support` | **Date**: 2026-08-28 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/003-windows-support/spec.md`

## Summary

Add a `midi-windows` sibling crate implementing `midi_core::application::ports::EventSource`,
occupying exactly the position `midi-macos` holds, and select it at the composition root with
`cfg(target_os)`. Windows device access goes through classic WinMM (`midiInOpen`) because it is the
only Windows path that reports malformed bytes; identity comes from the device-interface string;
hot-plug comes from `CM_Register_Notification`, which needs no window handle and feeds the existing
`Debouncer` unchanged. See [research.md](./research.md) for the evidence behind each.

Three things the platform genuinely forces reach above the adapter, and nothing else does:

1. **`SourceKey` gains a `DeviceInterface(String)` variant.** Windows has no signed integer identity.
   The existing `Endpoint(i32)` variant is untouched, so macOS settings files keep loading.
2. **`Availability` gains an `Unsupported` variant**, so a control can be present, visible, and
   *not operable*, with a reason — which is what the spec's Decision 1 requires of the
   `Act as a destination for other programs` row on Windows.
3. **`EventSource` gains a `capabilities()` method** returning, among other things, how faithfully the
   platform reports bytes. This is how the spec's Decision 2 statement reaches the `Data` column
   without a single platform conditional in the webview.

The pre-existing coupling — the Tauri shell naming the concrete `CoreMidiSource` type, including its
non-trait `sync_ports` — is resolved by moving `sync_ports` into the port and having the shell hold
`Arc<Mutex<dyn EventSource>>`. After that the concrete adapter names appear exactly twice in the
repository, both inside `cfg` blocks in one new file.

## Technical Context

**Language/Version**: Rust (workspace `rust-version = "1.77"`, edition 2021); TypeScript 5 strict in the webview

**Primary Dependencies**: existing — `tauri` 2, `tauri-specta`/`specta`, `tauri-plugin-store`, `serde`, `thiserror`, `chrono`, `coremidi` 0.9 (macOS only). New — `windows` (or `windows-sys`), Windows only, features `Win32_Media_Multimedia` and `Win32_Devices_DeviceAndDriverInstallation`

**Storage**: `tauri-plugin-store` JSON, via the existing `SettingsRepository` port. Schema effect: one new `SourceKey` variant, backward compatible

**Testing**: None. The constitution's Verification Standard prohibits a test suite; the Spec Kit templates' test phases are skipped under Governance. Verification is `cargo clippy` clean, `tsc --noEmit` clean, and a manual pass on **both** platforms

**Target Platform**: macOS (unchanged) and Windows 10 1809+ / Windows 11, x64 desktop

**Project Type**: Tauri desktop application — Rust workspace (domain/application core, per-platform adapters, Tauri shell) plus a React webview

**Performance Goals**: a played note visible within 250 ms (SC-001); Sources list reflects plug/unplug within 2 s (SC-005); dense multi-device traffic without dropped or reordered rows (FR-029, FR-030)

**Constraints**: no message dropped from the record; no byte displayed that was not received; the WinMM callback must call no multimedia function (research D5); zero `unwrap`/`expect`/`panic` outside `main`; zero clippy warnings

**Scale/Scope**: one new crate (~5 modules), 3 additive type changes in `midi-core`, 1 new file plus edits to 3 files in `src-tauri`, 2 small webview changes, README and quickstart. No new IPC commands

## Constitution Check

*GATE: evaluated before Phase 0, re-evaluated after Phase 1 design. Result: **PASS**, with four
entries in Complexity Tracking.*

| Principle | Assessment | Verdict |
|---|---|---|
| **I. Code quality** | Typed errors reused (`CoreError`); newtypes untouched; every WinMM literal becomes a named `const` in the adapter. FFI forces `unsafe`, which the constitution does not forbid but Principle III does require justifying: every `unsafe` block gets a comment stating the invariant that makes it sound. `unwrap_used`/`expect_used`/`panic` stay denied — FFI results are matched, never unwrapped. | PASS |
| **II. Reuse libraries** | `windows` is Microsoft's own generated binding; hand-declaring `extern "system"` signatures would be the re-solving this principle forbids. `midir` was evaluated and rejected on evidence (research D1), and the rejection is recorded in the crate's module docs as the principle requires. | PASS |
| **III. Documentation explains why** | Every new `pub` item and module carries a doc comment stating *why*. The three highest-value ones are pre-identified: why WinMM rather than the newer stack; why the callback hands off instead of decoding in place; why `Absent` is unreachable here. | PASS |
| **IV. Layered architecture + typed IPC** | Device access stays in infrastructure. The shell keeps holding no rules — moving `sync_ports` into the port *removes* a concrete-type dependency it should never have had. The DTO change regenerates TypeScript from the Rust declarations; no hand-written parallel types. | PASS |
| **V. Named patterns, never ceremony** | No new pattern. This is Ports & Adapters doing the exact job it was introduced for: a second adapter behind an existing port. The `dyn EventSource` move is the *removal* of an accidental coupling, not a new abstraction. | PASS |
| **VI. Screenshots are the design authority** | No depicted control is relabelled, reordered, restyled, or repurposed. Two platform-forced deviations, both permitted by the principle's escape clause and both documented at the code: the destination checkbox is not operable on Windows, and native chrome/font/control rendering follow the platform. | PASS |
| **VII. Extend rather than edit** | All three core changes are **added variants**, not edits: `SourceKey::DeviceInterface`, `Availability::Unsupported`, `ByteFidelity`. Every match site stays exhaustive with no catch-all arm, so the compiler enumerates the work. New interface text is new surface; the reference surface is untouched. | PASS |
| **Verification Standard** | No tests added. The Spec Kit template's test phases are skipped, recorded here per Governance. Manual verification on both platforms is specified in [quickstart.md](./quickstart.md). | PASS (skip recorded) |

**Re-evaluation after Phase 1**: unchanged. The design added no abstraction beyond the four tracked
items, and each of those is forced by a concrete, present pressure rather than an anticipated one.

## Project Structure

### Documentation (this feature)

```text
specs/003-windows-support/
├── plan.md              # This file
├── research.md          # Phase 0 — D1..D8, each with published sources
├── data-model.md        # Phase 1 — the three additive type changes and their reach
├── quickstart.md        # Phase 1 — build and verify on both platforms
├── contracts/
│   ├── event-source-port.md    # The internal port contract both adapters satisfy
│   └── persisted-settings.md   # The on-disk settings contract and its compatibility
├── checklists/
│   └── requirements.md
└── tasks.md             # Phase 2 — created by /speckit-tasks, not here
```

### Source Code (repository root)

```text
crates/
├── midi-core/
│   └── src/
│       ├── domain/
│       │   ├── ids.rs           # + SourceKey::DeviceInterface(String); derive Clone, drop Copy
│       │   └── source.rs        # + Availability::Unsupported { detail }; .clone() at key sites
│       ├── application/
│       │   ├── ports.rs         # EventSource gains sync_ports, capabilities, dropped_events
│       │   │                    #   + PlatformCapabilities, ByteFidelity
│       │   └── monitor.rs       # holds and exposes PlatformCapabilities; key clones
│       └── support/
│           └── debounce.rs      # MOVED here from midi-macos, unchanged (see Complexity Tracking)
│
├── midi-macos/                  # unchanged in behaviour
│   └── src/
│       ├── lib.rs               # re-export of Debouncer removed; now imports from midi-core
│       ├── endpoints.rs         # SourceKey::Endpoint construction gains a clone
│       ├── notifications.rs     # DELETED — contents moved to midi-core::support::debounce
│       └── source.rs            # impl EventSource gains sync_ports/capabilities/dropped_events
│                                #   (moved from inherent impl); returns ByteFidelity::AsTransmitted
│
└── midi-windows/                # NEW — mirrors midi-macos module for module
    ├── Cargo.toml               # windows crate, target-gated to Windows
    └── src/
        ├── lib.rs               # #![cfg(target_os = "windows")]; why WinMM, why not midir
        ├── endpoints.rs         # midiInGetNumDevs/midiInGetDevCapsW + DRV_QUERYDEVICEINTERFACE
        ├── notifications.rs     # CM_Register_Notification -> midi-core Debouncer
        ├── receive.rs           # the MidiInProc callback and its hand-off channel
        ├── sysex.rs             # MIDIHDR buffer pool: prepare, add, requeue off the callback
        └── source.rs            # WindowsMidiSource: impl EventSource

src-tauri/
├── Cargo.toml                   # adapter deps become target-gated
└── src/
    ├── platform.rs              # NEW — the only cfg(target_os) in the application
    ├── lib.rs                   # constructs via platform::event_source(); holds dyn EventSource
    ├── state.rs                 # source: Arc<Mutex<dyn EventSource>> (no concrete type named)
    └── dto.rs                   # availability becomes a tagged union; + byte-fidelity note

src/                             # webview
├── components/
│   ├── SourcesPanel.tsx         # renders an inoperable row with its reason
│   └── EventTable.tsx           # renders the byte-fidelity note with the Data column
└── bindings.ts                  # regenerated, not edited

README.md                        # "macOS" -> both platforms; Windows prerequisites
```

**Structure Decision**: The workspace already separates domain/application (`midi-core`) from a
platform adapter (`midi-macos`) and a shell (`src-tauri`). This feature adds a second adapter in the
identical position and changes nothing about the shape. `midi-windows` mirrors `midi-macos` module for
module so that a reader who knows one knows the other; the two extra modules (`receive.rs`,
`sysex.rs`) exist because WinMM's callback restriction and manual buffer management have no CoreMIDI
counterpart (research D5).

## Key Design Decisions

### 1. How two adapters present a single type to the shell

**The problem, as it exists today**: `src-tauri/src/state.rs` names `midi_macos::CoreMidiSource`
concretely in a field type and a constructor parameter, and `src-tauri/src/lib.rs` names it again.
`AppState::sync_ports` then calls `source.sync_ports(&sources)` — an **inherent** method, not part of
the `EventSource` trait. So the shell is coupled to the concrete adapter by a method the port does not
describe. `lib.rs`'s own module docs claim "the one line that names the platform adapter"; that claim
is not currently true.

**Decision**: Move `sync_ports` into the `EventSource` trait and have the shell hold
`Arc<Mutex<dyn EventSource>>`.

`sync_ports(&mut self, sources: &[Source]) -> Vec<(SourceId, String)>` is object-safe as written — no
generics, no `Self` in return position — so this needs no change to its signature. Two other inherent
methods come along for the same reason: `dropped_events()` (which would otherwise become unreachable
behind the trait object) and the new `capabilities()`.

**Why this rather than a `cfg`-selected type alias.** A type alias — `#[cfg(target_os = "macos")] type
PlatformSource = CoreMidiSource;` — would also give the shell one name, and is less code. It was
rejected because the contract between the two adapters would then be *structural*: two inherent
methods that merely happen to have the same name and signature, checked only on the platform being
compiled. A macOS developer never compiles `midi-windows`, so a drift in `sync_ports` would be
invisible until a Windows build ran. Putting the method in the trait makes the compiler enforce on
both platforms what the shell actually depends on — which, with no test suite, is the only enforcement
available.

**Why this is not a layering regression.** It moves in the correct direction: `midi-core` gains no
dependency, the shell *loses* one, and the port now describes the whole of what the shell needs. This
is the port finally being complete, not the port being widened for convenience.

**Result**: `src-tauri/src/platform.rs` is the only file in the application containing
`cfg(target_os)`:

```rust
#[cfg(target_os = "macos")]
pub fn event_source(clock: Arc<dyn Clock>) -> Box<dyn EventSource> { … }

#[cfg(target_os = "windows")]
pub fn event_source(clock: Arc<dyn Clock>) -> Box<dyn EventSource> { … }
```

Adapter dependencies in `src-tauri/Cargo.toml` become target-gated, so the wrong platform's crate is
never even built.

### 2. How the two platform limitations reach the interface without platform conditionals

The spec requires the destination row to be present-but-inoperable on Windows with a reason (FR-031 to
FR-033), and a byte-fidelity statement to appear with the `Data` column on Windows and not on macOS
(FR-037, FR-038). Neither may be achieved by a platform check in the webview or in the shell.

**Decision**: the adapter — the one component that knows which platform it is — reports both, and
everything above simply renders what it is told.

- **The row**: `Availability::Unsupported { detail }`. The existing `Unopenable` variant is wrong for
  this, deliberately: `SourcesPanel.tsx` keeps unavailable rows tickable on purpose, so that a device
  currently held by another program can be pre-selected and picked up when released. "Unavailable
  right now" and "this platform cannot" call for different controls, so they must be different
  variants. The Windows adapter's `scan()` emits the standalone row with `Unsupported`; the macOS
  adapter continues to emit it with `Open`.
- **The column note**: `EventSource::capabilities() -> PlatformCapabilities`, carrying
  `ByteFidelity::AsTransmitted` (macOS) or `ByteFidelity::Assembled { detail }` (Windows). The
  `Monitor` holds it, the DTO carries it, `EventTable.tsx` renders the detail when present. An enum
  rather than an `Option<String>` so that "faithful" is a state the type names rather than the absence
  of a message.

Both are exhaustively matched with no catch-all arm, so a third platform, or a third fidelity level,
becomes a compile error at every render site rather than a silent default.

### 3. Threading inside the Windows adapter

Forced by the documented callback restriction (research D5): the `MidiInProc` callback may not call
multimedia functions, and returning a System Exclusive buffer requires `midiInAddBuffer`.

- The callback timestamps the arrival through the existing `Clock` port and pushes the bytes onto a
  channel. Nothing else.
- A worker thread owned by the adapter drains the channel, runs the bytes through the **unmodified**
  `midi_core::domain::decoder::MessageDecoder`, calls the `EventSink`, and re-queues SysEx buffers.
- Ordering holds because the channel is FIFO and every port feeds the same one.
- The timestamp is taken in the callback, never on the worker, so queue latency cannot leak into
  displayed times (FR-029).

This differs structurally from the macOS adapter, which decodes inside the CoreMIDI callback. The
difference is recorded in the Windows adapter's module docs as platform-imposed rather than chosen.

### 4. What is deliberately not changed

- **The decoder.** All Windows bytes — from `MIM_DATA`, `MIM_LONGDATA`, `MIM_ERROR`, `MIM_LONGERROR` —
  are fed through the existing `MessageDecoder`. Its running-status branch will never fire on Windows
  because Windows expands running status before delivery (research D1). It is left in place, correct
  and exercised on macOS. Removing it because one platform cannot reach it would be deleting a
  correct implementation to match a platform limitation.
- **The use cases.** `Monitor` gains one field and its accessor for `PlatformCapabilities`. No rule
  changes.
- **The IPC command surface.** No command is added, removed, or resignatured.
- **`Availability::Absent`.** Unreachable on Windows and left exactly as it is (research D7).

## Complexity Tracking

Four departures that require justification in writing, per the constitution's Compliance Review.

| Violation | Why needed | Simpler alternative rejected because |
|---|---|---|
| **`SourceKey` loses `Copy`** — the new `DeviceInterface(String)` variant makes the enum non-`Copy`, touching about eighteen use sites across `midi-core` and `midi-macos` with mechanical `.clone()` additions | Windows offers no integer identity. The device-interface string is the documented stable handle (research D3), and a saved selection that returns on replug (FR-017, SC-006) is a headline requirement of this feature | **Widening `Endpoint(i32)` itself**: invalidates every existing macOS settings file, a direct FR-050 violation. **Hashing the string to an `i32`**: preserves `Copy` and the wire shape, but a collision restores the wrong device's selection silently, and the persisted value becomes unreadable to a human inspecting the settings file — trading a compile-time cost for a runtime lie. **A shared opaque string for both platforms**: forces macOS to stringify a number, making both keys worse to serve neither |
| **Effect on persisted settings**: `PersistedSettings.selected_sources` may now contain a second variant shape | Same as above | Serde's externally-tagged default means old files hold `{"Endpoint": 123}` and stay valid; new Windows files hold `{"DeviceInterface": "…"}`. Nothing migrates, nothing resets. A settings file copied between platforms yields keys that match no local port — retained, matching nothing, which is exactly the spec's stated edge case |
| **Effect on the generated TypeScript bindings**: none | — | Verified, not assumed: `SourceKey` never crosses IPC (its own doc comment says so; `grep -rn SourceKey src-tauri/` returns nothing). The webview keys rows on `SourceId`, unchanged. The bindings *do* regenerate for the separate `dto.rs` availability change, which is an ordinary additive contract change |
| **`EventSource` grows three methods** (`sync_ports`, `capabilities`, `dropped_events`) and the shell moves to `Arc<Mutex<dyn EventSource>>` | Two adapters must present one type to the shell, and `sync_ports` is already a de facto part of the contract — the shell calls it today through a concrete type. The trait was incomplete, and a second implementation is what exposed it | **A `cfg`-selected type alias**: leaves the contract structural rather than compiler-checked, so drift between the adapters is invisible on the platform not being compiled. With no test suite, an unchecked cross-platform contract is an untested one |
| **`Debouncer` moves into `midi-core` as `support::debounce`** — a thread-spawning helper now lives in the core crate | Both adapters need it verbatim, and the brief requires reusing it unchanged. It is entirely platform-free: a generation counter, a sleep, and a callback | **A fourth crate for one 80-line type**: ceremony under Principle V. **Duplicating it into `midi-windows`**: two copies of a timing policy that must agree, with nothing forcing them to. **Leaving it in `midi-macos` and depending on that crate from `midi-windows`**: makes every Windows build depend on the macOS adapter, which is worse than the placement problem it solves. Dependencies still point inward — `midi-core` gains nothing; the adapters reach into it as they already do |

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| A macOS developer never compiles `midi-windows`, so breakage is invisible until a Windows build runs | Build both targets in the pipeline. The constitution allows this — pipelines build, they do not test. Until that exists, the Definition of Done for this feature requires a build on both platforms |
| WinMM truncates port names to 32 characters (research D1) | Passed through unmodified, as FR-007 requires. Recorded in the adapter's module docs so it reads as Windows' truncation rather than ours. Recovering the full name is a possible later refinement, out of scope here |
| `MIM_ERROR` does not state how many packed bytes are meaningful | Report the bytes implied by the status byte where it is recognisable; otherwise report the one byte Windows placed first and say the remainder was not determinable. This is the case FR-027 already provides for |
| `unsafe` FFI in a codebase whose safety net is the compiler | Confine every `unsafe` block to `midi-windows`, keep each as small as the call it wraps, and document the invariant that makes it sound (Principle III). No `unwrap`/`expect` in any of them |
| Two adapters' `scan()` functions drift in how they shape the standalone and spy rows | Both build the same `Vec<Source>` against the same `SourceGroupId::ALL`; the differences are exactly two `Availability` values. Reviewed side by side as part of the manual pass |

## Phase 2 Note

`/speckit-tasks` generates `tasks.md`. Per the constitution's Governance clause, the tasks template's
test-task categories are inert here and must be skipped rather than silently ignored — the same skip
recorded in the Constitution Check above.
