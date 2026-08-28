# Contract: The `EventSource` Port

**Feature**: `003-windows-support` | **Location**: `crates/midi-core/src/application/ports.rs`

This is the contract two adapters must satisfy identically. It is the one seam the project was
designed around, and this feature is the first time it carries a second implementation — which is
what exposed the fact that the contract was incomplete.

---

## What changes

Three methods move **into** the trait. None of them is new behaviour; all three exist today as
inherent methods on `CoreMidiSource`, and the shell already calls one of them.

```text
trait EventSource: Send {
    fn start(&mut self, events: EventSink, catalogue: CatalogueSink) -> Result<(), CoreError>;   unchanged
    fn stop(&mut self);                                                                          unchanged
    fn catalogue(&self) -> Vec<Source>;                                                          unchanged

    fn sync_ports(&mut self, sources: &[Source]) -> Vec<(SourceId, String)>;                     MOVED IN
    fn capabilities(&self) -> PlatformCapabilities;                                              NEW
    fn dropped_events(&self) -> u32;                                                             MOVED IN
}
```

All six are object-safe — no generics, no `Self` in return position — so the shell can hold
`Arc<Mutex<dyn EventSource>>`. `EventSource: Send` already, so the trait object is `Send`, the
`Mutex` is `Send + Sync`, and the `Arc` satisfies Tauri's managed-state bounds.

### Why `sync_ports` belongs here

`src-tauri/src/state.rs` calls `source.sync_ports(&sources)` today through the concrete
`CoreMidiSource` type. So the shell already depends on it; the port simply failed to say so. A second
adapter cannot be substituted while that dependency is invisible to the compiler.

The alternative — a `cfg`-selected type alias, leaving `sync_ports` inherent on each adapter — was
rejected in [plan.md](../plan.md): it makes the cross-adapter contract structural rather than
compiler-checked, and a macOS developer never compiles the Windows adapter, so drift would be
invisible until a Windows build ran.

### Why `dropped_events` comes along

It exists on `CoreMidiSource` today and would become unreachable behind a trait object. Both adapters
genuinely have the number — both can lose events under pressure and both must be able to admit it
(FR-030). Moving it preserves an existing capability rather than adding a speculative one.

### Why `capabilities` is new

The spec requires each platform to state what it cannot do, on the control it affects, with no
platform conditional above the adapter (FR-037, FR-038, FR-042 to FR-044). The adapter is the only
component that knows its platform. This is the method through which it says so.

---

## Method contracts

### `start(events, catalogue) -> Result<(), CoreError>`

Unchanged. Begins producing events; `catalogue` fires whenever the set of available sources changes.

**Errors**: `CoreError::MidiSystemUnavailable` when the MIDI system cannot be reached at all. A
*single* port failing to open is **not** an error here — it is reported per-source through
`Availability::Unopenable`, because the application must keep monitoring everything else.

**Platform notes**:
- *macOS*: creates the process's first CoreMIDI client. Must run on the main thread before any other
  MIDI work, or hot-plug silently never fires. Documented at the call site.
- *Windows*: registers for device-interface notifications via `CM_Register_Notification` and starts
  the adapter's worker thread. **No equivalent thread constraint** — `CM_Register_Notification`
  requires no window handle and binds to no run loop.

### `stop()`

Unchanged. Idempotent. Closes every open port, releases the published destination where one exists,
and abandons any System Exclusive transfer still arriving so that an interrupted dump is *reported*
rather than stranded (FR-026).

### `catalogue() -> Vec<Source>`

Unchanged. A **snapshot**, never a cacheable list — devices come and go while the application runs.
Every later change arrives through the `CatalogueSink`.

Both adapters return, in this order: every MIDI input port the system reports, under
`SourceGroupId::MidiSources`; then the standalone `Act as a destination for other programs` row; and
nothing at all under `SourceGroupId::SpyOnOutput`.

| Row | macOS | Windows |
|---|---|---|
| Input ports | `Availability::Open`, or `Absent` when the system reports the endpoint offline | `Availability::Open`. `Absent` is never constructed — a removed device stops being enumerated (research D7) |
| `Act as a destination…` | `Availability::Open` — the row is operable and publishes an endpoint | `Availability::Unsupported { detail }` — present, visible, not operable, with its reason |
| `Spy on output to destinations` | empty group, unchanged | empty group, unchanged |

