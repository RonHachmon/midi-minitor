# Phase 1 Data Model: Display Preferences

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Date**: 2026-09-23

All types below live in `crates/midi-core/src/domain/` unless stated otherwise. Every enum is
matched exhaustively with no catch-all arm, so adding a variant is a compile error at each site
that must handle it (Principle VII).

---

## New domain types

### `DisplaySettings` — `domain/display.rs`

The six choices, as one value. One set for the application, not one per source (Assumptions).

| Field | Type | Default (FR-011) |
|-------|------|------------------|
| `time` | `TimeFormat` | `ClockTime` |
| `note` | `NoteFormat` | `NameMiddleC3` |
| `controller` | `ControllerFormat` | `StandardName` |
| `data` | `DataFormat` | `Decimal` |
| `program` | `ProgramNumbering` | `FromOne` |
| `expert` | `ExpertMode` | `Off` |

Derives `Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize`. `Copy` because it is six
enum discriminants — passing it by value to a renderer costs nothing and avoids threading a
lifetime through every rendering call.

**Invariant**: every field is always a valid variant. There is no constructor that can fail and
no `new()` returning `Result` — the type cannot represent an invalid state, which is why
`set_display_settings` has no validation error to report (see [contracts/ipc-commands.md](./contracts/ipc-commands.md)).

### The six setting enums — `domain/display.rs`

| Type | Variants | Verbatim label (FR-008) |
|------|----------|-------------------------|
| `TimeFormat` | `ClockTime` | `Clock time` |
| | `HostInteger` | `Host time (integer)` |
| | `HostSeconds` | `Host time (seconds)` |
| | `HostNanoseconds` | `Host time (nanoseconds)` |
| `NoteFormat` | `NameMiddleC3` | `Note (Middle C = C3)` |
| | `NameMiddleC4` | `Note (Middle C = C4)` |
| | `Decimal` | `Decimal number` |
| | `Hexadecimal` | `Hexadecimal number` |
| `ControllerFormat` | `StandardName` | `Standard name` |
| | `Decimal` | `Decimal number` |
| | `Hexadecimal` | `Hexadecimal number` |
| `DataFormat` | `Decimal` | `Decimal number` |
| | `Hexadecimal` | `Hexadecimal number` |
| `ProgramNumbering` | `FromOne` | `1 – 128 (Standard)` |
| | `FromZero` | `0 – 127 (Less common)` |
| `ExpertMode` | `Off`, `On` | `Expert mode` (the checkbox itself) |

Each carries `const fn label(self) -> &'static str` and `const ALL: [Self; N]`, matching the
existing `MessageKind::label` / `Column::ALL` convention. The en dash in the two
`ProgramNumbering` labels is U+2013 with a space either side, as `screenshots/setting.jpg` shows.

`ExpertMode` is a two-variant enum rather than a `bool` so that its rendering sites read
`ExpertMode::On` rather than `true`, and so the three suppressions it governs are matched
exhaustively rather than branched on.

**Group labels** (FR-008), also `&'static str` constants in this module: `Time format`,
`Note format`, `Controller format`, `Data format`, `Program number`, `(Decimal)`.

**Expert-mode explanatory lines** (FR-010), a `const [&str; 3]`:
`Data formatted according to settings above`,
`Note On with velocity 0 shows as Note Off`,
`Zero timestamp shows time received`.

### `Arrival` — `domain/ids.rs`

Replaces `MidiEvent.timestamp`. Both readings are taken at the same instant by one `Clock` call.

| Field | Type | Meaning |
|-------|------|---------|
| `wall` | `Timestamp` | Milliseconds since local midnight — what `Clock time` renders. Unchanged in meaning from today. |
| `host` | `HostTime` | The host clock, as the platform reported it. |

### `HostTime` — `domain/ids.rs`

```text
enum HostTime {
    Stamped(HostTicks),
    ZeroMeaningNow { received: HostTicks },
}
```

Two variants because CoreMIDI defines a packet timestamp of zero as "now" (verified in
`coremidi` 0.9.2 — see [research.md](./research.md) D3), and the reference image's third
explanatory line is exactly about that case.

**Rendering rule** (FR-022): `Stamped(ticks)` renders `ticks`. `ZeroMeaningNow { received }`
renders `received` when `ExpertMode::Off` — "Zero timestamp shows time received" — and renders
`0` when `ExpertMode::On`.

Windows never produces `ZeroMeaningNow`: `QueryPerformanceCounter` is read in the callback and
has no zero-means-now convention, so that adapter always constructs `Stamped`. This is honest
rather than a gap — the variant models a CoreMIDI convention, and the Windows adapter has none to
model.

### `HostTicks` — `domain/ids.rs`

Newtype over `u64`. Raw platform ticks: mach absolute time on macOS,
`QueryPerformanceCounter` units on Windows. Deliberately **not** normalised to nanoseconds, so
that `Host time (integer)` can render the platform's own value (see [research.md](./research.md)
D2).

### `TickRate` — `domain/ids.rs`

Newtype carrying ticks per second, held once by the `Monitor` rather than per event. Supplies the
two conversions that make FR-014 true by construction:

- seconds = `ticks / rate`
- nanoseconds = `ticks * 1_000_000_000 / rate`

**Invariant**: non-zero. Construction returns `Result`, because a zero rate would make both
conversions a division by zero and `panic` is denied workspace-wide. A platform reporting zero is
a startup error, not a rendering-time one.

Both conversions must be computed to avoid overflow: `ticks * 1_000_000_000` overflows `u64` for
mach tick values within a machine's uptime, so the nanosecond conversion uses `u128`
intermediately, or multiplies by the timebase `numer`/`denom` rationally on macOS.

