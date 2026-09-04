# Phase 0 Research: A Send Screen With Built-In Requests and a Chosen Identity

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Date**: 2026-08-29

This is the first feature that makes the application *emit* MIDI rather than only observe it, so the
research divides in two. The outward-facing half asks what each platform can actually do, and is
answered against the vendored sources of the two crates already in the workspace — `coremidi` 0.9.2
and `windows` 0.62.2 — rather than against memory. The inward-facing half asks where the new rules
belong, and is mostly about resisting the two obvious mistakes: putting byte encoding in the webview,
and inventing a parallel set of types beside the ones the monitor already has.

Every decision records what was rejected. The rejections are the part that gets re-litigated.

---

## D1 — A second port, `Transmitter`, rather than widening `EventSource`

**Decision**: add a new application port beside `EventSource`:

```rust
pub trait Transmitter: Send {
    fn targets(&self) -> Vec<Target>;
    fn transmit(&mut self, target: TargetId, bytes: &[u8]) -> Result<(), CoreError>;
    fn publish(&mut self, name: &PublishedName) -> Result<(), CoreError>;
    fn unpublish(&mut self);
}
```

**Rationale**: `EventSource` is documented as "somewhere MIDI events come from", and its whole design
argument — the sinks, the catalogue push, the deliberate absence of any way to tell implementations
apart — is about *arrival*. Transmission has a different shape entirely: it is caller-driven, it is
synchronous, it returns a per-call result, and it has nothing to deliver to a sink. Folding it into
`EventSource` would give that trait two reasons to change, which Principle I forbids in as many words.

The port also clears the bar Principle V sets for an abstraction: it has two real implementations on
day one, macOS and Windows, and they differ substantially (CoreMIDI endpoints versus WinMM handles).
That is the identical pressure that justified `EventSource`, so the same answer applies.

**Alternatives considered**:

- *Add `transmit`/`publish` to `EventSource` and rename it `MidiAccess`.* Rejected on single
  responsibility, and because it would rewrite the documentation of the one port in the system whose
  contract is already load-bearing on both platforms — a large edit to a working seam, to save one
  trait declaration.
- *Put transmission in the Tauri shell, calling the platform crates directly.* Rejected outright:
  `src-tauri` is a translation layer, and `src-tauri/src/platform.rs` is documented as the only file
  in the application permitted to name an operating system. Sending from the shell would put
  `cfg(target_os)` back into the layer the project spent two features keeping free of it.
- *A separate `Transmitter`-only adapter crate.* Rejected by D2.

---

## D2 — One adapter object implements both ports, bundled as `MidiAccess`

**Decision**: `CoreMidiSource` and `WindowsMidiSource` each implement `EventSource` **and**
`Transmitter`. The shell holds one object, typed through a bundling supertrait:

```rust
/// The machine's MIDI access, in both directions.
pub trait MidiAccess: EventSource + Transmitter {}
impl<T: EventSource + Transmitter> MidiAccess for T {}
```

`platform::event_source` becomes `platform::midi_access`, returning `Box<dyn MidiAccess>`.

**Rationale**: this is forced by CoreMIDI, not chosen for tidiness. A `MIDIClientRef` is the parent of
every port and every virtual endpoint an application owns, and this application's client is created at
one specific moment for one specific reason — `EventSource::start`, on the main thread, because macOS
binds hot-plug notification delivery to the thread and run loop current at that instant, silently and
with no error if it is wrong. A separate transmitting object would have to either create a *second*
client, or receive the first one from the shell. The second client is an avoidable risk against a
constraint the codebase already documents as fragile; passing the client through the shell would leak
`coremidi::Client` into `src-tauri`, which is the layering violation D1 exists to prevent.

Windows has no such constraint — `midiOut*` handles are independent of `midiIn*` handles — but
honouring the stricter platform costs Windows nothing, exactly as the existing comment on start-order
already argues.

The bundle is one line and one blanket impl, and it names the pressure it answers, which is what
Principle V asks of any indirection.

**Alternatives considered**:

