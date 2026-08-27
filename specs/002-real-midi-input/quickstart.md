# Quickstart: Validating Real MIDI Input

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-27

Per the constitution's Verification Standard, **this project has no test suite**. Verification is
compiling and running the application by hand. This guide is the manual pass that stands in for one, and
it is a completion gate, not an afterthought.

---

## Prerequisites

| Requirement | Why |
|---|---|
| macOS | The only supported platform (FR-039) |
| Xcode Command Line Tools | CoreMIDI headers and linking |
| Rust stable + Node 20+ | Already installed in this workspace |
| **At least one real MIDI device** | Most scenarios below cannot be validated without one |
| **Audio MIDI Setup** (`/Applications/Utilities/`) | The reference for SC-002 — the OS's own view of MIDI |
| A second MIDI-capable application | For the virtual destination (US4) |

**No-hardware fallback**: enable **IAC Driver** in Audio MIDI Setup (*Window → Show MIDI Studio →
IAC Driver → Device is online*). This gives a real, OS-provided endpoint with no physical device, and it
is genuine MIDI plumbing — not a mock. It cannot validate hot-plug (US3), which needs a cable.

---

## Build and run

```bash
cargo clippy --all-targets -- -D warnings   # gate 1: zero warnings
cargo fmt --check                            # gate 2
npm run typecheck                            # gate 3: tsc --noEmit, strict
npm run tauri dev                            # gate 4: launch
```

`src/bindings.ts` is regenerated on debug startup. If `npm run typecheck` fails with
`Cannot find module './bindings'`, run `npm run tauri dev` once first — the bindings are generated at
runtime and are gitignored.

---

## Scenario 1 — Nothing is fabricated (US1, FR-001, SC-003)

**The single most important check in this document.** Everything else confirms the app works; this
confirms it stopped lying.

1. Disconnect every MIDI device. Disable IAC Driver in Audio MIDI Setup.
2. Launch the application. Leave it completely untouched for **five minutes**.

**Expected**: the event list is empty and stays empty. The row count never increments. The Sources list
is empty and states that no MIDI devices were found (FR-007).

**Fails if**: a single row appears. Any row here means a generated path survived — stop and find it.

Then, as a static confirmation of SC-014:

```bash
rg -i 'simul|mock|fake|dummy|synthetic' crates/ src-tauri/src/ src/
rg 'rand' Cargo.toml crates/*/Cargo.toml src-tauri/Cargo.toml
```

**Expected**: no simulator module, no `rand` dependency, and no code path constructing a `MidiEvent`
from anything but received bytes. Matches inside prose documentation explaining the removal are fine;
matches in code are not.

---

## Scenario 2 — The source list is the machine's real list (US2, SC-002)

1. Attach one or more devices. Open **Audio MIDI Setup → Window → Show MIDI Studio**.
2. Launch the application and expand **Sources**.

**Expected**: every input endpoint the OS shows appears under `MIDI sources`, **name for name and count
for count**, using the OS's own names with no cleanup or renaming (FR-004). A device exposing several
ports shows one entry per port, not one per device (FR-005).

**Also check**: `Act as a destination for other programs` is present as a standalone row, and
`Spy on output to destinations` is present, empty, and states it is unavailable (FR-028).

---

## Scenario 3 — Real messages, real bytes (US1, FR-014, SC-001, SC-008)

1. With a device selected, play a note.

**Expected**: a row within a quarter second (SC-001) — `Note On`, the correct channel, the note and
velocity actually played. Releasing produces `Note Off` (or `Note On` velocity 0, if that is what the
device sends — report what arrived, do not normalise it).

2. Move a knob or fader. **Expected**: `Control` rows with the real controller number and value.
3. Play a chord and release it. **Expected**: one row per message, in transmission order, none merged or
   reordered (US1 scenario 4).
4. Enable the `Data` column's raw hex view and compare against the device's documented output.

**Expected**: byte-for-byte identical in value, order, and count (SC-008).

---

## Scenario 4 — Hot-plug (US3, FR-009 to FR-012, SC-005, SC-006)

**Requires a physically connected device.**

1. With the application running and the device **selected and transmitting**, unplug it.

**Expected**: within two seconds the Sources list updates (SC-005). The application keeps running. Other
sources keep being monitored (FR-010). **Rows the device already produced remain listed and correctly
attributed** (FR-011).

2. Plug it back in.

**Expected**: it returns **still selected**, and its events resume with no control touched (FR-012,
SC-006).

3. Unplug the last remaining device.

**Expected**: the window states that no MIDI devices are available — not a frozen or unexplained empty
list (US3 scenario 5).

