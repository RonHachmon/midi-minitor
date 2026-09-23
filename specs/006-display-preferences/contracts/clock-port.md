# Contract: The Extended `Clock` Port

**Feature**: [../spec.md](../spec.md) | **Date**: 2026-09-23

`crates/midi-core/src/application/ports.rs` — the application-layer port that keeps system time
out of the domain. This feature extends it rather than adding a second port; the reasoning is in
[../research.md](../research.md) D2.

---

## The port today

```text
pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}
```

Its doc states the problem it solves: the domain formats times to the millisecond and must not
call the system clock itself, because a domain that reaches for wall-clock time cannot be driven
deterministically from anywhere else. That reasoning is unchanged and extends unmodified to host
time.

## The port after this feature

```text
pub trait Clock: Send + Sync {
    /// The current wall-clock time, as milliseconds since local midnight.
    fn now(&self) -> Timestamp;

    /// Both readings of the current moment, taken together.
    fn arrival(&self) -> Arrival;

    /// Ticks per second for the host clock `arrival` reports.
    fn tick_rate(&self) -> TickRate;
}
```

### Why `arrival()` is one call rather than two

The wall clock and the host clock must describe the same instant. Two methods called in sequence
would let a scheduler interleave between them, and the `Time` column would disagree with itself
across a format change — the exact inconsistency FR-014 forbids. One call, both readings.

### Why `now()` survives alongside it

Three call sites need only the wall clock and have no host-time column:
`src-tauri/src/state.rs:254` and `crates/midi-windows/src/source.rs:514,577`, all of which stamp
**send records**. `SendRecord.time` stays a plain `Timestamp` because the send log is out of this
feature's scope (FR-003). Forcing those sites to build an `Arrival` and discard half of it would
be ceremony.

### Why `tick_rate()` is on the port rather than on each event

The rate is fixed for the life of the process on both platforms. The `Monitor` reads it once at
construction and holds it; repeating it on every event would be one magic constant copied a
thousand times (Principle I). A renderer that called the platform for it would be a domain type
doing I/O (Principle IV).

---

## Implementations

### `SystemClock` — `src-tauri/src/settings.rs`

The existing `now()` is unchanged: `chrono::Local`, milliseconds since local midnight, local
rather than UTC because the Time column is a wall clock the user compares against their own.

`arrival()` pairs that with a host reading, and `tick_rate()` reports the platform rate. Both are
`#[cfg]`-split:

**macOS** — `libc::mach_timebase_info` supplies `numer`/`denom`; the rate in ticks per second is
`1_000_000_000 * denom / numer`. Verified present in `libc` 0.2 at
`src/unix/bsd/apple/mod.rs:4514` and `:179`. The timebase is not 1:1 on Apple silicon, so it must
be read rather than assumed.

**Windows** — `QueryPerformanceFrequency` supplies the rate directly and
`QueryPerformanceCounter` the reading. Verified present in `windows` 0.62.2 at
`Windows/Win32/System/Performance/mod.rs:866,871`, behind the `Win32_System_Performance` feature.

**Both dependencies belong to `src-tauri`**, not to the platform adapter crates. `SystemClock`
lives there, and the adapters reach the clock through this port rather than calling the platform
themselves — so adding `libc` to `midi-macos` or the performance feature to `midi-windows` would
leave them unused in those crates and still missing where they are actually needed.

**A rate the platform will not report falls back to one tick per nanosecond** rather than failing
the launch. A machine that cannot answer costs the user the three `Host time` formats; taking the
whole monitor down over a column format would be out of all proportion. `TickRate::NANOSECOND` is
a constant precisely so that fallback cannot itself fail, which is what lets `tick_rate()` return
a value rather than a `Result`.

Both calls are `unsafe` FFI and both return a `Result` in the `windows` crate. Neither may
`unwrap` — `unwrap_used`, `expect_used`, and `panic` are denied workspace-wide. A failure at
startup is a typed error; a failure at read time falls back to the last good reading rather than
taking down the MIDI callback thread.

### `TickRate` construction is fallible

A rate of zero would make both host conversions a division by zero. `TickRate::new` returns
`Result`, and a platform reporting zero is a **startup** error surfaced through the existing
`MidiSystemStatus` path — not a rendering-time one. The renderer receives a `TickRate` that is
already known non-zero, so it has no error case.

---

## Where `arrival()` is called

Exactly two sites, both already the single place their adapter reads the clock:

### `crates/midi-macos/src/source.rs`

Line 234 currently calls `self.clock.now()`. It must instead use the **packet's own** timestamp,
because CoreMIDI stamps packets and defines zero as "now" — the mechanical basis for the
reference image's third explanatory line (`coremidi` 0.9.2, `src/packets.rs:128` and
`:198-202`; see [../research.md](../research.md) D3).

This requires a change to the decode loop at `source.rs:193-195`, which today flattens
`packets.iter()` into one `outcomes` vector and loses which packet each outcome came from.
Outcomes must carry their packet's timestamp through to `deliver`.

Mapping: a non-zero packet timestamp becomes `HostTime::Stamped`; a zero one becomes
`HostTime::ZeroMeaningNow { received }`, where `received` is `clock.arrival()` taken in the
callback. A System Exclusive transfer spanning several packets completes on the last, and takes
that packet's timestamp — which is when the message finished, and what the decoder reports.

### `crates/midi-windows/src/receive.rs`

Line 160 already takes the arrival reading in the callback "and nowhere else", so that however
long a batch waits on the worker the time shown is the time of arrival. It changes from `now()`
to `arrival()` and nothing about that rule changes.

Windows always produces `HostTime::Stamped`: `QueryPerformanceCounter` has no zero-means-now
convention. `dwParam2` from `midiInProc` is deliberately **not** used — it is milliseconds since
`midiInStart`, coarser and differently based than QPC.

---

## What this port still refuses to expose

No scheduling, no conversion helpers, no formatting. The port reports readings; the domain's
`rendering.rs` converts and formats them. A `Clock` that formatted would put a display rule in the
application layer, and a display rule is a domain fact (Principle IV).
