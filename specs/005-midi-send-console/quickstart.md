# Quickstart: Verifying the Send Screen

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Date**: 2026-08-29

This project has no test suite by design. Verification is the compiler plus this manual pass — see the
constitution's Verification Standard. Every scenario below names the requirement it proves, so a
partial pass can be reported honestly rather than as "mostly works".

---

## Gates

Run all four before working through the scenarios. A failure here is a failure; none of it is advisory.

```bash
cargo clippy --all-targets -- -D warnings
cargo fmt --check
npm run typecheck
npm run tauri dev
```

**Build the other platform's adapter too.** This feature adds an output path to *both* adapters, and a
macOS developer never compiles `midi-windows`. From Windows:

```bash
rustup target add x86_64-apple-darwin
cargo clippy -p midi-core -p midi-macos --target x86_64-apple-darwin -- -D warnings
```

From macOS, the reverse:

```bash
rustup target add x86_64-pc-windows-msvc
cargo clippy -p midi-core -p midi-windows --target x86_64-pc-windows-msvc -- -D warnings
```

If `npm run typecheck` fails with `Cannot find module './bindings'`, run `npm run tauri dev` once
first — `src/bindings.ts` is generated at debug startup.

---

## What you need

**Somewhere to send.** One of:

- **macOS** — the IAC Driver (*Audio MIDI Setup → Window → Show MIDI Studio → IAC Driver*, set online).
  It gives you a destination to send to and a source to watch it arrive on.
- **Windows** — a loopback utility such as loopMIDI. Its port appears as both a destination and a
  source.
- Any hardware synth or a DAW that lists MIDI inputs.

**Something to receive.** For the publishing scenarios you need a second program that lists MIDI
inputs — a DAW, a DJ application, or any MIDI monitor other than this one.

> Do **not** use `Microsoft GS Wavetable Synth` as a general check on Windows. It is a real MIDI output
> and will appear in the target list, but it only responds to note and controller messages on its own
> terms, so a silent response there proves nothing.

---

## Scenario 1 — Send a built-in request (US1, SC-001, SC-002)

1. Open the app. Switch to the send screen.
2. Choose a destination in the target picker.
3. Pick `Note On` from the request list.
4. Confirm the screen shows its name, a plain-language description, and the bytes `90 3C 64`.
5. Press send.

**Expected**: the send is confirmed on screen, and the receiving end shows a Note On, channel 1, note
60, velocity 100. Two interactions from a cold screen: pick, send.

**Also check**: change the note value and confirm the preview updates to match *before* sending, and
that the next send carries the new value (FR-009). Send the same request twice with no re-entry
(US1 sc. 5).

**Then**: with no target chosen, press send. The screen must explain that a target is needed and
transmit nothing (US1 sc. 4).

---

## Scenario 2 — Compose without hexadecimal (US2, SC-004)

1. Choose `Control` as the message type.
2. Confirm the fields shown are exactly channel, controller, and value — no more, no fewer (FR-013),
   each labelled by meaning rather than by byte position (FR-014).
3. Set channel 1, controller 7, value 100. Watch the preview become `B0 07 64`.
4. Send, and confirm the receiving end reports exactly that.

**Range check (FR-016)**: try to set value to 128. The entry must be prevented or corrected and the
permitted range stated. Nothing out of range may ever be transmitted.

**Field-list check**: switch to `Program`. The velocity and controller fields must disappear and a
program field appear — this is FR-013 doing its job, and it is the fastest way to see that the field
table lives in the core rather than in the screen.

**Raw entry (FR-017, FR-018)**: type `90 3C 64` into the raw entry and send — it must transmit
identically. Then type `90 3C` and try to send: refused, with the decoder's own reason, before
anything is transmitted. Then type `ZZ`: refused the same way.

**The lossy case, which is the point of storing raw bytes**: type `90 3C 00` (a Note On with velocity
zero). It must transmit as `90 3C 00`, **not** as `80 3C 00`. If it comes out as `80`, the raw
composition is being re-encoded and the contract is broken.

---

## Scenario 3 — Publish a source and be seen by another program (US3, SC-011) — macOS

1. Turn on publishing. Leave the name at `MIDI Monitor`, or set your own.
2. Open a **different** application that lists MIDI inputs. Confirm the name appears there.
3. Back in the app, choose the published source as the target and send a built-in request.
4. Confirm the other application receives it.

**Expected**: the other program lists the name you chose and receives what you send.

**Then check each of these:**

- **It appears here too.** The published source is a real endpoint, so this application's own Sources
  list now shows it. Tick it and watch your own sends arrive in the event table (FR-032). This is
  expected behaviour, not a loop.