- *Two independent adapters, one per direction.* Rejected on the client argument above.
- *`EventSource::transmitter(&mut self) -> &mut dyn Transmitter`.* Rejected: it makes one port
  responsible for handing out the other, and the borrow it returns would conflict with the `&mut self`
  the shell needs for `sync_ports` in the same scope.
- *No bundle — store `Box<dyn EventSource>` and `Box<dyn Transmitter>` pointing at one object behind
  an `Arc<Mutex<..>>`.* Rejected: it cannot be expressed without either duplicating the object or
  reaching for trait-object downcasting, and it would describe as two collaborators something that is
  one device connection.

---

## D3 — Encoding lives in the domain, mirroring the decoder

**Decision**: a new `midi-core/src/domain/encoder.rs` provides
`encode(message: &MidiMessage) -> Result<Vec<u8>, CoreError>`. The webview never computes bytes.

**Rationale**: the spec requires that the bytes shown before sending are byte-for-byte the bytes
transmitted (US2 scenario 4, SC-004). That is a promise about two values being equal, and the cheapest
way to keep such a promise is to make them the same value: the preview *is* the encoding, computed
once in the core and carried to the screen as data. Computing it in TypeScript would create a second
implementation of the MIDI wire format whose agreement with the first nothing checks — and with no
test suite, "nothing checks" means "nobody finds out".

The domain is also where the channel offset already belongs. `constants.rs` states it outright:
channel numbers are 1-based for display, and "conversion happens once at the byte-encoding boundary so
that no display or filtering code can forget the offset." That boundary did not exist until now. This
is it.

`Result` rather than an infallible return, with `MidiMessage::Invalid` the only `Err`: a monitor's
message type must model what can be *observed*, which includes malformed data, and narrowing it to
what can be *sent* would damage the monitor to serve the sender. The composition API in D5 cannot
construct `Invalid`, so the error is unreachable in practice — and it is documented as such rather
than hidden behind an `unwrap` the constitution forbids anyway. The `match` is exhaustive with no
catch-all, so a nineteenth message type is a compile error here.

**Alternatives considered**:

- *Encode in the webview and send bytes over IPC.* Rejected on the duplication argument, and because
  it puts a rule ("what a Note On looks like on the wire") in the layer forbidden to hold rules.
- *Encode in each platform adapter.* Rejected: two implementations of one wire format, and the format
  is not platform-specific.
- *A `SendableMessage` type that structurally cannot be `Invalid`.* Rejected in D6.

---

## D4 — Raw byte entry is validated by the existing decoder

**Decision**: bytes typed by hand are parsed with `MessageDecoder::feed`. The entry is accepted only
when it yields exactly one `Decoded::Message` and leaves nothing pending; anything else — including
`Decoded::Invalid`, `SysexTruncated`, and `SysexIncomplete` — is refused with the reason the decoder
gave.

**Rationale**: FR-018 needs a judgement of "are these bytes a valid MIDI message", and the application
already contains exactly one component whose job is that judgement, written for this project, already
handling running status and truncated System Exclusive. Writing a second validator would be the
re-solving Principle II forbids, and the two would disagree at the edges — which, for a *monitor*,
means the send screen would refuse to transmit something the monitor would happily display.

It also gives the refusal message for free: the decoder's `InvalidReason` already names what is wrong
with the bytes.

Note the deliberate asymmetry recorded in the spec: the monitor displays invalid data it receives, and
the send screen refuses to transmit invalid data on purpose. Same decoder, opposite response, because
showing what arrived malformed is the point of a monitor and emitting malformed bytes is not the point
of a send screen.

**Alternatives considered**:

- *Accept any bytes and let the receiving end deal with it.* Rejected by FR-018.
- *A separate lenient parser for the entry field.* Rejected: two definitions of "valid".

---

## D5 — The composition lives in the core, not in the webview

**Decision**: the message being built is application state on a new `Sender` service. The webview
sends intents (`set_composition_kind`, `set_composition_field`, `compose_raw`) and renders the
`SendViewDto` that comes back, including the byte preview and the list of value fields.