> If hot-plug does not fire, this is the risk recorded in
> [research.md D4](./research.md#d4-hot-plug-via-clientnew_with_notifications-created-on-the-main-thread-first):
> the notification client must be created **first**, on the main thread, before any other MIDI call.
> Check that ordering in `setup` before looking anywhere else.

---

## Scenario 5 — Selection survives quit, relaunch, and a different socket (SC-007, FR-031 to FR-034)

1. With several devices attached, select some and deselect others. Quit. Relaunch.

**Expected**: exactly the same devices are selected.

2. Quit. Move a device to a **different physical USB port**. Relaunch.

**Expected**: still selected — identity keys on the device, not the socket (FR-031).

3. Quit **while a selected device is unplugged**. Relaunch, then reattach it.

**Expected**: it returns selected. Its choice was not erased by being absent (FR-033) — the failure mode
[D6](./research.md#d6-remembered-selections-outlive-absent-devices) exists to prevent.

4. If two attached ports share a display name, select one only, relaunch.

**Expected**: only that one is selected (FR-033).

5. Attach a device the settings have never seen. **Expected**: it appears **unselected** (FR-032), so it
   cannot flood a list you had narrowed.

---

## Scenario 6 — Receiving from another application (US4, FR-024 to FR-027)

1. Check `Act as a destination for other programs`.
2. In another MIDI-capable application, enumerate destinations.

**Expected**: this monitor appears in the list (FR-024).

3. Send messages to it. **Expected**: they are listed, attributed to that entry (FR-025).
4. Uncheck the entry. **Expected**: messages stop entering the list (FR-026).

---

## Scenario 7 — Malformed and awkward input (FR-016 to FR-020, SC-010, SC-011)

**The scenarios that disqualified `midir`** ([D1](./research.md#d1-midi-system-access--coremidi-not-midir)).
If a device that emits these is not available, send them through IAC Driver from a script.

| Send | Expected | Requirement |
|---|---|---|
| A SysEx dump ≥ 1000 bytes | **One** row, true byte count shown — not fragments | FR-015, FR-017, SC-010 |
| A SysEx dump > 65 536 bytes | One row, marked truncated, reporting the **true** size | FR-018 |
| A SysEx started then abandoned (unplug mid-dump) | Reported incomplete — not held forever | FR-019 |
| Running status (repeated notes, status byte omitted) | Every message listed in full with its implied status — **not** dropped | FR-016 |
| A stray data byte with no preceding status | A row under `Invalid` showing the raw bytes | FR-020 |
| A valid message immediately after that stray byte | Listed correctly — one malformed byte must not swallow what follows | FR-020, SC-011 |

Uncheck every filter category except `Invalid` to isolate the malformed rows.

---

## Scenario 8 — Everything from feature 001 still works (FR-036, FR-037, SC-012)

Real traffic must not have changed any behaviour the previous feature specified.

- All five columns render, `Time` as `HH:MM:SS.mmm`.
- `Remember up to [N] events` trims immediately when lowered; `Clear` empties and refills.
- Source checkboxes work; a partly-checked group shows the **mixed** state.
- **Every** Filter checkbox demonstrably changes the list (SC-012). Play traffic containing each type
  and toggle its box off and on.
- `One Channel` = *N* restricts to that channel and excludes channel-less messages.
- The hex prefix filter includes and excludes correctly; `9` matches `90`–`9F`.
- Hiding columns reflows without gaps and never changes which events are listed.

---

## Scenario 9 — Load (SC-009, FR-022, FR-023)

Drive **≥ 500 messages/second across at least two sources** — a sequencer sending Clock plus a
controller, or two IAC buses.

**Expected**: every control responds within a quarter second. Rows stay in arrival order — no later
message appears above an earlier one (FR-022). The retention cap holds exactly. **No message is missing
from the record**, or if the application genuinely cannot keep pace, it *says so* rather than dropping
silently (FR-023).

> Note the pre-existing drop path: `EventPump::push` drops an event when the queue lock is contended
> ([`stream.rs`](../../src-tauri/src/stream.rs)). Acceptable for a simulator; FR-023 makes it reportable
> now. Confirm a drop is counted and surfaced, not invisible.

---

## Scenario 10 — Screenshot fidelity (FR-036, SC-013)

Place the running window beside `screenshots/main-screen.png`, `sources.png`, and `filters.png`.

**Expected**: same controls, same verbatim labels, same order, same indentation, same default states.
The **only** permitted differences are real device names in the Sources list and messages about hardware
availability. A relabelled, reordered, restyled, or repurposed control is a Constitution VI violation
and fails the gate regardless of how well it works.

---

## Scenario 11 — Denied permission (FR-040, SC-016)

If macOS prompts for MIDI or input-device access, **deny it**, then relaunch.

**Expected**: the application states that access was refused — clearly distinct from "no devices found"
— and every control stays operable (FR-008). This is the distinction `MidiSystemStatusDto` exists to
carry; an empty list here would be a failure.

---

## Definition of Done

Every gate from the constitution, in order:

- [ ] `cargo clippy` — zero warnings
- [ ] `cargo fmt --check` — clean
- [ ] `tsc --noEmit` under `strict` — zero errors
- [ ] App builds, launches, scenarios 1–11 exercised by hand
- [ ] Every new/modified public item carries rustdoc/TSDoc explaining **why**
- [ ] Every new/modified module carries a module-level doc
- [ ] No test artifacts introduced
- [ ] No dead code, no magic values, no `unwrap`/`expect`/`any` outside `main`
- [ ] IPC changes regenerated into `src/bindings.ts` from the Rust declarations
- [ ] Screenshot comparison done (Scenario 10)
- [ ] **No simulation remains anywhere in the product** (Scenario 1) — the gate this feature exists for