### `sync_ports(sources) -> Vec<(SourceId, String)>`

Opens and closes ports so the listening set matches the selections in `sources`. Returns the sources
that could **not** be opened, each paired with the reason.

**Why failures are returned rather than raised**: a device held by another program is a fact the user
needs to see in the Sources panel next to the row it concerns. Returning it as an error would imply
the whole operation failed when every other device connected successfully.

**Required behaviour on both platforms**:
- Ports selected and not open → open them; failures are collected, not propagated.
- Ports open and no longer selected, or whose device has gone → close them and flush any transfer
  still arriving.
- The standalone destination row follows its own checkbox, separately from port opening.
- Idempotent: calling it twice with the same `sources` changes nothing and returns the same failures.

**Windows note**: because Windows grants MIDI input ports exclusively, `Unopenable` is a routine state
rather than a rarity. FR-016 requires that a port which becomes free later starts being monitored
without the user re-ticking it — satisfied because `sync_ports` runs again on every catalogue change,
and a release is a device-interface event.

### `capabilities() -> PlatformCapabilities`

Describes what this platform's MIDI access can and cannot do. Read once at construction and held by
`Monitor`.

| | macOS | Windows |
|---|---|---|
| `byte_fidelity` | `ByteFidelity::AsTransmitted` | `ByteFidelity::Assembled { detail }` |

The Windows `detail` states that Windows delivers short messages already assembled and that System
Exclusive transfers are exact — the substance FR-037 requires. It is prose supplied by the adapter,
following the house pattern already set by `Availability::Unopenable { detail }` and
`MidiSystemStatus::Unavailable { detail }`.

**Not persisted.** It describes the machine the application is running on right now.

### `dropped_events() -> u32`

How many events were lost because the adapter could not keep pace or could not reach its own shared
state. Surfaced rather than kept internal: a monitor that silently loses traffic is worse than one
that admits it fell behind (FR-030).

---

## What does *not* change

- **`EventSink` and `CatalogueSink`** — same signatures, same threading contract: called from the
  source's own thread, must be cheap, must not block.
- **`MidiSystemStatus`** — unchanged. Windows maps a failure to reach the MIDI system onto
  `Unavailable { detail }` exactly as macOS does.
- **`SettingsRepository`, `Clock`** — unchanged.
- **The IPC command surface** — no command added, removed, or resignatured.
- **`MessageDecoder`** — not part of this port and not touched. Both adapters feed it raw bytes and
  publish what it returns.

---

## Conformance checklist for a new adapter

An adapter satisfies this contract when all of the following hold. Both `midi-macos` and
`midi-windows` are checked against it during the manual verification pass.

- [ ] Every event delivered originates from the operating system. Nothing is generated (FR-004).
- [ ] Every source listed corresponds to a port the system reports, or to a row the reference layout
      depicts. No padding, no substitutes (FR-005).
- [ ] Port names are passed through exactly as the system supplies them (FR-007).
- [ ] Two ports sharing a display name get distinct `SourceId`s and distinct `SourceKey`s (FR-008).
- [ ] `start` returns `MidiSystemUnavailable` only when the MIDI system itself is unreachable, never
      for a single failed port (FR-010, FR-015).
- [ ] Device arrival and removal reach the `CatalogueSink` without polling from above (FR-011).
- [ ] Raw bytes are delivered exactly as received from the operating system — nothing added, removed,
      reordered, or re-encoded (FR-022).
- [ ] A message split across deliveries is reassembled into one event (FR-023).
- [ ] System Exclusive is one event with its true byte count; oversize is marked truncated; an
      incomplete transfer is reported rather than withheld (FR-024 to FR-026).
- [ ] Bytes forming no valid message reach the `Invalid` category with whatever bytes the system
      supplied (FR-027).
- [ ] Each event carries its arrival time, taken as close to arrival as the platform allows (FR-029).
- [ ] `capabilities()` describes this platform truthfully, including what it cannot do.
- [ ] No `unwrap`, `expect`, or `panic` outside `main`; every FFI result is matched.
- [ ] Every `unsafe` block is as small as the call it wraps and documents the invariant that makes it
      sound.