**Rationale**: three requirements land in the same place if the composition lives in Rust and scatter
if it does not. FR-016 (values outside a message's range are prevented at entry, with the range
stated) is a domain rule — `DataByte`, `Data14`, and `ChannelNumber` already own those ranges and
already return typed errors. FR-013 (show exactly the value controls this message type carries) is a
table of which message has which fields, which is the MIDI specification, not a layout preference.
FR-019 (what you were composing survives moving between screens) is free when the state is not in a
component.

This also follows the pattern the application already uses for its most control-dense surface:
`FilterViewDto` carries the categories, the labels, and the checkbox states, and `FilterPanel.tsx`
renders what it is given rather than holding a table of message kinds. The send screen's field list is
the same idea one layer further.

The cost is an IPC round trip per value change. It is acceptable and bounded: the commands are the
same shape and cost as `set_filter`, which already runs on every checkbox click. Where a control would
otherwise fire continuously, the webview holds the in-flight text or slider position as presentation
state and commits on change — which the constitution explicitly permits, provided the webview does not
compute the bytes. It does not.

**Alternatives considered**:

- *Composition in the zustand store, bytes computed in TypeScript.* Rejected by D3 and by FR-016 —
  the webview would need its own copy of every value range.
- *Composition in the store, bytes computed by a `preview_bytes` command.* Rejected as the worst of
  both: still one IPC call per change, but now with the field table duplicated in the webview and two
  places that can disagree about what is being composed.

---

## D6 — `SendableKind` is a third message-shaped enum, and that is correct

**Decision**: add `SendableKind` — one payload-free variant per composable message, with `ALL`,
`label()`, `id()`, `fields()`, and `default_message()`. It excludes `Invalid`.

**Rationale**: the three enums answer three different questions and have three different reasons to
change, which is precisely the test Principle I applies.

| Type | Question it answers | Changes when |
|---|---|---|
| `MidiMessage` | What did we observe, with what payload? | The set of observable messages changes |
| `MessageKind` | Which filter checkbox does this fall under? | `screenshots/filters.png` changes |
| `SendableKind` | Which message can I compose, with which fields? | The set of *sendable* messages changes |

`MessageKind` is the wrong granularity for composing and its own documentation says why: it collapses
Note On and Note Off into one checkbox and Start/Stop/Continue into another, because that is what the
reference image shows. A composer needs them separate — you send a Note On, not a "Note On/Off".

The alternative that looks cleanest — a `SendableMessage` enum carrying payloads, so that
`Invalid` is structurally unrepresentable and D3's encoder becomes infallible — is rejected because it
would duplicate all eighteen payload-carrying variants of `MidiMessage`. Two payload enums for one
concept will drift, and the drift would be silent. A payload-free discriminant cannot drift in the
same way: it is a list of names, every `match` over it is exhaustive with no catch-all, and adding a
message type breaks the build in both files at once.

**Alternatives considered**:

- *Reuse `MessageKind`.* Rejected on granularity: `Note On/Off` is not a message.
- *`SendableMessage` with payloads.* Rejected on duplication, as above.
- *No enum — a `const DEFAULTS: [MidiMessage; 18]` array driving the picker.* Rejected: the id and
  label for each entry would then be produced by matching on `MidiMessage` anyway, which is the same
  code without the type that makes it exhaustive.

---

## D7 — Targets are a snapshot, re-read on the device-change notification that already exists

**Decision**: `Transmitter::targets()` returns the set of targets at the moment of asking, exactly as
`EventSource::catalogue()` does. The shell re-reads it inside the catalogue callback it already
registers, updates the `Sender`, and pushes the result to the webview.

**Rationale**: one physical event — a cable moved — changes both lists, and both platforms report it
through a single notification the application is already subscribed to. Adding a second sink to the
port would mean a second subscription to the same underlying event, and would force the adapters to
decide which of the two lists a given notification affects. Re-reading a snapshot is not a decision, so
it does not offend the rule that the shell holds no logic; it is the same move `AppState::sync_ports`
already makes when the catalogue changes.

**Alternatives considered**:

- *A `TargetSink` beside `CatalogueSink`.* Rejected: two callbacks for one notification, and the
  adapters gain a classification job they cannot do reliably.
