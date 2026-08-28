# Phase 1 Data Model: Windows Support

**Feature**: `003-windows-support` | **Date**: 2026-08-28

This feature adds no new domain concept. It adds **three variants and one value type**, each because
a platform difference exists that the current types cannot represent. Everything else in the model is
untouched.

The test applied to each addition: *can the existing types already say this truthfully?* Where the
answer was yes, nothing was added. Where the answer was "only by overloading a variant that means
something else", a variant was added — because overloading is how a type stops being a description of
reality.

---

## 1. `SourceKey` — a second kind of persistent identity

**Location**: `crates/midi-core/src/domain/ids.rs`

```text
SourceKey
├── Endpoint(i32)               UNCHANGED — macOS, CoreMIDI's signed unique-id property
├── DeviceInterface(String)     NEW — Windows, the device-interface string
└── VirtualDestination          UNCHANGED — the endpoint this application publishes
```

**Why a variant and not a widened `Endpoint`**: settled in [research.md](./research.md) D3. Briefly:
existing macOS settings files must keep loading (FR-050), each platform's key should say what it
actually is, and exhaustive matching turns the migration into a list the compiler produces.

**Derive change**: `Copy` is dropped; `Clone` is added. `PartialEq`, `Eq`, `Hash`, `Debug`,
`Serialize`, `Deserialize` are unchanged.

**Reach of the derive change** — about eighteen sites, every one flagged by the compiler:

| File | What changes |
|---|---|
| `domain/source.rs` | `Source.key` field; `SourceCatalogue::replace` previous-selection vector; `key_of`; `apply_selection`; `selected_keys`; `present_keys` — clones where the value was copied |
| `application/monitor.rs` | `remembered: Vec<SourceKey>`; `select_by_key` takes it by reference or by value with a clone |
| `application/settings.rs` | `PersistedSettings.selected_sources` — no code change, type is already owned |
| `midi-macos/endpoints.rs` | one construction site |
| `midi-macos/source.rs` | `IdMinter`'s `HashMap<SourceKey, SourceId>` and `id_for`; the `VirtualDestination` comparisons in `sync_ports` and `scan` |

**Invariants**:
- A key is stable across quit, replug into a different socket, and reboot. This is the whole reason
  the type exists and is what FR-017 and SC-006 test.
- A key never crosses the IPC boundary. Verified: `SourceKey` does not appear anywhere under
  `src-tauri/`. The webview keys rows on `SourceId`.
- Two ports sharing a display name have different keys. Where Windows returns an empty interface
  string, no key can be formed from it and the name-matching fallback of FR-020 applies.

---

## 2. `Availability` — a fourth state: the platform cannot do this at all

**Location**: `crates/midi-core/src/domain/source.rs`

```text
Availability
├── Open                          UNCHANGED — attached and listening
├── Unopenable { detail }         UNCHANGED — attached, but something else holds it
├── Absent                        UNCHANGED — remembered, not currently attached
└── Unsupported { detail }        NEW — this platform cannot offer this capability at all
```

**Why this cannot reuse `Unopenable`**: the two call for different *controls*, not just different
text. `SourcesPanel.tsx` deliberately keeps an `Unopenable` row tickable, so a user can pre-select a
device another program is currently holding and have it picked up the moment it is released. A row
that the platform can never satisfy must not behave that way — ticking it would be the "operable but
inert" control the spec forbids (FR-042). Collapsing the two would make the panel unable to tell which
kind of unavailability it is rendering.

**Why not a boolean beside the existing field**: `unavailable: Option<String>` plus
`selectable: bool` can disagree — "selectable and unsupported" is representable and meaningless. A
fourth variant makes that state impossible rather than merely wrong.

**State transitions**:

| From | To | Trigger |
|---|---|---|
| `Open` | `Unopenable` | a port open attempt fails during `sync_ports` |
| `Unopenable` | `Open` | the holding program releases the port; the next `sync_ports` succeeds (FR-016) |
| `Open` / `Unopenable` | `Absent` | macOS only — the endpoint reports itself offline |
| any | *row disappears* | Windows — the device stops being enumerated (research D7) |
| `Unsupported` | — | terminal. It is a property of the platform, not of the moment |

