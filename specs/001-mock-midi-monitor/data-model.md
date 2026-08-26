# Phase 1 Data Model: Mock MIDI Monitor

**Date**: 2026-08-26 · **Plan**: [plan.md](./plan.md) · **Spec**: [spec.md](./spec.md)

Types below live in `crates/midi-core`. They are the domain vocabulary; the wire shapes that cross
IPC are derived from them and specified in [contracts/ipc-contract.md](./contracts/ipc-contract.md).
Rust shown is illustrative of shape and invariants, not final code.

Naming follows Constitution I: no abbreviation outside the domain's own vocabulary (`midi`, `sysex`,
`cc`). Every type here is `pub` and therefore carries rustdoc explaining *why* (Constitution III).

## Newtypes

Primitive obsession is what lets a channel number reach a source-id parameter. Each of these is a
**Newtype** (pattern named per Constitution V) whose constructor is the only place its invariant is
checked.

| Type | Wraps | Invariant | Constructed by |
|------|-------|-----------|----------------|
| `SourceId` | `u32` | Opaque; unique per source; never displayed | Source catalogue at startup |
| `ChannelNumber` | `u8` | `1..=16` — the MIDI channel range (FR-032) | `ChannelNumber::new(u8) -> Result<Self, CoreError>` |
| `RetentionLimit` | `usize` | `1..=100_000` (D-04) | `RetentionLimit::new(usize) -> Result<Self, CoreError>` |
| `HexPrefix` | `String` | Uppercase hex nibbles only, length ≥ 1 | `HexPrefix::parse(&str) -> Result<Self, CoreError>` |
| `Timestamp` | `u64` millis since midnight local | `< 86_400_000` | Clock port |

`ChannelNumber` stores the **1-based** value the user sees, not the 0-based wire nibble. Conversion
happens once, at the byte-decoding boundary, so no display code can forget the offset.

## Core entities

### `MidiEvent`

One observed message (spec: *MIDI Event*).

```text
pub struct MidiEvent {
    pub id: EventId,               // monotonic; stable React key and snapshot cursor
    pub timestamp: Timestamp,
    pub source: SourceId,
    pub message: MidiMessage,
    pub raw: RawBytes,             // ArrayVec<u8, 8> for channel/system; Vec<u8> for sysex
}
```

- `raw` is the authority for the hex-prefix filter (FR-035) and for the raw-bytes view (FR-015).
- Channel and Data column values are **derived** from `message`, never stored twice. Storing a
  pre-rendered display string would be two sources of truth for one fact (Constitution I).

### `MidiMessage`

The domain enum described in [research.md](./research.md) D-05. Its variants are exactly the leaf
entries of `screenshots/filters.png`, which is what makes every filter checkbox mechanically
mappable to a variant.

```text
pub enum MidiMessage {
    NoteOn        { channel: ChannelNumber, note: NoteNumber, velocity: u7 },
    NoteOff       { channel: ChannelNumber, note: NoteNumber, velocity: u7 },
    AftertouchPoly{ channel: ChannelNumber, note: NoteNumber, pressure: u7 },
    Control       { channel: ChannelNumber, controller: u7, value: u7 },
    Program       { channel: ChannelNumber, program: u7 },
    ChannelPressure { channel: ChannelNumber, pressure: u7 },
    PitchWheel    { channel: ChannelNumber, value: u14 },
    TimeCode      { data: u7 },
    SongPositionPointer { position: u14 },
    SongSelect    { song: u7 },
    TuneRequest,
    Clock, Start, Stop, Continue, ActiveSense, Reset,
    SystemExclusive { bytes: Vec<u8> },
    Invalid       { reason: InvalidReason },
}
```

Two mappings hang off this enum, both exhaustive `match` with **no catch-all arm** — so adding a
variant later forces the compiler to point at every site (Constitution VII):

- `fn kind(&self) -> MessageKind` — the filter identity.
- `fn channel(&self) -> Option<ChannelNumber>` — `None` for System Common, Real Time, SysEx, and
  Invalid. This `Option` is what makes FR-012 (blank Chan cell) and FR-033 (One Channel excludes
  channel-less messages) unavoidable rather than forgettable.

`Start`, `Stop`, and `Continue` are separate variants but share one filter entry, because the
screenshot shows a single `Start/Stop/Continue` checkbox. The mapping lives in `MessageKind`, not in
the UI.