- *Widen `CatalogueSink` to carry both lists.* Rejected: it changes the signature of a working port on
  both platforms to avoid one `targets()` call in the shell.
- *Let the webview refresh targets when its catalogue subscription fires.* Rejected: it puts a refresh
  policy in the webview, and leaves the send screen stale for any client that is not subscribed to the
  Sources catalogue.

---

## D8 — Publication support is a capability the adapter reports, never a platform check

**Decision**: `PlatformCapabilities` gains a second field:

```rust
pub enum PublicationSupport {
    Supported,
    Unsupported { detail: String },
}
```

macOS returns `Supported`. Windows returns `Unsupported` with prose naming the loopback-utility route.

**Rationale**: this is the existing mechanism, used for the reason it was built. `PlatformCapabilities`
was introduced so that "every platform difference a user can observe reaches them as *data* the
adapter reported, never as a conditional someone wrote further up", and `src-tauri/src/platform.rs`
documents itself as the only file in the application allowed to name an operating system. Publication
support is exactly such a difference, and the enum mirrors `ByteFidelity` — including the argument for
why it is an enum rather than an `Option<String>`: "this platform can do it" is a fact worth naming,
not the absence of a complaint.

The detail string is the adapter's own words, following the precedent set by
`Availability::Unsupported` and `MidiSystemStatus::Unavailable`: the domain holds no table of
per-platform prose.

Read once at construction, like the rest of `PlatformCapabilities` — it describes the machine, and the
machine does not change while the application runs.

**Alternatives considered**:

- *`cfg(target_os)` in the send screen's command handlers.* Rejected by the layering rule, explicitly
  and by name.
- *A boolean `can_publish`.* Rejected: it cannot carry the reason or the workaround, and FR-027
  requires both.
- *Let `publish()` simply fail on Windows.* Rejected: it would make the control operable-but-inert,
  which the spec forbids in FR-027 and SC-007. The user must be told before they try.

---

## D9 — Windows transmits through WinMM `midiOut*`, with no new dependency

**Decision**: `midi-windows` gains an output path using the `windows` crate bindings already in the
manifest.

Verified present in `windows` 0.62.2 under the `Win32_Media_Audio` feature the crate already enables:
`midiOutGetNumDevs`, `midiOutGetDevCapsW`, `midiOutOpen`, `midiOutShortMsg`, `midiOutLongMsg`,
`midiOutPrepareHeader`, `midiOutUnprepareHeader`, `midiOutReset`, `midiOutClose`.

Shape of the implementation:

- **Enumeration** — `midiOutGetNumDevs`, then `midiOutGetDevCapsW` per index for the name.
- **Short messages** — `midiOutShortMsg` with the bytes packed into the low three bytes of a `u32`.
- **System Exclusive** — `midiOutPrepareHeader` / `midiOutLongMsg` / `midiOutUnprepareHeader` over a
  `MIDIHDR`, the same structure the receive path in `sysex.rs` already handles.
- **Handle lifetime** — the handle for the chosen target is opened on first use and closed when the
  chosen target changes or the application exits, rather than opened per send. Opening a MIDI output
  is slow enough to be visible and can fail when another program holds the device; doing it once per
  send would turn a working target into an intermittent one.

**The MIDI Mapper is excluded.** `midiOutOpen` accepts the pseudo-device `MIDI_MAPPER` (`-1`), which
is not a device but a system routing setting. Listing it would put an entry in the target list that
does not correspond to anything the user can point at, and its behaviour depends on a control panel
this application does not own.

**Alternatives considered**:

- *`midir` for output.* Rejected for the third time in this project, and for a new reason as well as
  the old one. The old reason still stands (its Windows backend is unusable for a monitor, per
  003/D1); the new one is that adopting it for output alone would put two MIDI stacks in one process,
  each with its own device handles and its own view of the machine.
- *Windows MIDI Services / the WinRT `MidiOutPort` API.* Rejected: it raises the floor above the
  Windows 10 1809 baseline this project supports, and buys nothing for output, which WinMM does
  completely.
- *Opening the output handle per send.* Rejected on the latency and contention argument above.