### `ControllerName` — `domain/controller.rs`

A `const [&str; 128]` table plus a lookup returning `Option<&'static str>`. `None` means the
number has no standard name; the renderer then shows the number, never a blank (FR-018). See
[research.md](./research.md) D4 for why this is hand-written.

---

## Changed domain types

### `MidiEvent` — `domain/event.rs`

`timestamp: Timestamp` becomes `arrival: Arrival`. This is the one field change that propagates
outward, to two construction sites:

- `crates/midi-macos/src/source.rs:234` — must also thread the per-packet timestamp through the
  decode loop, which currently flattens `packets.iter()` into a single `outcomes` vector and
  loses the association (`source.rs:193-195`).
- `crates/midi-windows/src/receive.rs:160` — the existing single arrival reading gains host ticks.

The type's doc comment about *why* raw bytes are stored but display values are not stays true and
becomes more load-bearing: this feature is the change that would have made cached display strings
disagree, and it is also what makes Expert mode implementable (see [research.md](./research.md)
D5).

### `MidiMessage` — `domain/message.rs`

`data_display()` and the private `note_name()` move to `domain/rendering.rs`. `display_name()`
stays — the Message column's name is a fact about the message type, not a display setting.

The doc on `note_name` currently justifies middle C = C4 as "the scientific convention and the one
that makes note 36 read as `C2` — matching the reference window". Per FR-011a that justification
is superseded and MUST be **replaced** with the reasoning from the plan's Complexity Tracking, not
deleted. Deleting it invites the next reader to restore C4 as a bug fix.

### `SendRecord` — `domain/send_record.rs`

**Unchanged.** Its `time: Timestamp` stays a plain wall clock. Display settings govern how
received traffic is read; the send log is out of scope (Assumptions, FR-003).

### `PersistedSettings` — `application/settings.rs`

Gains one field:

```text
#[serde(default)]
pub display: DisplaySettings,
```

`#[serde(default)]` is the whole compatibility story, for the same reason the `send` field
records: this feature *adds* a field and changes none, so a document written by any earlier
version has no `display` key, supplies the default, and keeps every other setting intact
(FR-029). No migration is written; one would be dead code.

FR-030 — an unrecognised stored value falls back to that setting's default without discarding the
rest — is why each setting enum needs its own `#[serde(default)]` or a deserialisation that
tolerates an unknown variant per field, rather than relying on the struct-level default. A single
bad field must not reset the other five, and must not take the user's filters down with it.

### `Monitor` — `application/monitor.rs`

Gains `display: DisplaySettings` and `tick_rate: TickRate`, a `set_display(&mut self, …)`, a
`display()` accessor, and includes `display` in `persisted_settings()`.

The `TickRate` is supplied at construction from the `Clock` port, not read on demand, because it
is fixed for the process lifetime and a renderer that reaches for the platform would be a domain
type doing I/O.

---

## Rendering rules — `domain/rendering.rs`

One module, whose reason to change is a display rule. Each function takes `&MidiEvent` or the
value it renders, plus `DisplaySettings` (and `TickRate` where host time is involved).

| Column | Governed by | Rule |
|--------|-------------|------|
| `Time` | `TimeFormat`, `ExpertMode` | `ClockTime` → `HH:MM:SS.mmm`, exactly as `Timestamp::to_display` does today. The three host forms render `HostTime` per the rule above. |
| `Message` | `ExpertMode` | `Off` → `MidiMessage::display_name()`, unchanged. `On` → what the retained `raw` status byte says, so a `Note On` with velocity zero reports as `Note On` (FR-022). Under running status `raw` carries no status byte, so the decoded interpretation stands. |
| `Data` — note numbers | `NoteFormat` | Note On, Note Off, Aftertouch (Poly). `NameMiddleC3` → octave = `note / 12 - 2`; `NameMiddleC4` → `note / 12 - 1`. Both produce a name for every number 0–127, running negative at the bottom (FR-016, SC-004). |
| `Data` — controller numbers | `ControllerFormat` | `StandardName` → the table, falling back to the number (FR-018). |
| `Data` — program numbers | `ProgramNumbering` | `FromOne` → `value + 1`; `FromZero` → `value` (FR-020). |
| `Data` — everything else | `DataFormat` | Velocities, controller values, pressures, pitch bend, song position, song select, time code (FR-019). |
| `Data` — System Exclusive | none | Keeps reporting `N bytes`; `Hexadecimal number` must not turn it into an unbounded byte dump (Edge Cases). |
| `Data` — invalid messages | none | Keeps reporting reason and byte count (Edge Cases). |
| All of `Data` | `ExpertMode` | `On` shows values raw rather than formatted per the settings above (FR-022, first explanatory line). |

**Independence invariant** (FR-021): each setting governs only what it names. The velocity beside
a note is governed by `DataFormat`, never by `NoteFormat` — Acceptance Scenario 3.6 tests exactly
this.

---

## What is deliberately not modelled

- **Which tab is showing.** A property of the moment, not a stored setting (Key Entities). It
  lives in webview state, like the current screen does.
- **`Sources` and `Other` tab contents.** Out of scope by instruction. `UnavailableTab.tsx`
  renders a statement, holds no state, and offers no control (FR-005).
- **Per-source or per-column overrides.** `screenshots/setting.jpg` shows no such scoping.
- **A dirty/applied distinction.** Changes take effect immediately (FR-024), so there is no
  pending state to model and no Apply or Revert button to add.
