# Phase 1 Data Model: A Send Screen With Built-In Requests and a Chosen Identity

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Date**: 2026-08-29

Every type below is new except where marked *changed*. Nothing existing is renamed or removed, with
one exception noted in [Changed types](#changed-types): `platform::event_source` becomes
`platform::midi_access`, because it now returns something that transmits as well as receives.

Layer placement follows the constitution: domain types know nothing of Tauri or of a platform, the
application service orchestrates them, the adapters implement the port, and the DTOs exist only to
cross the IPC boundary.

---

## Domain — `crates/midi-core/src/domain/`

### `encoder.rs` (new module)

```rust
pub fn encode(message: &MidiMessage) -> Result<Vec<u8>, CoreError>;
```

The mirror of `decoder.rs`, and the boundary `constants.rs` has always referred to: the one place
where a 1-based display channel becomes a 0-based wire nibble.

Total over all eighteen sendable variants of `MidiMessage`; returns
`CoreError::UnsendableMessage` for `Invalid` alone. The `match` carries no catch-all arm, so a new
message type is a compile error here as well as in the decoder.

**Why the error is documented as unreachable rather than removed**: the composition API cannot
construct `MidiMessage::Invalid`, so no caller in this application can reach the `Err`. The variant
still has to be handled, because `MidiMessage` models what a *monitor* can observe — and narrowing the
monitor's message type to what a sender can emit would damage the older feature to serve the newer
one. See [research D3](./research.md#d3--encoding-lives-in-the-domain-mirroring-the-decoder).

### `sendable.rs` (new module)

```rust
pub enum SendableKind {
    NoteOn, NoteOff, AftertouchPoly, Control, Program, ChannelPressure, PitchWheel,
    TimeCode, SongPositionPointer, SongSelect, TuneRequest,
    Clock, Start, Stop, Continue, ActiveSense, Reset, SystemExclusive,
}

impl SendableKind {
    pub const ALL: [Self; 18];
    pub const fn label(self) -> &'static str;
    pub const fn id(self) -> &'static str;
    pub fn fields(self) -> &'static [FieldSpec];
    pub fn default_message(self) -> MidiMessage;
}

pub struct FieldSpec {
    pub id: FieldId,
    pub label: &'static str,
    pub min: u16,
    pub max: u16,
}

pub enum FieldId { Channel, Note, Velocity, Controller, Value, Program, Pressure, Bend, Position, Song, Data }
```

One payload-free variant per composable message. Deliberately *not* `MessageKind`, which groups
`Note On/Off` and `Start/Stop/Continue` behind single filter checkboxes — the right granularity for
the Filter panel and the wrong one for a composer. Deliberately *not* a second payload-carrying enum
either; the reasoning and the rejected alternatives are in
[research D6](./research.md#d6--sendablekind-is-a-third-message-shaped-enum-and-that-is-correct).

`FieldSpec` is what makes FR-013 structural: the screen renders the fields it is given and holds no
table of which message carries which values. `min`/`max` come from the existing range constants
(`CHANNEL_RANGE`, `MAX_DATA_BYTE`, `MAX_DATA_14`), never from literals repeated here.

`FieldId::Data` is the System Exclusive payload, which is bytes rather than a bounded number and is
therefore the one field the screen renders as a byte entry rather than a numeric control.

### `composition.rs` (new module)

```rust
pub enum Composition {
    /// Built from named controls. The message is the authority; bytes are derived.
    Guided { message: MidiMessage },
    /// Typed as bytes and validated by the decoder. The bytes are the authority.
    Raw { bytes: Vec<u8>, message: MidiMessage },
}

impl Composition {
    pub fn from_kind(kind: SendableKind) -> Self;
    pub fn parse_raw(entry: &str) -> Result<Self, CoreError>;
    pub fn bytes(&self) -> Result<Vec<u8>, CoreError>;
    pub fn fields(&self) -> &'static [FieldSpec];
    pub fn set_field(&mut self, field: FieldId, value: u16) -> Result<(), CoreError>;
}
```

**Why two variants rather than one struct holding both a message and bytes**: because which of the two
is authoritative differs between them, and a single struct would leave that unstated. This is the same
distinction `MidiEvent` already documents for received data — interpretation is lossy, so the bytes
that arrived are kept and are the authority for the raw view. Here the direction is reversed but the
asymmetry is identical: a hand-typed `90 3C 00` must be transmitted as `90 3C 00`, not re-encoded into
the `80` status a `Note On` with velocity zero would otherwise produce.

`fields()` returns an empty slice for `Raw`, so there is nothing to edit and no error to invent;
`set_field` on a `Raw` composition returns `CoreError::UnknownField`, which only a stale webview can
provoke.

`parse_raw` accepts an entry only when `MessageDecoder::feed` yields exactly one `Decoded::Message`
and leaves nothing pending — see [research D4](./research.md#d4--raw-byte-entry-is-validated-by-the-existing-decoder).

### `target.rs` (new module)

```rust
pub struct Target {
    pub id: TargetId,
    pub key: TargetKey,
    pub name: String,
    pub kind: TargetKind,
}

pub enum TargetKind {
    /// A MIDI destination the operating system reports.
    Destination,
    /// This application's own published source.
    PublishedSource,
}
```

The direct counterpart of `Source`, and split the same way for the same reason: `TargetId` is session
identity, because two destinations may share a display name; `TargetKey` is the identity that outlives
the session, so a chosen target is recognised after a replug (FR-023).

`TargetKind` exists because the two are not interchangeable to the adapter — one is reached with an
output port, the other by distributing from a virtual source — and because the screen must say which
kind is in force, since only one of them carries the chosen name (FR-024).

Note what `Target` does **not** carry: an `Availability`. A destination that has gone is simply absent
from the next snapshot, and a failed send reports the reason at the moment it fails (FR-028). The
Sources panel needs `Availability` because a *remembered* selection must stay visible while absent;
the send screen shows one chosen target, and telling the user it is gone is the send's job.

### `request.rs` (new module)

```rust
pub struct Request {
    pub name: RequestName,
    pub description: String,
    pub composition: Composition,
    pub origin: RequestOrigin,
}

pub enum RequestOrigin { BuiltIn, Saved }

pub struct RequestLibrary { saved: Vec<Request> }

impl RequestLibrary {
    pub const BUILT_IN: [BuiltInRequest; 15];
    pub fn all(&self) -> impl Iterator<Item = &Request>;
    pub fn get(&self, name: &RequestName) -> Result<&Request, CoreError>;
    pub fn save(&mut self, name: RequestName, composition: Composition) -> Result<(), CoreError>;
    pub fn rename(&mut self, from: &RequestName, to: RequestName) -> Result<(), CoreError>;
    pub fn delete(&mut self, name: &RequestName) -> Result<(), CoreError>;
}
```

A request's name is its identity — unique across saved requests *and* built-ins, so a saved entry can
never shadow a built-in and quietly falsify FR-011. `save`, `rename`, and `delete` all refuse rather
than overwrite, returning `DuplicateRequestName`, `UnknownRequest`, or `BuiltInRequestImmutable`, and
mutate nothing when they refuse. This is 004's prefix-as-identity decision applied again; see
[research D11](./research.md#d11--a-saved-requests-name-is-its-identity).

The fifteen built-ins are listed in
[research D15](./research.md#d15--the-built-in-library-fixed).

### `publication.rs` (new module)

```rust
pub struct Publication {
    pub name: PublishedName,
    pub published: bool,
}
```

The disguise, as state. Both fields persist (FR-025, FR-026), and `published` is what the adapter is
asked to make true — the source is republished under the saved name at launch.

### `send_record.rs` (new module)

```rust
pub struct SendRecord {
    pub id: SendRecordId,
    pub time: Timestamp,
    pub target: String,
    pub message: MidiMessage,
    pub bytes: Vec<u8>,
    pub outcome: SendOutcome,
}

pub enum SendOutcome {
    Sent,
    Failed { detail: String },
}
```

`target` is the name as it was at the time of the send, copied rather than referenced: the record must
still read correctly after the device is unplugged, which is precisely when someone goes back to read
it.

`SendOutcome` is an enum rather than an `Option<String>` for the reason `ByteFidelity` already
records — "it worked" is a fact worth naming, not the absence of a complaint — and because FR-028
requires a successful send and a failed one to be visibly different, not merely differently annotated.

Capped at `SEND_RECORD_LIMIT` (200), oldest evicted first. Session-only: not persisted
([research D12](./research.md#d12--settings-gain-an-additive-field-no-migration-is-needed)).

### `ids.rs` (changed — additions only)

```rust
pub struct TargetId(u32);
pub enum TargetKey { Endpoint(i32), DeviceName(String), PublishedSource }
pub struct SendRecordId(u32);
pub struct RequestName(String);   // trimmed, non-empty, <= MAX_REQUEST_NAME_LEN
pub struct PublishedName(String); // trimmed, non-empty, <= MAX_PUBLISHED_NAME_LEN
```

`TargetKey` mirrors `SourceKey`'s three-way split for the same platform reasons: CoreMIDI unique ids
are stable integers, WinMM output devices have no stable id and must be keyed by name, and the
published source is a fixed identity this application owns.

`RequestName::parse` and `PublishedName::parse` return `Result`, following `HexPrefix::parse` — the
established shape for "a string the user typed that has rules".

---

## Application — `crates/midi-core/src/application/`

### `sender.rs` (new module)

```rust
pub struct Sender { /* targets, chosen, publication, library, records, capabilities */ }

impl Sender {
    pub fn new(targets: Vec<Target>, saved: Option<&PersistedSettings>, capabilities: PlatformCapabilities) -> Self;

    pub fn replace_targets(&mut self, targets: Vec<Target>);
    pub fn set_target(&mut self, id: TargetId) -> Result<(), CoreError>;
    pub fn chosen_target(&self) -> Option<&Target>;

    pub fn set_publication_name(&mut self, name: PublishedName);
    pub fn set_published(&mut self, published: bool) -> Result<(), CoreError>;

    pub fn select_request(&mut self, name: &RequestName) -> Result<(), CoreError>;
    pub fn set_composition_kind(&mut self, kind: SendableKind);
    pub fn set_composition_field(&mut self, field: FieldId, value: u16) -> Result<(), CoreError>;
    pub fn compose_raw(&mut self, entry: &str) -> Result<(), CoreError>;

    pub fn outgoing(&self) -> Result<Outgoing, CoreError>;   // what to send, and where
    pub fn record(&mut self, outgoing: &Outgoing, outcome: SendOutcome, at: Timestamp);
    pub fn recorded(&self, id: SendRecordId) -> Result<&SendRecord, CoreError>;

    pub fn library_mut(&mut self) -> &mut RequestLibrary;
    pub fn persisted_settings(&self) -> SendSettings;
}
```

The counterpart of `Monitor`, and one service for the same reason `Monitor` is one service: the
chosen target, the publication, the composition, and the record of what was sent all read or write
each other. Sending needs the target *and* the composition; recording needs the target's name *and*
the outcome; changing the target must not silently invalidate a composition. Splitting them would push
the coordination into the Tauri layer, which is forbidden to hold it.

**`outgoing()` / `record()` is a deliberate two-step, and it is the existing pattern.** `Sender` does
not hold the `Transmitter` — exactly as `Monitor` does not hold the `EventSource`. It decides *what*
should be sent and *where*; the shell performs the transmission through the port and feeds the outcome
back. `AppState::sync_ports` already has this shape: read state from the core, call the adapter, return
the failures to the core. No new pattern is introduced.

```rust
pub struct Outgoing {
    pub target: TargetId,
    pub target_name: String,
    pub message: MidiMessage,
    pub bytes: Vec<u8>,
}
```

`outgoing()` returns `Err(CoreError::NoSendTarget)` when nothing is chosen (US1 scenario 4), and
`Err(CoreError::UnknownTarget)` when the chosen target is no longer in the list.

### `settings.rs` (changed)

```rust
pub struct PersistedSettings {
    pub selected_sources: Vec<SourceKey>,
    pub filter: FilterSettings,
    pub columns: ColumnVisibility,
    pub retention: RetentionLimit,
    #[serde(default)]
    pub send: SendSettings,          // NEW
}

pub struct SendSettings {
    pub target: Option<TargetKey>,
    pub publication: Publication,
    pub saved_requests: Vec<SavedRequest>,
}
```

Additive only. Documents written by features 001–004 deserialize unchanged, so no migration is needed
and none is written — see [contracts/persisted-settings.md](./contracts/persisted-settings.md).

### `ports.rs` (changed — additions only)

```rust
pub trait Transmitter: Send { /* targets, transmit, publish, unpublish */ }
pub trait MidiAccess: EventSource + Transmitter {}
impl<T: EventSource + Transmitter> MidiAccess for T {}

pub struct PlatformCapabilities {
    pub byte_fidelity: ByteFidelity,
    pub publication: PublicationSupport,   // NEW
}

pub enum PublicationSupport {
    Supported,
    Unsupported { detail: String },
}
```

Full contract in [contracts/transmitter-port.md](./contracts/transmitter-port.md). `EventSource` is
unchanged.

### `error.rs` (changed — additions only)

Sixteen `CoreError` variants, each carrying what its message must quote:

| Variant | Raised when | Requirement |
|---|---|---|
| `UnsendableMessage` | `encode` is given `MidiMessage::Invalid` | D3; unreachable via the composition API |
| `MalformedSendBytes { detail }` | raw entry is not exactly one valid message | FR-018 |
| `ValueOutOfRange { field, min, max }` | a field is set outside its range | FR-016 |
| `NoSendTarget` | a send is attempted with nothing chosen | US1 sc. 4 |
| `UnknownTarget { name }` | the chosen target is no longer present | FR-021 |
| `TransmitFailed { target, detail }` | the adapter could not transmit | FR-028 |
| `PublicationUnsupported { detail }` | publishing attempted where the platform cannot | FR-027 |
| `PublicationFailed { detail }` | the platform could publish but did not | US3 |
| `MalformedPublicationName` | the name is empty after trimming, or too long | FR-024 |
| `UnknownRequest { name }` | a request name is not in the library | FR-039 |
| `DuplicateRequestName { name }` | a saved name collides with any existing name | D11 |
| `BuiltInRequestImmutable { name }` | rename or delete of a built-in | FR-011 |
| `MalformedRequestName` | the name is empty after trimming, or too long | FR-037 |
| `UnknownField` | `set_field` names a field the composition does not carry | stale view only |
| `UnknownSendableKind` | a kind id is not one of the eighteen | stale view only |
| `UnknownSendRecord` | a re-send names a record evicted past the cap | FR-031 |

The last three are provokable only by a webview whose model is stale, which is why they are shown in
the shared banner rather than at a control — the split
[contracts/ipc-commands.md](./contracts/ipc-commands.md) records.

### `constants.rs` (changed — additions only)

```rust
pub const SEND_RECORD_LIMIT: usize = 200;
pub const MAX_REQUEST_NAME_LEN: usize = 64;
pub const MAX_PUBLISHED_NAME_LEN: usize = 64;
pub const DEFAULT_PUBLISHED_NAME: &str = "MIDI Monitor";
pub const IDENTITY_REQUEST: [u8; 6] = [0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7];
```

`DEFAULT_PUBLISHED_NAME` matches the name the application already publishes its virtual *destination*
under. Other programs list sources and destinations separately, so one product name in both lists
reads correctly rather than ambiguously.

---

## Infrastructure — the two adapters

Both implement `Transmitter` on their existing type, so both automatically satisfy `MidiAccess`.

| | `midi-macos` | `midi-windows` |
|---|---|---|
| Enumerate | `Destinations` iterator | `midiOutGetNumDevs` + `midiOutGetDevCapsW` |
| Key | `TargetKey::Endpoint(unique id)` | `TargetKey::DeviceName(name)` |
| Short send | `OutputPort::send` with a `PacketBuffer` | `midiOutShortMsg` |
| SysEx send | the same call — no size cliff | `midiOutPrepareHeader` / `midiOutLongMsg` / `midiOutUnprepareHeader` |
| Publish | `Client::virtual_source` + `VirtualSource::received` | `PublicationSupport::Unsupported` |
| New state | `output_port`, `virtual_source` | `out_handle`, `open_target` |

Neither adapter gains a dependency. Details and the verification behind each call are in
[research D9](./research.md#d9--windows-transmits-through-winmm-midiout-with-no-new-dependency) and
[D10](./research.md#d10--macos-transmits-through-one-output-port-and-publishes-with-midireceived).

---

## IPC surface — `src-tauri/src/dto.rs`

One view model, mirroring how `FilterViewDto` already drives the Filter panel: the screen renders what
it is given and holds no MIDI knowledge of its own.

```text
SendViewDto
├── targets: Vec<TargetDto>            { id, name, kind }
├── chosen_target: Option<u32>
├── publication: PublicationDto        { name, published, support: PublicationSupportDto }
├── composition: CompositionDto
│   ├── kind: Option<String>           // absent for a raw composition
│   ├── kinds: Vec<SendableKindDto>    { id, label }
│   ├── fields: Vec<FieldDto>          { id, label, min, max, value }
│   └── preview: String                // "90 3C 64" — spaced uppercase hex
├── requests: Vec<RequestDto>          { name, description, preview, origin }
└── records: Vec<SendRecordDto>        { id, time, target, message, data, outcome }
```

`TargetsDto` — the targets list alone — is what the target pump pushes when devices change; the full
`SendViewDto` is returned by every mutating command, following the existing
`SnapshotDto`/`MutationResultDto` convention.

Byte formatting (`90 3C 64`) happens here, not in the domain: the domain owns the bytes, and this layer
owns how they are shown, exactly as it already does for timestamps and the `Data` column.

---

## Changed types

| Type | Change | Why |
|---|---|---|
| `PlatformCapabilities` | `+ publication: PublicationSupport` | FR-027, via the existing capability mechanism (D8) |
| `PersistedSettings` | `+ send: SendSettings`, `#[serde(default)]` | FR-023, FR-025, FR-026, FR-040 |
| `platform::event_source` | renamed to `platform::midi_access`, returns `Box<dyn MidiAccess>` | It now returns something that transmits too; the old name would be a lie |
| `AppState` | `+ sender: Mutex<Sender>`, `+ target_pump` | The send side needs the same managed-state treatment as the monitor |
| `CataloguePump` | generalised to `PushPump<T>`; `CataloguePump` becomes an alias | A second identical pump would be duplication (D13) |
| `CoreError` / `IpcError` | 16 variants each, plus `From` arms | Errors cross as typed variants, never strings |

Nothing is removed. No existing type changes shape in a way that alters its meaning, and no depicted
control is touched.