---

## D10 — macOS transmits through one output port, and publishes with `MIDIReceived`

**Decision**: `midi-macos` gains an `OutputPort` created alongside the input ports, and a
`VirtualSource` created on demand when publishing is switched on.

Verified in the vendored `coremidi` 0.9.2 sources:

- `Client::output_port(name) -> Result<OutputPort, OSStatus>`
- `OutputPort::send(&Destination, packets)` where `packets: Into<Packets>`
- `Client::virtual_source(name) -> Result<VirtualSource, OSStatus>`
- `VirtualSource::received(packets)` — wraps `MIDIReceived`, distributing to every input port
  connected to that source
- `Destinations` / `Destination::from_index` / `Destinations::count()` for enumeration
- `PacketBuffer::new(timestamp, data)`, with `impl From<&PacketBuffer> for Packets`

Sending uses timestamp `0`, which CoreMIDI defines as "now" — this application sends when the user
presses send, and has no scheduling model to express anything else.

**The two directions are genuinely different calls, and that is the point.** Sending to a destination
is `OutputPort::send`; sending *as* the published source is `VirtualSource::received`. The port hides
that behind one `transmit(target, bytes)` because the distinction is a platform mechanism, not
something the user chose — the user chose a target, and the adapter knows which mechanism that target
requires.

Publishing is symmetric with the virtual *destination* the adapter already creates for
`Act as a destination for other programs`, and reuses that code's error handling shape:
`CoreError::PortUnavailable` with the endpoint name and the OS status.

**One consequence to expect rather than fix**: the published source is a real endpoint on the machine,
so this application's own Sources list will show it, like any other device. That is correct — it is
how a user watches their own sends — and it is why the spec calls out telling two same-named entries
apart rather than hiding one.

**Alternatives considered**:

- *`MIDISendSysex` for System Exclusive.* Rejected: `OutputPort::send` handles a `PacketBuffer` of any
  length, and the crate does not expose the asynchronous variant. One path for all messages is
  simpler and has no size cliff worth special-casing.
- *A second CoreMIDI client for output.* Rejected in D2.

---

## D11 — A saved request's name is its identity

**Decision**: saved requests are keyed by their name, which must be unique across the whole library —
built-ins included. There is no request id.

**Rationale**: this is 004's decision, applied again for the same reasons. That feature made a prefix
the identity of a data rule because the clash rule already guaranteed uniqueness, and recorded the
payoff: deletion needs no index, no id, and no fragile positional contract, and a stale webview asking
to delete something already gone gets a typed error instead of deleting the wrong row. Uniqueness here
is a rule the user benefits from anyway — two saved requests called "Fader up" would be a defect in the
list, not a feature — so the same shape falls out.

Uniqueness against built-in names too, so that a saved request can never shadow one and make FR-011
("the built-in library is always available in full") quietly false.

**Alternatives considered**:

- *A `RequestId` newtype over a counter.* Rejected: it adds a concept the user never sees, and needs
  its own persistence and collision story, to solve a problem the uniqueness rule already solves.
- *Positional index.* Rejected: it breaks the moment two windows or a stale view disagree about the
  list.

---

## D12 — Settings gain an additive field; no migration is needed

**Decision**: `PersistedSettings` gains one `#[serde(default)]` field, `send`, holding the chosen
target key, the publication name and state, and the saved requests. Documents written by features 001
through 004 deserialize unchanged.

**Rationale**: 004 needed a real migration because it *changed the shape* of an existing field, and the
repository treats an unreadable document as absent — so a naive change would have silently discarded
source selections, columns, and retention along with the filter. This feature adds a field and changes
none, so `#[serde(default)]` is the whole of it. The distinction is worth stating explicitly because
the previous feature's migration machinery is right there and it would be easy to reach for it out of
habit.

**Alternatives considered**:

- *A separate store file for send settings.* Rejected: two files to keep consistent, two failure modes,
  and one `SettingsRepository` port that already exists and already works.
- *Persisting the send records.* Rejected: the spec scopes the record to the session (FR-030), and a
  record of what was sent to a device that may no longer exist has no value on a later launch.

