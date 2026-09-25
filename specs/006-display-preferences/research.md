# Phase 0 Research: Display Preferences

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Date**: 2026-09-23

Seven decisions. Each was checked against the code and the vendored crate sources actually in
`Cargo.lock`, not against recollection — the versions below are the ones this workspace resolves.

---

## D1 — Re-rendering retained events reuses the snapshot-return precedent

**Decision**: `set_display_settings` returns a freshly built `SnapshotDto`, exactly as
`set_retention_limit`, `set_capture_state`, and `clear_events` already do. The webview replaces
its event list with the returned snapshot. No new mechanism, no re-streaming, no cache
invalidation.

**Rationale**: FR-025 requires retained events to re-render. The obstacle looked like
`EventDto::from_event` formatting `time` and `data` into strings at capture
([dto.rs:56-71](../../src-tauri/src/dto.rs#L56-L71)) — but those strings are built *from*
`MidiEvent`, and every retained `MidiEvent` is still held in Rust and reachable through
`Monitor::visible_events()` ([monitor.rs:208](../../crates/midi-core/src/application/monitor.rs#L208)).
Rendering is therefore a pure function of retained state, and re-rendering is just calling it
again.

`SnapshotDto` already carries `high_water_mark`, documented as existing so that "the webview
discards stream batches at or below this mark, so a batch in flight during a settings change
cannot resurrect a filtered-out event". A display-settings change is precisely such a settings
change, and the guard it needs is already built and already correct.

**Alternatives considered**:

- *Send unformatted values and render in the webview.* Rejected on Principle IV: the webview must
  not contain business rules, and `data_display`'s own doc explains why rendering lives in the
  domain — it is a fact about MIDI, and moving it across the serialization boundary would
  duplicate it. It would also put the 128-name controller table in TypeScript.
- *Re-stream every retained event through the `EventPump`.* Rejected: the pump coalesces
  *arrivals* into frame batches. Pushing a thousand retained rows through it would race the live
  stream, and the webview would have to distinguish a replay from new traffic — inventing a
  problem the snapshot return does not have.
- *Cache rendered strings on `MidiEvent` and invalidate.* Rejected on Principle I and on
  `MidiEvent`'s own doc, which states that display values are computed on demand precisely
  because "caching them would create several representations of one fact that could disagree
  after a change". This feature is that change.

---

## D2 — Host time is captured at arrival, as raw platform ticks plus a process-wide rate

**Decision**: `MidiEvent` carries an `Arrival { wall: Timestamp, host: HostTime }`. `HostTime`
stores the platform's raw tick count (`HostTicks(u64)`). The ticks-per-second (`TickRate`) is
read once at startup, reported by the `Clock` port, and held by the `Monitor` — not repeated on
every event.

**Rationale**: FR-025a. A user who captures events under `Clock time` and then selects
`Host time (nanoseconds)` must see host time for those already-retained events; nothing can be
re-derived after the fact, so both readings are taken at arrival whether or not the setting that
needs them is selected. The cost is two integer reads per event on a path that already allocates
a `Vec<u8>` per event.

Raw ticks rather than normalised nanoseconds, because `Host time (integer)` means the platform's
own host-clock value — the number a user compares against another tool printing
`mach_absolute_time`. Normalising at the edge would silently redefine that option. Storing ticks
and converting at render time also satisfies FR-014 by construction: all three host-time
renderings are conversions of one stored value.

`TickRate` held once rather than per event, because it is fixed for the life of the process on
both platforms; copying it onto every event would be a magic constant duplicated a thousand
times.

**Alternatives considered**:

- *A separate `HostClock` port.* Rejected on Principle V: it would have the same two
  implementations as `Clock`, be constructed at the same place, and have no independent reason to
  change. Worse, the two readings must be taken at the same instant — two ports invites two call
  sites and a skew between the columns.
- *Store nanoseconds and derive ticks back.* Rejected: the conversion is lossy in both directions
  on macOS, where the timebase is a rational `numer/denom` that is not 1 on Apple silicon.
- *Capture host time only while a host format is selected.* Rejected: it is the exact failure
  FR-025a forbids — retained events would have a hole where the setting had not yet been chosen.

---

## D3 — Platform host clocks: `mach_absolute_time` via `libc`, QPC via the `windows` crate

**Decision**: on macOS read the CoreMIDI packet timestamp, with `mach_timebase_info` from `libc`
supplying the rate. On Windows read `QueryPerformanceCounter` in the MIDI callback, with
`QueryPerformanceFrequency` supplying the rate, both through the existing `windows` dependency
plus its `Win32_System_Performance` feature.

**Rationale** — verified against the vendored sources, not assumed:

- `coremidi` 0.9.2 exposes `Packet::timestamp() -> Timestamp` where `pub type Timestamp = u64`
  (`src/packets.rs:128`, `src/events.rs:13`). That value is a `MIDITimeStamp`, i.e. mach absolute
  time, and the crate's own docs state "the timestamp represents the time at which the events are
  to be played, **where zero means 'now'**" (`src/packets.rs:198-202`). This is the direct
  mechanical basis for the `Zero timestamp shows time received` line in the reference image — see
  D5.
- `libc` 0.2 declares `mach_timebase_info(info: *mut mach_timebase_info) -> c_int` and
  `mach_timebase_info_data_t` on Apple targets (`src/unix/bsd/apple/mod.rs:4514`, `:179`). `libc`
  is already in `Cargo.lock` transitively; this makes it a direct macOS-only dependency of
  `midi-macos`. `coremidi-sys` 3.2.1 exposes no timebase function, so it cannot serve.
- `windows` 0.62.2 declares `QueryPerformanceCounter(*mut i64)` and
  `QueryPerformanceFrequency(*mut i64)` in `Windows/Win32/System/Performance/mod.rs:866,871`,
  gated by the `Win32_System_Performance` feature (`Cargo.toml:622`). The crate is already a
  dependency; only the feature is added.

Both are monotonic tick counters with a platform-specific rate, which is exactly what `HostTicks`
and `TickRate` model.

**Alternatives considered**:

- *Hand-declared `extern "system"` FFI.* Rejected by Principle II and by the existing
  justification in `midi-windows/Cargo.toml`, which already rejects hand-declared signatures on
  the grounds that a mistyped field is a silent memory bug rather than a compile error.
- *The `mach2` crate for the timebase.* Rejected: `libc` already declares it, is already in the
  tree, and adding a second crate for one function is the dependency bloat Principle II guards
  against.
- *Windows `dwParam2` from `midiInProc` as host time.* Rejected: it is milliseconds since
  `midiInStart`, a coarser and differently-based clock than QPC, and `receive.rs:158` already
  documents that the arrival reading is taken in the callback "and nowhere else" precisely so the
  displayed time is the time of arrival. QPC fits that existing rule; `dwParam2` would not.
- *`std::time::Instant`.* Rejected: it exposes no absolute value, only differences, so
  `Host time (integer)` could not be rendered at all.

---

## D4 — The 128 standard controller names are a hand-written domain table

**Decision**: a named `const` array of 128 entries in `domain/controller.rs`, with the naming
source documented at the module level. A controller with no standard name renders by number
rather than blank (FR-018).

**Rationale**: no crate in `Cargo.lock` supplies these, and none should be added for them. The
workspace deliberately depends on no general MIDI crate — `midi-macos/Cargo.toml` and
`midi-windows/Cargo.toml` both record why `midir`, and by extension `wmidi`, `midi-msg`, and
`midly`, were rejected: each refuses or discards the malformed input a monitor exists to display.
A search of the vendored registry confirms none of the four is present today.

Even adopting one would not answer the question. `wmidi::ControlFunction` models controllers as
associated constants whose names are Rust identifiers, not display text; `midi-msg` models them
as enum variants for round-tripping. Either way a number-to-string table gets written by hand —
the crate would add a dependency tree without removing the work. Controller names are also
domain vocabulary, which Principle II names as its own exception.

Recorded in [plan.md](./plan.md) Complexity Tracking, as Principle II requires of any
hand-rolling.

**Alternatives considered**: covered above — adopt `wmidi`, adopt `midi-msg`, or generate the
table from an external data file at build time. The last was rejected because a build script to
read a constant list is more machinery than the list.

---

## D5 — Expert mode reads the retained raw bytes; the decoder is not changed

**Decision**: `Expert mode` renders from `MidiEvent.raw`. The decoder keeps converting a `Note On`
with velocity zero into `MidiMessage::NoteOff`
([decoder.rs:430-438](../../crates/midi-core/src/domain/decoder.rs#L430-L438)); when the checkbox
is ticked, the renderer inspects the retained status byte and reports what actually arrived.

**Rationale**: this is the decision that keeps the feature additive, and it fell out of a doc
comment already in the code. `MidiEvent.raw` is documented as kept because "interpretation is
lossy in two ordinary cases: under running status the wire carries no status byte, and a
`Note On` with velocity zero means a release, so re-encoding it would produce an `8n` status the
device never sent." Expert mode is the user asking to see that retained truth. The information is
already there; nothing needs to be recovered.

The alternative — stop converting in the decoder and apply the convention at render time — was
seriously considered and rejected, because it would silently move vel-0 note-ons from the
`Note Off` filter checkbox to the `Note On` one. That breaks FR-027 (a display setting must not
change which events pass the filter) and would change existing behaviour on a surface
`screenshots/filters.png` governs, which Principle VII forbids. Reading `raw` leaves the model,
the filter, and the decoder untouched.

**Consequence to implement carefully**: the renderer's Expert-mode path must handle running
status, where `raw` carries no status byte. In that case the retained bytes cannot contradict the
decoded interpretation, so the decoded message stands.

**Alternatives considered**:

- *A flag on `MidiMessage` recording that a conversion happened.* Rejected on Principle VII —
  it is a display concern riding along on a domain type, the same shape as the `is_mock` flag the
  constitution names as forbidden.
- *Render both forms and let the webview choose.* Rejected on Principle IV: it puts the rule in
  the webview and doubles the payload for a setting almost always off.

---

## D6 — Labels come from Rust, following `get_filter_model`

**Decision**: `get_display_model` returns the tab's structure — group labels, option labels, and
which option is selected — as data. The webview renders what it is handed and hard-codes no
user-visible string from `screenshots/setting.jpg`.

**Rationale**: the precedent is explicit and was set for this exact reason. `get_filter_model`'s
doc states the panel's "entries and labels come from Rust so the interface cannot invent a
category or reword a checkbox — the screenshot is the design authority, and these strings are its
normative content." Principle VI requires labels be copied verbatim including capitalisation,
punctuation, and parentheses; strings such as `Note (Middle C = C3)`, `1 – 128 (Standard)`, and
`0 – 127 (Less common)` contain an en dash and parenthetical text that a well-meaning edit would
normalise. Keeping them in Rust next to the enum they name makes drift a compile-adjacent
concern rather than a styling one.

Note the en dash in `1 – 128 (Standard)` and `0 – 127 (Less common)`: it is U+2013 with spaces
either side, as the image shows, not a hyphen.

**Alternatives considered**: hard-coding the strings in `DisplayTab.tsx`. Rejected — it is the
practice `get_filter_model` exists to prevent, and it would put the normative content of a
reference image in the layer most likely to be restyled.

---

## D7 — One `set_display_settings` command, not six

**Decision**: a single command takes the whole `DisplaySettingsDto` and returns the updated model
together with a fresh snapshot. `get_display_model` reads it once when the screen mounts.

**Rationale**: the tab is one form whose six controls are independent, and the codebase already
has both shapes — `set_filter` takes a composite, while the send screen uses per-field commands
that each return the whole model. The composite fits here because there is no per-field
validation to report: every value is an enum, so the DTO cannot carry an invalid state, and
generated string unions mean the webview cannot compose one. Six commands would be six round
trips to the same persistence write with no error case to distinguish them.

Returning `{ model, snapshot }` in one payload rather than making the webview call `snapshot`
afterwards keeps the change atomic — there is no interval in which the settings have changed and
the visible rows have not.

**Alternatives considered**:

- *Six commands.* Rejected as above.
- *Return only the snapshot.* Rejected: the tab must reflect the state the core actually holds,
  including any value it declined or defaulted (FR-030), so the model is returned with it.
