# Contract: Display Settings and Their Rendering Rules

**Feature**: [../spec.md](../spec.md) | **Date**: 2026-09-23

The domain contract: what each setting governs, and what it must not. Types are catalogued in
[../data-model.md](../data-model.md); this file fixes the *behaviour* an implementer must
reproduce.

---

## Placement

`DisplaySettings` and the renderers are **domain**, in `crates/midi-core/src/domain/`.

The precedent is explicit. `MidiMessage::data_display`'s doc already states why rendering lives in
the domain: what belongs in the Data cell is a fact about MIDI, not a presentation choice, and
rendering it in the webview would put a MIDI rule on the far side of a serialization boundary and
duplicate it. A settings-dependent rendering is that same fact with a parameter.

The webview continues to receive finished strings. It never sees a note number, a controller
number, or a tick count.

---

## Persistence

Stored inside `PersistedSettings` as `#[serde(default)] pub display: DisplaySettings`.

- **FR-029** — a document written before this feature has no `display` key, takes the default,
  and keeps every other setting. No migration is written; one would be dead code. This is the
  same additive shape the `send` field records, and its doc explains the contrast with the
  filter change that *did* need a migration.
- **FR-030** — an unrecognised value for one setting falls back to that setting's default
  **without** discarding the other five, and without taking filters, columns, retention, sources,
  or the send screen's state down with it. Per-field tolerance, not struct-level.

This matters more than it looks: `StoreSettingsRepository::load` treats an unreadable document as
absent, so a deserialisation failure anywhere in the document silently discards everything the
user had configured.

---

## Rendering rules

### `Time` column — `TimeFormat`, with `ExpertMode`

| Setting | Renders |
|---------|---------|
| `Clock time` | `HH:MM:SS.mmm` from `Arrival.wall` — byte for byte what `Timestamp::to_display` produces today, so `screenshots/data.png` still matches under the default. |
| `Host time (integer)` | The raw tick count. |
| `Host time (seconds)` | `ticks / rate`, with a fractional part. |
| `Host time (nanoseconds)` | `ticks * 1_000_000_000 / rate`. |

**FR-014** — the three host forms are conversions of one stored `HostTicks`, so they are
consistent by construction rather than by agreement between three code paths.

**Overflow**: `ticks * 1_000_000_000` exceeds `u64` for mach tick values within an ordinary
machine uptime. Compute through `u128`, or apply the macOS timebase rationally. A wrapping
multiply here would show a plausible wrong number, which is worse than an obvious one.

**Zero timestamps** (FR-022, third explanatory line): `HostTime::ZeroMeaningNow { received }`
renders `received` under `ExpertMode::Off`, and `0` under `ExpertMode::On`. `Clock time` is
unaffected — the wall-clock reading is always real.

### `Message` column — `ExpertMode` only

| Mode | Renders |
|------|---------|
| `Off` | `MidiMessage::display_name()`, unchanged from today. A `Note On` with velocity zero reads `Note Off`, because the decoder converted it. |
| `On` | What the retained `MidiEvent.raw` status byte says. A `Note On` with velocity zero reads `Note On`. |

**This is why the decoder is not changed.** `decoder.rs:430-438` keeps converting; Expert mode
consults the raw bytes the event already retains. `MidiEvent.raw`'s own doc anticipates exactly
this case. The rejected alternative — moving the conversion to render time — would have shifted
vel-0 note-ons from the `Note Off` filter checkbox to the `Note On` one, breaking **FR-027** and
altering a control `screenshots/filters.png` governs. See [../research.md](../research.md) D5.

**Running status**: `raw` carries no status byte, so it cannot contradict the interpretation. The
decoded message stands.

### `Data` column

| Value | Governed by | Rule |
|-------|-------------|------|
| Note numbers — Note On, Note Off, Aftertouch (Poly) | `NoteFormat` | `Note (Middle C = C3)` → octave `note / 12 - 2`. `Note (Middle C = C4)` → octave `note / 12 - 1`. `Decimal number` / `Hexadecimal number` → the bare number. |
| Controller numbers | `ControllerFormat` | `Standard name` → the table, **falling back to the number** when there is none (FR-018) — never blank, never a placeholder. |
| Program numbers | `ProgramNumbering` | `1 – 128 (Standard)` → `value + 1`. `0 – 127 (Less common)` → `value`. |
| Velocities, controller values, pressures, pitch bend, song position, song select, time code | `DataFormat` | Decimal or hexadecimal (FR-019). |
| System Exclusive | none | Keeps reporting `N bytes`, framing included. `Hexadecimal number` must **not** turn it into an unbounded byte dump. |
| Invalid messages | none | Keeps reporting reason and byte count. |
| All of the above | `ExpertMode` | `On` shows values raw rather than formatted per the settings above (first explanatory line). |

**Every note number 0–127 produces a name under both conventions** (FR-016, SC-004). Under
`C3` the octave runs negative at the bottom: note 0 is `C-2`. That is correct, not a defect to
clamp away.

---

## Invariants an implementer must not break

1. **Independence** (FR-021). Each setting governs only what it names. The velocity beside a note
   is `DataFormat`, never `NoteFormat` — Acceptance Scenario 3.6 exists to catch exactly this.
2. **Filters are untouched** (FR-027). Filtering is over what arrived. No rendering path may feed
   back into `EventLog::view`, and no display setting may change which events pass.
3. **Re-rendering moves nothing** (FR-026). Rebuilding the snapshot must not admit, drop,
   reorder, or duplicate an event, nor change scroll position, selection, or paused state. It
   reads `Monitor::visible_events()` and formats; it must not touch the log.
4. **Exhaustive matching, no catch-all.** Every `match` over a settings enum lists its variants.
   With no test suite, the compiler enumerating the work is the substitute for one — a `_ =>` arm
   would let a seventh setting ship half-implemented.
5. **`Expert mode` is reversible without loss** (FR-023). Ticking it suppresses; it never
   overwrites the five stored format choices. Unticking restores them without the user
   re-choosing, which follows from `ExpertMode` being a separate field rather than a state the
   others collapse into.
6. **Labels are data, not code.** Every user-visible string from `screenshots/setting.jpg` lives
   beside the enum it names and crosses to the webview over IPC (see
   [ipc-commands.md](./ipc-commands.md)). No label is written in TypeScript.
