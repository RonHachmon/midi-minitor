# Phase 1 Data Model: Real MIDI Input

**Feature**: [spec.md](./spec.md) | **Research**: [research.md](./research.md) | **Date**: 2026-08-27

Types are grouped by the layer that owns them. Dependencies point inward only (Constitution IV), so
nothing below the adapter section names `coremidi`.

Legend: **new** · **changed** · *unchanged, listed for context*

---

## Domain — `crates/midi-core/src/domain/`

### `SourceKey` — **new** (`ids.rs`)

The identity that survives a quit, a replug, and a reboot. Persistence keys on this; the webview never
sees it. See [D5](./research.md#d5-two-identities--a-session-handle-and-a-persistence-key).

| Variant | Carries | Meaning |
|---|---|---|
| `Endpoint(i32)` | `kMIDIPropertyUniqueID` | A real endpoint the OS reports |
| `VirtualDestination` | — | Our own inbound endpoint, whose OS-assigned id is not stable across launches |

- Derives `Serialize`/`Deserialize` (it is persisted), plus `Clone, Copy, PartialEq, Eq, Hash`.
- Not `specta::Type`: it never crosses IPC. Keeping it off the wire is what allows `SourceId` to stay
  `u32` while the real identity is a signed integer.

### `SourceId` — *unchanged* (`ids.rs`)

Stays the opaque `u32` session handle and IPC value. What changes is only how it is minted: assigned by
the adapter as ports are discovered, rather than by a fixed catalogue. Its doc comment must be updated —
it currently promises identity "for the lifetime of the process", which remains true, but its rationale
cites the simulator's duplicate `IAC Driver Bus 1` rows and should cite real duplicate port names.

### `Source` — **changed** (`source.rs`)

| Field | Change |
|---|---|
| `id: SourceId` | unchanged |
| `name: String` | unchanged — now the OS `display_name`, used verbatim (FR-004) |
| `group: Option<SourceGroupId>` | unchanged |
| `selected: bool` | unchanged |
| `key: SourceKey` | **new** — the persistence identity |
| `availability: Availability` | **new** — see below |

### `Availability` — **new** (`source.rs`)

Why a source may not be delivering. Drives FR-008, FR-013, and the "remembered but absent" row.

| Variant | Carries | Shown as |
|---|---|---|
| `Open` | — | normal row |
| `Unopenable { detail: String }` | reason | row present, marked, with the reason (FR-013) |
| `Absent` | — | remembered from settings, not currently attached (FR-033) |

Exhaustive matching, no catch-all — a fourth state must force a compile error at every render site.

### `SourceCatalogue` — **changed** (`source.rs`)

The largest domain change, and the one Constitution VII did not anticipate
([D8](./research.md#d8-the-eventsource-port-grows-a-catalogue-channel--and-that-is-a-real-edit)).

| Method | Change |
|---|---|
| `replace(sources: Vec<Source>)` | **new** — swap the live set on a hot-plug notification, preserving `selected` for keys already known |
| `apply_selection(&[SourceKey])` | **changed** — was `&[SourceId]` |
| `selected_keys() -> Vec<SourceKey>` | **new**, replaces `selected_ids()` |
| `key_of(SourceId) -> Option<SourceKey>` | **new** |
| `event_label(SourceId)` | *unchanged* — the `From`/`To` rule still holds |
| `sources`, `name_of`, `is_selected`, `any_selected`, `set_selected`, `set_group_selected`, `group_state`, `deselected_ids` | *unchanged* |

The type doc must be rewritten: it currently states the catalogue is fixed at startup with no add or
remove, which becomes false.

### `MessageDecoder` — **new** (`decoder.rs`)

A per-source stream state machine turning raw bytes into `MidiMessage`. Pure — no I/O, no clock, no
platform. One instance per source, because running status and SysEx state are per-stream.
Justification for hand-writing it: [D3](./research.md#d3-the-byte-stream-decoder-is-written-here-and-that-is-a-principle-ii-exception).

**State**

| Field | Purpose |
|---|---|
| `running_status: Option<u8>` | The last channel-voice status byte, for FR-016 |
| `sysex: Option<SysexAccumulator>` | An in-progress transfer, for FR-015 / FR-018 / FR-019 |

**Behaviour**

`fn feed(&mut self, bytes: &[u8]) -> Vec<Decoded>` — one call per packet, returning zero or more
outcomes in arrival order.

| Outcome | Produced when | Requirement |
|---|---|---|
| `Message { message, raw }` | A complete message, raw bytes carried alongside | FR-014 |
| `Invalid { raw, reason }` | Bytes that form no valid message | FR-020 |
| `SysexTruncated { raw, true_len }` | Transfer exceeded `MAX_SYSEX_BYTES` | FR-018 |
| `SysexIncomplete { raw }` | Abandoned — flushed by `abandon()` on disconnect | FR-019 |

Rules the decoder must hold:

- A data byte with no preceding status **and** no running status is `Invalid`, not a discarded byte.
  This is the exact behaviour that disqualified `midir`.
- Real-Time status bytes (`0xF8`–`0xFF`) may interleave *inside* a SysEx transfer and must be emitted
  as their own messages without ending it.
- A non-Real-Time status byte arriving mid-SysEx ends the transfer as `SysexIncomplete` and begins the
  new message — one malformed transfer must not swallow what follows (FR-020).
- Running status is cleared by any System message, per the MIDI specification.
- `abandon()` flushes an in-progress transfer, called when a port disappears (FR-019).

### `SysexAccumulator` — **new** (`decoder.rs`)

Buffers one transfer: bytes so far, the true byte count (which keeps counting past the cap so
`SysexTruncated` can report the real size), and whether the cap was passed.

### `MidiMessage`, `MessageKind`, `MessageCategory` — *unchanged* (`message.rs`)

The whole point of D3. `MidiMessage::Invalid` already exists and is now reachable from real traffic
rather than only from generated traffic. The module doc's Principle II justification should gain a
sentence pointing at `decoder.rs` as the same argument applied to decoding.

### `MidiEvent` — *unchanged* (`event.rs`)

Still id, timestamp, source, message. Raw bytes already ride along, which is what makes FR-014
expressible without a new field.

### Constants — **changed** (`constants.rs`)

| Name | Value | Why |
|---|---|---|
| `MAX_SYSEX_BYTES` | `65_536` | CoreMIDI's own practical packet-buffer ceiling, observed in `coremidi`'s tests. Beyond this a transfer is reported truncated (FR-018) rather than growing without bound |

---

## Application — `crates/midi-core/src/application/`

### `EventSource` — **changed** (`ports.rs`)

```rust
pub type CatalogueSink = Box<dyn Fn(Vec<Source>) + Send + Sync>;

pub trait EventSource: Send {
    fn start(&mut self, events: EventSink, catalogue: CatalogueSink) -> Result<(), CoreError>;
    fn stop(&mut self);
    fn catalogue(&self) -> Vec<Source>;
}
```

`catalogue()` becomes the *initial snapshot*; `CatalogueSink` carries every later change. Observer
pattern, pressure named in [D8](./research.md#d8-the-eventsource-port-grows-a-catalogue-channel--and-that-is-a-real-edit).
The doc comment claiming the catalogue is "fixed for the lifetime of the process" must go.

### `MidiSystemStatus` — **new** (`ports.rs` or `monitor.rs`)

Separates "no devices" from "cannot see any devices" — FR-007 vs FR-008, and SC-016.

| Variant | Meaning |
|---|---|
| `Available` | The MIDI system was reached |
| `Unavailable { detail: String }` | It was not, with the reason |

### `PersistedSettings` — **changed** (`settings.rs`)

| Field | Change |
|---|---|
| `selected_sources: Vec<SourceKey>` | **changed** from `Vec<SourceId>` |
| `filter`, `columns`, `retention` | *unchanged* (FR-035) |

**Migration**: an existing settings file holds `selected_sources` as integers, which will not
deserialize into `SourceKey`. Loading must treat a malformed or old settings file as `None` — a first
run — rather than failing startup. `SettingsRepository::load` already returns
`Result<Option<_>>` with "a missing store is `None` rather than an error", so this extends an
established rule rather than inventing one. The old simulator ids carried no meaning for real hardware,
so nothing of value is lost.

### `Monitor` — **changed** (`monitor.rs`)

| Member | Change |
|---|---|
| `remembered: Vec<SourceKey>` | **new** — selections for ports not currently present ([D6](./research.md#d6-remembered-selections-outlive-absent-devices)) |
| `status: MidiSystemStatus` | **new** |
| `replace_catalogue(Vec<Source>)` | **new** — applies a hot-plug update, re-applying `remembered` to newly arrived ports (FR-012) and keeping events from departed ones (FR-011) |
| `persisted_settings()` | **changed** — unions live selections with `remembered` so an unplugged device's choice survives (FR-033) |
| `new(...)` | **changed** — takes initial status; seeds `remembered` from saved settings; first launch with no settings selects everything, otherwise unknown ports start unselected (FR-032) |
| `ingest`, `visible_events`, `set_filter`, `set_retention`, `clear`, `set_columns`, … | *unchanged* (FR-037) |

`ingest` is deliberately untouched: an event arriving from a port whose selection was just revoked is
already dropped by `is_selected`, and that logic does not care whether the port is real.

---

## Infrastructure — `crates/midi-macos/` (**new crate**)

The only place that names `coremidi`. Implements `EventSource`; depends on `midi-core`.

### `CoreMidiSource` — **new**

The adapter. Owns the CoreMIDI client, one `InputPort` per connected source
([D2](./research.md#d2-midi-10-packetlist-not-midi-20-eventlist)), the virtual destination, and one
`MessageDecoder` per source.

| Member | Purpose |
|---|---|
| `client: Client` | Created with notifications, first, on the main thread ([D4](./research.md#d4-hot-plug-via-clientnew_with_notifications-created-on-the-main-thread-first)) |
| `ports: HashMap<SourceId, InputPort>` | One per source; the closure captures its `SourceId` for attribution |
| `virtual_destination: Option<VirtualDestination>` | The standalone row (FR-024) |
| `decoders: Arc<Mutex<HashMap<SourceId, MessageDecoder>>>` | Per-stream decode state |
| `next_id: SourceId` | Session handles, minted on discovery |

### `EndpointSnapshot` — **new**

One enumerated endpoint before it becomes a domain `Source`: `unique_id: i32`, `display_name: String`,
`offline: bool`. Exists so the mapping from CoreMIDI properties to domain types has one named place
rather than being inlined into the enumeration loop.

---

## IPC — `src-tauri/src/dto.rs`

Wire types only; see [contracts/ipc-contract.md](./contracts/ipc-contract.md) for the full contract.

| Type | Change |
|---|---|
| `SourceDto` | **changed** — gains `unavailable: Option<String>` (FR-013, FR-033) |
| `SourceGroupDto` | **changed** — gains `unavailableReason: Option<String>` for the deferred spy group (FR-028) |
| `CatalogueDto` | **new** — groups plus `midiSystem`, pushed on hot-plug |
| `MidiSystemStatusDto` | **new** — `Available` / `Unavailable { detail }` |
| `EventDto`, `EventBatchDto`, `FilterViewDto`, `ColumnDto`, `SnapshotDto` | *unchanged* (FR-037) |

`EventDto` staying unchanged is the clearest evidence the port seam held where it could: real events
render through exactly the same wire type as simulated ones did.

---

## Entity relationships

```text
CoreMidiSource (infrastructure, macOS only)
  ├── Client ──── notifications ──→ CatalogueSink ──→ Monitor::replace_catalogue
  ├── InputPort per Source ──→ MessageDecoder ──→ Decoded ──→ MidiEvent ──→ EventSink
  └── VirtualDestination ────────→ MessageDecoder ──→ (same path)

Monitor (application)
  ├── SourceCatalogue ──→ Source { SourceId, SourceKey, Availability }
  ├── remembered: Vec<SourceKey> ──→ persisted, unions with live on save
  ├── EventLog ──→ MidiEvent          (unchanged)
  └── FilterSettings, ColumnVisibility (unchanged)

SourceId  → session handle, crosses IPC, reassigned freely
SourceKey → stable identity, persisted, never crosses IPC
```

---

## Requirements traceability

| Requirement | Where it lands |
|---|---|
| FR-001, FR-002 | `crates/midi-core/src/simulator/` deleted; `rand` removed ([D9](./research.md#d9-deleting-the-simulator-and-what-goes-with-it)) |
| FR-003, FR-007, FR-008 | `MidiSystemStatus` + `monitoring` + empty catalogue, distinguished in `CatalogueDto` |
| FR-004, FR-005, FR-006 | `EndpointSnapshot` → `Source`, names verbatim |
| FR-009, FR-010 | `Client::new_with_notifications` → `CatalogueSink` → `replace_catalogue` |
| FR-011 | `replace_catalogue` leaves `EventLog` untouched |
| FR-012 | `remembered` re-applied to arriving ports |
| FR-013 | `Availability::Unopenable` |
| FR-014 | `PacketList` bytes carried onto `MidiEvent` |
| FR-015, FR-018, FR-019 | `SysexAccumulator` |
| FR-016 | `MessageDecoder::running_status` |
| FR-017 | existing `MidiMessage::SystemExclusive` |
| FR-020 | `Decoded::Invalid` → `MidiMessage::Invalid` |
| FR-021 | existing `MessageKind` mapping |
| FR-022 | existing `Clock` port + `EventId` ordering ([D7](./research.md#d7-arrival-time-from-the-existing-clock-port)) |
| FR-023 | `EventPump::push` drop path must be counted and surfaced, not silent |
| FR-024–FR-027 | `VirtualDestination` |
| FR-028–FR-030 | `SourceGroupDto.unavailableReason`, spy group emitted empty |
| FR-031–FR-034 | `SourceKey`, `remembered`, `PersistedSettings` |
| FR-035–FR-038 | No control changes; new elements confined to availability states |
| FR-039, FR-040 | Crate is macOS-only; `MidiSystemStatus::Unavailable` carries a denial |