### `MessageKind` and `MessageCategory`

`MessageKind` is the filter's unit — one variant per checkbox in the Filter panel; `MessageCategory`
is its parent column heading.

| Category | Kinds (label verbatim from `screenshots/filters.png`) |
|----------|-------------------------------------------------------|
| `VoiceMessages` | `Note On/Off`, `Aftertouch (Poly)`, `Control`, `Program`, `Channel Pressure`, `Pitch Wheel` |
| `SystemCommon` | `Time Code`, `Song Position Pointer`, `Song Select`, `Tune Request` |
| `RealTime` | `Clock`, `Start/Stop/Continue`, `Active Sense`, `Reset` |
| *(standalone)* | `System Exclusive`, `Invalid` |

`Note On` and `Note Off` are distinct `MidiMessage` variants but one `MessageKind`
(`Note On/Off`) — again because the screenshot shows one checkbox. The display name in the Message
column stays per-variant (`Note On` / `Note Off`), matching `screenshots/main-screen.png`.

**Validation rule**: the set of `MessageKind` values and their category assignment is fixed by the
screenshot. A kind may not be added or renamed without a Constitution VI review.

### `Source` and `SourceGroup`

```text
pub struct Source { pub id: SourceId, pub name: String, pub group: SourceGroupId, pub selected: bool }
pub struct SourceGroup { pub id: SourceGroupId, pub name: String, pub expanded: bool }
```

The catalogue is fixed at startup (spec Assumptions — no hot-plug) and mirrors
`screenshots/sources.png` exactly:

| Group | Entries |
|-------|---------|
| `MIDI sources` | `IAC Driver Bus 1`, `MidiKeys` |
| *(standalone, no group)* | `Act as a destination for other programs` |
| `Spy on output to destinations` | `IAC Driver Bus 1` |

Note that `IAC Driver Bus 1` appears in two groups: they are two distinct `SourceId`s with the same
display name, which is the spec's "two sources sharing a display name" edge case made concrete.
Identity is the id; the name is presentation only.

### `CheckState`

```text
pub enum CheckState { Checked, Unchecked, Mixed }
```

Derived, never stored, for group rows and filter parents: all children checked → `Checked`, none →
`Unchecked`, otherwise → `Mixed` (FR-025, FR-031). Deriving rather than storing removes the entire
class of "parent says checked, children disagree" bugs.

### `FilterSettings`

```text
pub struct FilterSettings {
    pub kinds: EnumSet<MessageKind>,     // which checkboxes are ticked
    pub channel_mode: ChannelMode,
    pub data_filter: DataPrefixFilter,
}

pub enum ChannelMode { AllChannels, OneChannel(ChannelNumber) }
```

`ChannelMode` is a sum type rather than a `bool` plus a number, so "One Channel selected but no valid
channel" is unrepresentable — the compiler enforces what the radio pair implies (FR-032).
The channel number entered while `One Channel` is active is retained when switching back to
`All Channels` (FR-033 acceptance 7); that retained value lives in the UI's presentation state,
not the domain — the domain only knows the active mode.

### `DataPrefixFilter`

```text
pub struct DataPrefixFilter { pub mode: PrefixMode, pub prefixes: Vec<HexPrefix> }
pub enum PrefixMode { Include, Exclude }
```

**Matching rule** (FR-037–FR-039): render `event.raw` as an uppercase nibble string with no
separators (`[0x90, 0x3C, 0x7F]` → `"903C7F"`), then test `starts_with` against each prefix.
`HexPrefix::parse` normalises the user's entry by uppercasing and stripping whitespace, which is what
makes `b0 07`, `B007`, and `B0 07` equivalent (FR-038) and what makes an odd-length prefix match on
a leading nibble for free (FR-039) — matching on nibbles rather than bytes means the odd case needs
no special handling at all.

**Empty rule** (FR-040): `prefixes.is_empty()` admits everything in *both* modes. Stated explicitly
because the naive Include reading would hide every event.

**Invalid entry** (FR-041): `parse` returns `Err`; the last valid filter stays in effect. Invalid
text never reaches the domain.

### `EventLog`

The bounded ring buffer (spec: *Event Log*).