- **Turning it off** removes it from the other program's list, and sending to a *destination* still
  works (FR-026).
- **Renaming** warns you first that the receiving program keeps its settings against the old name
  (FR-026, US3 sc. 5). Confirm, and the new name appears in the other program.
- **Restart the app.** The source is published again under the same name, and the chosen target is
  restored (FR-023, US3 sc. 4).
- **The mapping caveat.** In the receiving program, before you map anything, send a message and
  observe that nothing happens there. The send screen must report the message as *transmitted* — it
  must not claim the far end received or acted on it (FR-028). This is the first confusion a real user
  will hit, and reporting it honestly is the requirement.

---

## Scenario 4 — Publishing on Windows (FR-027, SC-007, SC-012)

1. Open the send screen on Windows.

**Expected**: the publish control states plainly that it is unavailable on this platform, says what to
do instead (a MIDI loopback utility, whose port appears in the target list like any other
destination), and **cannot be switched on**. There must be no control that looks operable and does
nothing.

2. With loopMIDI running, confirm its port is in the target list, send to it, and watch the message
   arrive — in this monitor's own Sources list, or in another program.

**Expected**: every other capability of the send screen works in full. Publishing is the only thing
Windows loses.

**Also confirm**: the MIDI Mapper is **not** listed as a target.

---

## Scenario 5 — Know what was sent (US4, SC-003)

1. Send several messages.
2. Confirm each is listed with its time, what it was, and its bytes.
3. Unplug the destination — or stop the loopback utility — and send again.

**Expected**: the failure is listed with a reason, visibly different from a successful send, and the
composition is unchanged so you can retry without re-entering anything (FR-029).

4. Re-send a recorded entry and confirm the identical bytes go out without recomposing (FR-031). Do
   this with a raw-typed record too: it must send what was typed.

**Every send must produce a visible outcome.** If any send completes with nothing on screen, SC-003
has failed.

---

## Scenario 6 — Save your own requests (US5, SC-010)

1. Compose something, save it under a name, and confirm it is listed beside the built-ins and marked
   as yours.
2. Restart the app. It is still there and sends the same bytes.
3. Delete it; no built-in is affected.
4. Try to delete or rename a built-in: refused, and the built-in library stays intact (FR-011).
5. Try to save under a name already used — by a saved request *or* a built-in: refused, and the list
   is unchanged.

---

## Scenario 7 — The two screens do not disturb each other (FR-002, FR-005, SC-006, SC-008)

1. Start monitoring a source with live traffic. Let rows accumulate.
2. Switch to the send screen, use it, and switch back.

**Expected**: retained events are still there, source selections and filters are unchanged, the
monitor is still running (or still paused, if you paused it), and no event was lost while you were
away.

3. Pause the monitor, switch to the send screen, and send something.

**Expected**: sending works normally — pause governs what is retained, never what is transmitted
(FR-033). Traffic sent during the pause is not retained, for the same reason nothing else is.

4. **Side by side with the reference images.** Compare the monitoring screen against
   `screenshots/sources.png`, `screenshots/filters.png`, and `screenshots/data.png`. Every label,
   control type, order, indentation, and default state must be unchanged by this feature. Nothing on
   that surface may have been relabelled, reordered, or restyled to make room for the send screen.

---

## Scenario 8 — Edges worth provoking

| Do this | Expect |
|---|---|
| Open the send screen with no MIDI destination on the machine at all | It says so plainly. No empty picker with no explanation. |
| Unplug the chosen destination, then send | A failure naming the target and the reason. Not silence, and not "malformed message". |
| Send a long System Exclusive message | It sends whole or fails with a reason. A partial transmission reported as success is the one unacceptable outcome. |
| Hold the send control down, or trigger it rapidly | No unbounded backlog, no unresponsive screen (FR-034). |
| Send `Note On`, then `All Notes Off` on the same channel | The note is released without hunting for anything. |
| Publish under the same name as a real device on the machine | Permitted. Two entries carry one name; you must still be able to tell which is which. |
| Publish with nothing subscribed, and send | Not an error, and not reported as one. |

---

## Definition of done for this feature

- [ ] All four gates pass, on both platforms, including the cross-compiled adapter check.
- [ ] Scenarios 1, 2, 5, 6, 7, 8 pass on **both** macOS and Windows.
- [ ] Scenario 3 passes on macOS.
- [ ] Scenario 4 passes on Windows.
- [ ] Every new `pub` item and every new module carries documentation saying *why* (Principle III).
- [ ] No test artifacts were introduced.
- [ ] `src/bindings.ts` was regenerated, not hand-edited.
- [ ] The monitoring screen was compared side by side against all three reference images.
