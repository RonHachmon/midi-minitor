# Contract: The `Transmitter` Port

**Feature**: [../spec.md](../spec.md) | **Plan**: [../plan.md](../plan.md) | **Date**: 2026-08-29

The seam between the application layer and the machine's MIDI output. `midi-macos` and
`midi-windows` implement it; nothing above it names a platform.

This document is the contract both adapters must satisfy. Where the two platforms differ, the
difference is stated here as an obligation on the adapter — never as something a caller is expected
to branch on.

---

## Declaration

```rust
/// Somewhere MIDI messages go.
pub trait Transmitter: Send {
    fn targets(&self) -> Vec<Target>;
    fn transmit(&mut self, target: TargetId, bytes: &[u8]) -> Result<(), CoreError>;
    fn publish(&mut self, name: &PublishedName) -> Result<(), CoreError>;
    fn unpublish(&mut self);
}

/// The machine's MIDI access, in both directions.
pub trait MidiAccess: EventSource + Transmitter {}
impl<T: EventSource + Transmitter> MidiAccess for T {}
```

`EventSource` is unchanged. The bundle exists because on macOS both directions must share one
CoreMIDI client, and the shell must therefore hold one object — see
[research D2](../research.md#d2--one-adapter-object-implements-both-ports-bundled-as-midiaccess).

---

## `targets()`

Returns the set of targets available **at the moment of asking**. It is a snapshot and promises
nothing about later validity, for the same reason `EventSource::catalogue` is a snapshot: devices are
attached and removed while the application runs.

**Obligations**

1. Every MIDI destination the operating system reports appears, with the name the OS gives it, used
   verbatim. Cleaning up a device name would make this list disagree with every other MIDI utility on
   the machine.
2. The published source appears as a target — with `kind: PublishedSource` and the user's chosen
   name — **if and only if** it is currently published. It is not a device; it is this application's
   own endpoint, and it can only receive what it is publishing.
3. Each target carries a `TargetKey` that survives a replug, so a chosen target is recognised on the
   next launch and after a cable is moved (FR-023).
4. On a platform where publishing is unsupported, no `PublishedSource` target is ever returned.
5. The MIDI Mapper pseudo-device is **not** listed on Windows. It is a system routing setting rather
   than a device, and it does not correspond to anything the user can point at.

**When the list changes.** Callers re-read this on the device-change notification they are already
subscribed to through `EventSource::start`'s `CatalogueSink`. One physical event changes both lists,
and adding a second sink for the same notification was rejected in
[research D7](../research.md#d7--targets-are-a-snapshot-re-read-on-the-device-change-notification-that-already-exists).

---

## `transmit(target, bytes)`

Sends `bytes` to `target`, exactly as given.

**Obligations**

1. **The bytes are transmitted unaltered** — same values, same order, nothing inserted, nothing
   assembled, nothing normalised. This is the whole promise of SC-004, and it is the adapter's to
   keep. In particular, an adapter must not re-derive a status byte, must not expand or apply running
   status, and must not split or merge messages.
2. **Whole or not at all** (FR-035). A partial transmission is reported as `Err`, never as `Ok`. An
   adapter that cannot make this guarantee for a given size must fail rather than send a prefix.
3. **The two target kinds are one call.** Reaching a `Destination` and distributing from the
   `PublishedSource` are different platform mechanisms; choosing between them is the adapter's job,
   because the user chose a target, not a mechanism.
4. **Synchronous.** Returns after the platform has accepted the bytes. Callers report the outcome to
   the user, so a fire-and-forget signature would make FR-028 unkeepable.
5. **Idempotent in the sense that matters**: calling it twice sends twice. There is no deduplication,
   because two identical `Clock` messages are two clock ticks.

**Errors**

| Error | Condition | What the caller does |
|---|---|---|
| `UnknownTarget { name }` | the id is not in the current snapshot — typically unplugged since | Reports it, and refreshes the target list |
| `TransmitFailed { target, detail }` | the platform refused or the device could not be opened | Records a failed send carrying `detail` |

A failure must leave the adapter usable: a later `transmit` to a different target, or to the same one
after it returns, must work without restarting anything.

---

## `publish(name)` / `unpublish()`

Publishes a MIDI source under `name`, which other programs on the machine list among their MIDI inputs
and can receive from. `unpublish` removes it.

**Obligations**

1. **`name` is the name other programs see**, used verbatim.
2. **Idempotent.** Publishing while already published under the same name is `Ok` and changes
   nothing. `unpublish` on an unpublished source is a no-op — hence no `Result`, because there is no
   way for it to fail that a caller could act on.
3. **A rename is unpublish-then-publish.** The adapter is not asked to rename an endpoint in place;
   the receiving program sees a device disappear and a new one appear, which is exactly what the user
   was warned about (FR-026).
4. **Publishing does not disturb reception.** Input ports stay open, the catalogue is unaffected, and
   no event is lost (FR-005). On macOS this follows from sharing one client; an adapter must not
   restart its client to publish.
5. **On a platform that cannot publish**, `publish` returns `PublicationUnsupported` carrying the same
   detail the adapter reports through `PublicationSupport::Unsupported`. Callers are expected never to
   reach it — the control is not operable — so this is a backstop against a stale webview, not the
   mechanism by which the user is informed.

**Expected consequence, not a defect**: on a platform that supports publishing, the published source
is a real endpoint, so it appears in this application's *own* source catalogue like any other device.
That is how a user watches their own sends, and the spec's edge case about two same-named entries
exists because of it.

**Errors**

| Error | Condition |
|---|---|
| `PublicationUnsupported { detail }` | the platform cannot publish a source at all |
| `PublicationFailed { detail }` | the platform can, but this attempt did not succeed |

---

## `PlatformCapabilities::publication`

```rust
pub enum PublicationSupport {
    Supported,
    Unsupported { detail: String },
}
```

Read once at construction with the rest of `PlatformCapabilities`, and carried to the screen as data.

**Obligations**

1. The adapter states its own limitation. No layer above may test the platform (`src-tauri/src/platform.rs`
   remains the only file in the application that names an operating system).
2. `detail` states **what cannot be done and what the user can do instead**, in the adapter's own
   words — the pattern `Availability::Unsupported` and `MidiSystemStatus::Unavailable` already set.
3. `Supported` is a named fact, not the absence of a complaint. Matched exhaustively with no
   catch-all, so a third level of support is a compile error at every site that renders one.

**Windows** returns `Unsupported`, with detail to the effect that Windows offers no way for an
application to publish a MIDI source without installing a system-wide driver, and that a MIDI loopback
utility's port appears in the target list like any other destination. This is the same limitation the
application already reports for `Act as a destination for other programs`, and the wording should read
as a sibling of it.

**macOS** returns `Supported`.

---

## What the port deliberately does not expose

- **No flag describing which implementation is running.** `EventSource` is documented as offering no
  way for a caller to distinguish one implementation from another, and that property is what kept the
  whole event path unchanged when generated traffic was replaced by hardware. The same rule applies
  here.
- **No scheduling.** No timestamps, no send-at, no queue. The user presses send and the message goes;
  sequencing is out of scope, and a scheduling parameter nobody has asked for is the speculative
  abstraction Principle V rejects.
- **No fan-out.** One target per call. Sending to several destinations at once is an assumption the
  spec explicitly did not make.
- **No delivery confirmation.** `Ok` means the platform accepted the bytes, and nothing more. Whether
  a receiving program acted on them is unknowable from here, which is why FR-028 forbids the interface
  from claiming otherwise.