```text
pub struct EventLog { entries: VecDeque<MidiEvent>, limit: RetentionLimit }
```

| Operation | Behaviour | Requirement |
|-----------|-----------|-------------|
| `push` | Append; evict from the front while `len > limit` | FR-017 |
| `set_limit` | Store, then evict immediately | FR-018 |
| `clear` | Drop all entries | FR-020 |
| `retain_sources` | Drop entries whose source is deselected | FR-026 |
| `view(&FilterSettings)` | Iterate entries passing the filter, oldest first | FR-034 |

Holds events from **selected sources only**; message-type, channel, and prefix filters are a view
over it, never an admission gate (D-04). Order is insertion order, which is arrival order, which is
the display order — oldest at the top (FR-014).

### `ColumnVisibility`

```text
pub enum Column { Time, Source, Message, Chan, Data }   // declaration order == display order
pub struct ColumnVisibility { hidden: EnumSet<Column> }
```

Display order is the enum's declaration order, so a hidden column reappears in its original position
(FR-044) without storing positions anywhere. `hide` refuses when it would empty the set (FR-045),
returning `Err(CoreError::LastColumnVisible)` — enforced in the core so the UI cannot bypass it.
Visibility never reaches `EventLog::view`, which is how FR-046 (display-only) holds structurally.

## Ports (application layer)

Traits the application defines and the outside world implements — **Ports & Adapters**, named per
Constitution V. The problem each solves is the reason it exists:

| Port | Problem it solves | Implementations |
|------|-------------------|-----------------|
| `EventSource` | Lets simulated traffic be replaced by real MIDI input without touching domain or use-case code — the one seam Constitution VII mandates | `simulator::SimulatedSource` (now); a real MIDI adapter (later) |
| `SettingsRepository` | Keeps `tauri-plugin-store` and `serde_json::Value` out of the Tauri-free core | `src-tauri/settings.rs` |
| `Clock` | Lets timestamps come from somewhere other than the wall clock | System clock |

```text
pub trait EventSource {
    fn start(&mut self, sink: Box<dyn Fn(MidiEvent) + Send>) -> Result<(), CoreError>;
    fn stop(&mut self);
    fn catalogue(&self) -> Vec<Source>;
}
```

Nothing in this trait, or in any type it mentions, indicates the data is simulated — no `is_mock`
flag, no `Fake` in a name (Constitution VII). `SimulatedSource` is the only module that knows.

## `Monitor` — the application service

Owns `EventLog`, `FilterSettings`, source selection, and `ColumnVisibility`; the single place where
a settings change and an incoming event meet.

| Use case | Effect |
|----------|--------|
| `ingest(event)` | Admit if source selected → `log.push` → return whether it passes the filter (only passing events are streamed) |
| `snapshot()` | The full filtered view — called after any settings change (FR-034) |
| `set_sources(selection)` | Update selection, `log.retain_sources`, snapshot |
| `set_filter(settings)` | Replace filter, snapshot |
| `set_retention(limit)` | `log.set_limit`, snapshot |
| `clear()` | `log.clear` |
| `set_columns(visibility)` | Update visibility (no snapshot — display only) |

## Errors

Typed errors as values (Constitution I); every variant documents its trigger and the caller's
recourse (Constitution III).

```text
pub enum CoreError {
    ChannelOutOfRange { value: u8 },        // 1..=16 required
    RetentionOutOfRange { value: usize },   // 1..=100_000 required
    MalformedHexPrefix { entry: String },   // non-hex character present
    EmptyHexPrefix,
    LastColumnVisible,                      // refuse to hide the final column
    UnknownSource { id: SourceId },
}
```

`SettingsError` (persistence) and `IpcError` (boundary) are separate enums; `IpcError` is the only
one that crosses to TypeScript, as a discriminated union (see the contract).

## State transitions

Only one state machine exists, and it is deliberately small:

```text
Idle ──start()──▶ Streaming ──stop()──▶ Idle
```

`Streaming` with **zero selected sources** or **zero admitted message kinds** is a valid state, not
an error — it is what FR-027 and the "all categories unchecked" edge case describe. The UI must
distinguish it from "no traffic" by reading the selection, so the emptiness is explained rather than
ambiguous.

No other entity has a lifecycle worth modelling as a machine; introducing one would be the ceremony
Constitution V rejects.
