# Phase 0 Research: Real MIDI Input

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-27

All findings below were verified against crate documentation and published source, not recalled. Where
a claim decides the design, the source it came from is named.

---

## D1: MIDI system access — `coremidi`, not `midir`

**Decision**: Depend directly on [`coremidi`](https://crates.io/crates/coremidi) `0.9` for device access,
setup notifications, endpoint properties, and the virtual destination. Do **not** use `midir`.

**This reverses the obvious choice, so the reasoning matters.** `midir` is the popular, cross-platform
Rust MIDI crate, and the user's instruction was to use popular crates. It was the presumptive answer
and it was investigated first. Four of this feature's requirements rule it out — not because it is a
poor library, but because it is built for a different job.

`midir`'s macOS backend already wraps `coremidi`, so this is not a move away from the ecosystem: it is
using the layer `midir` itself sits on, and dropping a wrapper whose parsing layer actively removes
information this feature exists to display.

### What `midir` does provide (verified, and genuinely good)

| Capability | Finding |
|---|---|
| Port identity | `MidiInputPort::id()` returns `kMIDIPropertyUniqueID` as a string — a genuinely stable key, satisfying FR-031 |
| Virtual destination | The CoreMIDI backend implements `create_virtual()` — satisfies FR-024 |
| SysEx across packets | A `continue_sysex` flag accumulates bytes until `0xF7` — satisfies FR-015 |
| Maturity | 0.11.0 (Apr 2026), ~689k downloads, MIT |

### Why it is nonetheless disqualified

The blocker is `midir`'s per-packet decode loop, quoted from its CoreMIDI backend:

```rust
while cur_byte < pdata.len() {
    let status = pdata[cur_byte];
    if status & 0x80 == 0 {
        break;
    }
```

A byte without the high bit set — a data byte with no preceding status — ends the loop and **abandons
the rest of the packet with no callback and no error**. That single `break` breaks four requirements:

- **FR-016 (running status)** — running status *is* a data byte with no status byte. `midir` does not
  decode it; it stops at it. (An early reading of the handler suggested otherwise; reading the loop
  itself showed the opposite.)
- **FR-020 (`Invalid`)** — malformed bytes must be listed with their raw content. `midir` discards
  them silently, so they can never reach the table.
- **FR-023 (nothing dropped without a trace)** — the drop is invisible to the caller.
- **FR-014 (bytes exactly as received)** — the caller receives decoded messages, never the bytes
  `midir` chose not to decode.

The pattern behind all four: **`midir` is built for applications that consume MIDI, where discarding
garbage is correct.** A monitor is the one application where the garbage is the payload. A user opens
this window precisely because something is sending bytes that do not make sense, and a library that
tidies those away before the application sees them cannot serve that user.

- **FR-009 (hot-plug)** is a separate, independent gap: `midir` exposes no setup-change notification at
  all. That alone would be workable — poll, or add a notification client — but it removes the last
  reason to accept the parsing losses.

### Why `coremidi` clears the bar

Verified from the published source of [chris-zen/coremidi](https://github.com/chris-zen/coremidi):

```rust
// Hot-plug — FR-009
pub fn new_with_notifications<F>(name: &str, callback: F) -> Result<Client, OSStatus>
where F: Into<NotifyCallback>          // FnMut(&Notification) + Send + 'static

// Raw MIDI 1.0 bytes — FR-014
pub fn input_port<F>(&self, name: &str, callback: F) -> Result<InputPort, OSStatus>
where F: FnMut(&PacketList) + Send + 'static

// Inbound from other apps — FR-024
pub fn virtual_destination<F>(&self, name: &str, callback: F) -> Result<VirtualDestination, OSStatus>
where F: FnMut(&PacketList) + Send + 'static
```

`PacketList::iter()` yields `Packet`s exposing `data() -> &[u8]` and `timestamp()`. `Properties`
exposes `unique_id()` (`i32`, from `SInt32`), `display_name()`, `name()`, and `offline()`.

Maturity: **0.9.2, released 2 August 2026**, ~816k downloads, MIT, and already a transitive dependency
of `midir` — so choosing it promotes an existing dependency to a direct one rather than adding a new
tree. It satisfies Principle II's "actively maintained, compatible license, API surface fits the
consuming layer" on every count.

**Alternatives considered**

- **`midir`** — disqualified above. Its parser cannot report what it cannot parse.
- **`midir` + `coremidi` for notifications only** — keeps `midir`'s silent drops. Rejected: hot-plug
  was never the disqualifying gap.
- **`coremidi-hotplug-notification`** — a helper crate that wraps exactly the notification API this
  design uses directly. Rejected: a micro-crate dependency for one call, when `coremidi` is already a
  direct dependency and is far better maintained.
- **`portmidi-rs`** — a binding to a C library needing a system install. Heavier and less maintained,
  with the same consume-oriented parsing posture.

---

## D2: MIDI 1.0 `PacketList`, not MIDI 2.0 `EventList`

**Decision**: Receive through `Client::input_port` (`FnMut(&PacketList)`), not
`input_port_with_protocol` (`FnMut(&EventList, &mut T)`).

**Rationale**: FR-014 requires the bytes exactly as received. `EventList` delivers MIDI 2.0 Universal
MIDI Packets — 32-bit words. Even requesting `Protocol::Midi10`, reaching the original byte stream
means translating UMP words back into MIDI 1.0 bytes, and a monitor whose displayed bytes are a
round-trip reconstruction cannot honestly claim to show what arrived. The legacy path *is* the byte
stream.

**Cost, accepted**: only `input_port_with_protocol` carries a per-source context (`&mut T`);
`InputPort::connect_source(&source)` does not. Attribution therefore comes from **one `InputPort` per
connected source**, each closure capturing its own `SourceId`. CoreMIDI imposes no meaningful limit and
real machines have single-digit port counts, so the cost is a handful of ports for exact bytes plus
free attribution.

**Alternative considered**: one shared `InputPortWithContext` — one port, attribution built in, but
UMP translation. Rejected on FR-014.

---

## D3: The byte-stream decoder is written here, and that is a Principle II exception

**Decision**: Write a `MessageDecoder` in the domain that turns raw bytes into the existing
`MidiMessage` model. Do not adopt `midi-msg`, `wmidi`, or `midly` for decoding.

**Rationale**: This is a deliberate hand-rolling decision under a constitution that forbids
hand-rolling by default, so the justification is recorded here and will be repeated in the module docs
as Principle II requires.

Every candidate decoder is built to *reject* invalid MIDI:

- **`wmidi`** (4.0.11) — `FromBytesError` on malformed input; the bytes are an error, not a value.
- **`midi-msg`** — models the MIDI 1.0 specification for round-trip fidelity; invalid sequences are
  parse errors.
- **`midly`** — Standard MIDI *File* parsing, not a live stream. Wrong shape entirely.

This application needs the opposite: **`MidiMessage::Invalid` is a first-class outcome** (FR-020), and
so is running status (FR-016) and a SysEx transfer that is truncated (FR-018) or never completes
(FR-019). A library that returns `Err` for these gives the application a rejection where it needs a
row in a table.

The domain was already built for this. [`message.rs`](../../crates/midi-core/src/domain/message.rs)
carries the same justification for the *model* — one variant per Filter-panel checkbox, plus an
`Invalid` case "which no correct MIDI library would willingly produce". The decoder extends an argument
the project has already made and documented; it does not open a new one.

**What is genuinely reused**: everything about talking to the operating system — enumeration,
notifications, port lifecycle, properties, the virtual destination. That is the part with real
platform complexity, and none of it is hand-rolled.

**Alternative considered**: `wmidi` for well-formed messages with a hand-written fallback for the rest.
Rejected — two decoders disagreeing about where a message ends is a worse failure mode than one
decoder, and the fallback would need the same state machine anyway.

---

## D4: Hot-plug via `Client::new_with_notifications`, created on the main thread first

**Decision**: Create the notification client during Tauri's `setup`, on the main thread, **before any
other MIDI call**.

**Rationale**: Two verified constraints combine into a strict ordering requirement.

1. `coremidi`'s own notifications example states notifications "will be delivered on the run loop that
   was current when `Client` was created", and that applications built on `NSApplication` already have
   a running main run loop. Tauri's macOS backend is an `NSApplication`, and `setup` runs on the main
   thread before the event loop starts — so a client created there registers against the run loop that
   is about to run.
2. The `coremidi-hotplug-notification` README records the failure mode: macOS fixes the notification
   thread at the *first* CoreMIDI client creation, and registration fails if any MIDI work happened
   first.

**Consequence for implementation**: client creation is ordered first in `setup`, and that ordering is
load-bearing rather than incidental — it must be documented at the call site, because a later
refactor that moves a port creation above it would break hot-plug silently.

**Risk, and its fallback**: if notifications prove undeliverable under Tauri's run loop in practice,
the fallback is polling `Sources` on a timer. This is recorded as a risk in [plan.md](./plan.md) rather
than pre-emptively designed around — the adapter's interface to the rest of the application is the same
either way, so the fallback is contained to one module.

---

## D5: Two identities — a session handle and a persistence key

**Decision**: Keep `SourceId` as the opaque per-session handle that crosses IPC. Add a separate
`SourceKey` used only for persistence.

**Rationale**: These are two different jobs that a single type cannot hold once the sources are real.

- `SourceId(u32)` is the webview's key and the IPC value. It must stay a small unsigned integer
  ([`ids.rs`](../../crates/midi-core/src/domain/ids.rs) documents why `u32` and not `u64`), and it must
  be reassignable as devices come and go.
- The persistence key must survive quit, relaunch, replug into a different socket, and reboot. CoreMIDI
  supplies exactly this as `kMIDIPropertyUniqueID` — but it is an **`i32`**, it is not dense, and it is
  meaningless to the webview.

Collapsing them would either force a negative number through the IPC contract or key persistence on a
value that changes between sessions — FR-031 forbids the latter outright.

```rust
enum SourceKey {
    Endpoint(i32),          // kMIDIPropertyUniqueID
    VirtualDestination,     // fixed: our own endpoint's id is not stable across launches
}
```

`VirtualDestination` is a named variant because a virtual endpoint we create is assigned a fresh unique
id each launch unless one is set explicitly. A fixed variant is simpler than managing an id we would
then have to persist anyway, and it makes the standalone row's identity unrepresentable as anything
else.

**Alternative considered**: persist by display name. Rejected — FR-033 requires two ports sharing a
name to stay distinct, which name-keying cannot do.

---

## D6: Remembered selections outlive absent devices

**Decision**: `Monitor` retains persisted selections for ports that are not currently present, and
merges them back on save.

**Rationale**: This is a real behavioural gap in the current code, not just a new requirement.
`SourceCatalogue::selected_ids()` returns only sources *in the catalogue*, and `persisted_settings()`
saves exactly that. With a fixed simulated catalogue that was total; with real hardware, quitting while
a device is unplugged would silently erase its selection — breaking FR-033 and, more visibly, SC-007.

The fix is to carry the remembered set alongside the live catalogue and union the two when persisting.

---

## D7: Arrival time from the existing `Clock` port

**Decision**: Timestamp at ingest via the existing `Clock` port. Do not convert CoreMIDI packet
timestamps.

**Rationale**: FR-022 requires millisecond display precision and correct ordering — not host-clock
accuracy, and the spec's assumptions say so explicitly. `Packet::timestamp()` is mach absolute time
needing `mach_timebase_info` conversion, and `midir` divides its own conversion down to microseconds
anyway. Ordering is already guaranteed by the monotonic `EventId` assigned at ingest, so the packet
timestamp would add a unit conversion and a platform dependency to the domain in exchange for precision
the `HH:MM:SS.mmm` column cannot display.

The `Clock` port also stays the single place time enters the system, which
[`ports.rs`](../../crates/midi-core/src/application/ports.rs) states is the point of having it.

**Left open deliberately**: if event ordering under heavy bursts proves wrong in manual verification,
packet timestamps are the remedy. The `Clock` port is where that change would land.

---

## D8: The `EventSource` port grows a catalogue channel — and that is a real edit

**Decision**: Change the port from a fixed catalogue to a live one:

```rust
pub type CatalogueSink = Box<dyn Fn(Vec<Source>) + Send + Sync>;

pub trait EventSource: Send {
    fn start(&mut self, events: EventSink, catalogue: CatalogueSink) -> Result<(), CoreError>;
    fn stop(&mut self);
    fn catalogue(&self) -> Vec<Source>;   // initial snapshot only
}
```

**Rationale, stated plainly**: Constitution VII promised that substituting real MIDI would touch "no
domain type, no use case, and no webview code". **That promise does not survive hot-plug, and this plan
does not pretend otherwise.** `SourceCatalogue` documents itself as fixed at startup with "no add or
remove" so "callers can hold a `SourceId` without worrying that it will stop resolving"
([`source.rs`](../../crates/midi-core/src/domain/source.rs)) — FR-009 contradicts that directly.

The port seam still did most of its job: the *event* path, the filters, the log, retention, columns, and
every command handler are untouched. What the seam could not absorb is an assumption baked into the
domain — that the set of sources never changes — because that assumption was true of a simulator and is
false of hardware. This is tracked as a Constitution VII deviation in
[plan.md](./plan.md#complexity-tracking) rather than glossed as an extension.

**Pattern named** (Principle V): **Observer**. The pressure is concrete — the OS decides when the
catalogue changes, so the application cannot ask at the right moment and must be told.

**Alternative considered**: poll `catalogue()` on a timer from the Tauri layer, leaving the port
unchanged. Rejected — it puts a refresh policy in the IPC layer, which Principle IV forbids from holding
logic, and it trades a correct push for a latency/CPU tradeoff that SC-005's two-second bound would
make visible.

---

## D9: Deleting the simulator, and what goes with it

**Decision**: Delete `crates/midi-core/src/simulator/` entirely, and drop `rand` from the workspace
dependencies.

**Rationale**: FR-002 and SC-014 require that no code path can produce a fabricated event. Leaving the
module behind a `#[cfg]` would leave exactly the path SC-014 says must not exist. `rand` was introduced
solely to seed the traffic generator — the workspace `Cargo.toml` comment says so — so it leaves with
it. A dependency retained for deleted code is dead weight that Principle I's "no dead code" rule already
forbids.

**Consequence**: `catalogue::build()`'s hardcoded names (`IAC Driver Bus 1`, `MidiKeys`) disappear.
Those names will now appear only if the machine genuinely has such ports — which, for `IAC Driver Bus
1`, most macOS machines do.

---

## Summary of resolved unknowns

| Unknown | Resolution |
|---|---|
| Which MIDI crate | `coremidi` 0.9 direct (D1) |
| Byte fidelity path | `PacketList` / MIDI 1.0, one `InputPort` per source (D2) |
| Decoding malformed input | Hand-written decoder, Principle II exception recorded (D3) |
| Hot-plug mechanism | `new_with_notifications`, main thread, created first (D4) |
| Stable identity | `SourceKey::Endpoint(i32)` from `kMIDIPropertyUniqueID` (D5) |
| Absent-device selections | Remembered set merged on save (D6) |
| Timestamps | Existing `Clock` port at ingest (D7) |
| Hot-plug and the port seam | `CatalogueSink` added; Constitution VII deviation tracked (D8) |
| Simulator removal | Module and `rand` both deleted (D9) |

No `NEEDS CLARIFICATION` items remain.