---

## D13 — `CataloguePump` is generalised rather than copied

**Decision**: rename `CataloguePump` to `PushPump<T>` — the body is a `Mutex<Option<Channel<T>>>` with
`subscribe` and `send` — and instantiate it twice:
`type CataloguePump = PushPump<CatalogueDto>` and `type TargetPump = PushPump<TargetsDto>`.

**Rationale**: the target list needs exactly what the catalogue needs — a latest-wins push to the
webview, no batching, no coalescing, on a human-scale event. Copying twenty lines to get a second one
would be duplication that Principle I rules out, and the type parameter is the only difference between
the two.

`EventPump` stays as it is. Its existing documentation explains why it is separate — it batches on a
frame timer for a stream running at hundreds of events per second — and none of that applies here.

**Alternatives considered**:

- *A second hand-written `TargetPump`.* Rejected on duplication.
- *Generalise `EventPump` too.* Rejected: batching, dropped-event counting, and the flush thread are
  its whole substance, and none of it is shared.

---

## D14 — One message per request; a multi-channel panic is out of scope

**Decision**: every request — built-in or saved — is exactly one MIDI message. The built-in the spec
calls "all notes off / panic" is `All Notes Off`: control change 123, value 0, on the chosen channel.

**Rationale**: FR-007 names the capability and the spec's Out of Scope section rules out sequences
("no scheduled or looped sending beyond a repeat of a single request"). A true panic as most
applications implement it is All Sound Off plus All Notes Off plus Reset All Controllers across all
sixteen channels — forty-eight messages, which is a sequence by any definition. Building a sequence
concept to satisfy one built-in would be the speculative machinery Principle VII warns about.

One `All Notes Off` on the channel the user is sending on does release the note the edge case is
about, which is what the requirement actually needs.

**Alternatives considered**:

- *A `Sequence` request kind holding several messages.* Rejected as above. It is a clean extension
  later if it is ever asked for, and nothing in this design blocks it.
- *Silently sending three messages behind one button.* Rejected: it breaks the promise that the byte
  preview shows what will be sent.

---

## D15 — The built-in library, fixed

Fifteen entries, satisfying FR-007. Channel-bearing entries default to channel 1; every value is
adjustable per FR-009.

| Name | Message | Default bytes |
|---|---|---|
| `Note On` | Note On, note 60, velocity 100 | `90 3C 64` |
| `Note Off` | Note Off, note 60, velocity 0 | `80 3C 00` |
| `All Notes Off` | Control 123, value 0 | `B0 7B 00` |
| `Control Change` | Control 7, value 100 | `B0 07 64` |
| `Program Change` | Program 0 | `C0 00` |
| `Pitch Bend` | Pitch Wheel, 8192 (centre) | `E0 00 40` |
| `Channel Pressure` | Channel Pressure 64 | `D0 40` |
| `Aftertouch (Poly)` | Aftertouch, note 60, pressure 64 | `A0 3C 40` |
| `Start` | Start | `FA` |
| `Stop` | Stop | `FC` |
| `Continue` | Continue | `FB` |
| `Clock` | Clock | `F8` |
| `Active Sense` | Active Sense | `FE` |
| `Reset` | Reset | `FF` |
| `Identity Request` | System Exclusive | `F0 7E 7F 06 01 F7` |

`Identity Request` is the Universal Non-Real Time identity request with device `7F` (all devices) —
the one built-in that reliably provokes a reply, which arrives in the monitor as ordinary traffic
because this feature waits for nothing.

Names follow the reference vocabulary where the reference has one: `Aftertouch (Poly)`,
`Active Sense`, and `Reset` are copied from `screenshots/filters.png` rather than reworded.

---

## D16 — No new dependency

Every capability this feature needs is already in the workspace: `coremidi` and `windows` for the two
platforms, `serde` for the new settings field, `specta`/`tauri-specta` for the generated contract,
`zustand` for the webview's presentation state. Nothing was found that a crate would do better, and
nothing here is a solved problem being re-solved — the one thing that could have been (validating
typed bytes) is answered by reusing this project's own decoder in D4.