**`Availability::reason()`** gains its fourth arm, returning the `Unsupported` detail. No catch-all.

**Who constructs `Unsupported`**: the Windows adapter only, and only for the
`Act as a destination for other programs` row. Nothing in `midi-core` constructs it — the domain does
not know what platform it is on, and must not learn.

---

## 3. `PlatformCapabilities` and `ByteFidelity` — how a platform describes its own limits

**Location**: `crates/midi-core/src/application/ports.rs`, beside `MidiSystemStatus`, which solves the
adjacent problem and establishes the shape.

```text
PlatformCapabilities
└── byte_fidelity: ByteFidelity

ByteFidelity
├── AsTransmitted              the bytes delivered are the bytes the cable carried (macOS)
└── Assembled { detail }       the platform assembles messages before delivery (Windows)
```

**The problem this solves**: the spec's Decision 2 requires a standing statement attached to the
`Data` column on Windows and absent on macOS (FR-037, FR-038), and forbids the webview from deciding
this for itself. The adapter is the only component that knows which platform it is; so the adapter
says, and everything above renders what it is told.

**Why an enum rather than `Option<String>`**: "this platform is faithful" is a fact worth naming. As
an `Option`, faithfulness is the absence of a message — indistinguishable from a platform that simply
forgot to describe itself. As an enum with no catch-all arm, a third fidelity level becomes a compile
error at every site that renders one.

**Why `detail` is a `String` supplied by the adapter**: house style, already established by
`Availability::Unopenable { detail }` and `MidiSystemStatus::Unavailable { detail }`. The platform
explains itself in its own words; the domain does not hold a table of per-platform prose.

**Where it lives at runtime**: `Monitor` holds one, set at construction from
`EventSource::capabilities()`, and exposes it read-only. It is settings-adjacent but is **not**
persisted — it is a property of the machine the application is running on right now, and restoring
yesterday's copy onto a different machine would be a lie of exactly the kind this feature exists to
prevent.

---

## 4. Entities from the spec, mapped to existing types

The spec names six key entities. Five already exist and are unchanged in shape:

| Spec entity | Existing type | Change |
|---|---|---|
| MIDI Port | `domain::source::Source` | `key` field's type gains a variant |
| Port Availability | `domain::source::Availability` | gains `Unsupported` |
| MIDI System Access | `application::ports::MidiSystemStatus` | none |
| Received Message | `domain::event::MidiEvent` | none — `raw: Vec<u8>` already means "the bytes as received" on any platform |
| Saved Selection | `application::settings::PersistedSettings.selected_sources` | none in shape; entries may hold the new variant |
| **Platform Capability** | **`PlatformCapabilities` + `Availability::Unsupported`** | **new** — the entity the spec introduced, realised as one value type plus one variant rather than as two special cases |

`Incomplete Transfer` from feature 002 is unchanged: the Windows adapter tracks in-progress System
Exclusive per port through the same `MessageDecoder`, and flushes it on port close through the same
`abandon` path.

---

## 5. Wire contract change (`src-tauri/src/dto.rs`)

The only shape that crosses IPC and changes:

```text
SourceDto.unavailable: Option<String>        BEFORE
SourceDto.availability: AvailabilityDto      AFTER — tagged union
    { type: "open" }
    { type: "unopenable", data: { detail } }
    { type: "absent" }
    { type: "unsupported", data: { detail } }

CatalogueDto.byteFidelity: ByteFidelityDto   NEW — tagged union
    { type: "asTransmitted" }
    { type: "assembled", data: { detail } }
```

**Why a tagged union rather than keeping the loose optional string**: the webview must now distinguish
"unavailable but still tickable" from "unavailable and not tickable", and two loose fields can
contradict each other. The codebase already uses this shape — `midiSystem.type === "unavailable"` in
`SourcesPanel.tsx` — so this is the house pattern, not a new one.

TypeScript types are regenerated from the Rust declarations; `src/bindings.ts` is never hand-edited.
See [contracts/](./contracts/) for both contracts in full.
