# Quickstart: Verifying Windows Support

**Feature**: `003-windows-support` | **Date**: 2026-08-28

This project has no test suite by constitutional design. Verification is compiling and running the
application, by hand, **on both platforms**. This guide is the manual pass that closes the feature.

A change is not done until both columns below are green. A Windows build that works while macOS has
regressed fails User Story 3, which is P1.

---

## Prerequisites

### Windows

- Windows 10 version 1809 or later, or Windows 11 — 64-bit
- Rust (stable) with the MSVC toolchain
- **Microsoft C++ Build Tools** (Desktop development with C++), which the MSVC toolchain requires
- **WebView2 Runtime** — pre-installed on Windows 11 and current Windows 10; install it if the window
  fails to appear
- Node 20 or newer

No MIDI permission prompt exists on Windows, so there is nothing to grant. A **Bluetooth** MIDI device
must be paired in Windows Settings first — until it is, Windows does not report it and the application
correctly does not list it.

### macOS

Unchanged from the previous feature: Xcode Command Line Tools, Rust (stable), Node 20+.

---

## Getting a MIDI input without hardware

The macOS IAC Driver has no Windows equivalent, so the macOS instructions in the README do not
transfer. Two routes, in preference order.

**1. Built-in loopback — Windows 11 with Windows MIDI Services.** Recent Windows 11 (24H2/25H2)
carries built-in loopback endpoints with nothing to install. If your machine has them, they appear
under `MIDI sources` as ordinary ports. Best option when available.

**2. A loopback utility plus a sender — works on the whole platform floor.** Install a MIDI loopback
utility (loopMIDI is the common one) to create a virtual port pair, then use any program that can send
to a MIDI port to drive it.

**Do not use `Microsoft GS Wavetable Synth`.** Every Windows machine has it and it looks like the
obvious candidate, but it is a MIDI *output*. This application lists inputs, so it will never appear
and you will spend twenty minutes deciding the feature is broken.

---

## Build and run

```bash
npm install
npm run tauri dev
```

Identical on both platforms. The window opens; expand **Sources**, tick a device, and play.

### The gates, run on each platform

```bash
cargo clippy --all-targets -- -D warnings
cargo fmt --check
npm run typecheck
```

All three must be clean. `src/bindings.ts` is generated when the app starts in debug mode — if
`npm run typecheck` fails with `Cannot find module './bindings'`, run `npm run tauri dev` once first.

**Build both targets.** A macOS developer never compiles `midi-windows`, and a Windows developer never
compiles `midi-macos`. Whichever platform you are on, the other adapter is untouched by your local
build, so the feature is not verified until someone has built both.

---

## Verification walkthrough

Each step names the spec item it closes. Run the whole list on Windows; run the macOS column to
confirm nothing regressed.

### Windows — real input and real ports

| # | Do this | Expect | Closes |
|---|---|---|---|
| 1 | Attach a device, launch, expand Sources | Every MIDI input port Windows reports is listed under `MIDI sources`, named as Windows names it | US2·1, FR-007 |
| 2 | Compare the list against another MIDI application on the same machine | Matches name for name and count for count | SC-002 |
| 3 | Tick the device and play a note | A row appears within a quarter second: device name, `Note On`, correct channel, real note and velocity | US1·1, SC-001 |
| 4 | Stop touching the device and wait five minutes | The list does not grow by a single row | US1·2, SC-003 |
| 5 | Play and release a chord | Every Note On and Note Off in transmission order, one row each, none merged or reordered | US1·4 |
| 6 | Send a System Exclusive dump | One row, true byte count, bytes exactly as transmitted | US1·5, FR-024 |
| 7 | Untick a message type in the Filter panel | Matching rows stop appearing | US1·6, FR-028 |
| 8 | Detach every device and relaunch | Empty list stating no MIDI devices were found; every control still operable | US2·2, SC-004 |

### Windows — hot-plug and persistence

| # | Do this | Expect | Closes |
|---|---|---|---|
| 9 | With the app running, attach a device | It appears within two seconds, no restart, no manual refresh | US4·1, SC-005 |
| 10 | Unplug a ticked, transmitting device | The app keeps running, other sources keep being monitored, the row goes | US4·2, FR-012 |
| 11 | Read rows produced before the unplug | Still listed, still correctly attributed | US4·7, FR-013 |
| 12 | Reattach it | Returns **still ticked**; events resume with no re-ticking | US4·3, FR-014 |
| 13 | Quit and relaunch | Same device ticked again | US4·4, FR-017 |
| 14 | With two ports sharing a display name, tick one and relaunch | The same one is ticked; the other is not | US4·5, FR-020 |
| 15 | Unplug a ticked device, quit, relaunch later with it attached | Ticked — the remembered selection was kept | US4·6, FR-018 |
| 16 | Repeat 12 and 13 ten times | Ticked on all ten | SC-006 |
| 17 | Open a port in another program, then tick it here | Listed, still tickable, states another program is using it | US2·6, FR-015 |
| 18 | Close that other program | Monitoring begins with no re-ticking | FR-016 |

### Windows — the two platform decisions

| # | Do this | Expect | Closes |
|---|---|---|---|
| 19 | Find `Act as a destination for other programs` | Present, verbatim label, reference position, checkbox visible | US5·1, FR-031 |
| 20 | Try to switch it on | It does not switch on, and states that Windows provides no built-in way for an application to publish a MIDI destination | US5·2, FR-032 |
| 21 | Read that statement | It names the alternative — a loopback port, whose ports appear under `MIDI sources` | US5·3, FR-033 |
| 22 | Watch the table for a full session | No event is ever attributed to that row | US5·5, FR-034 |
| 23 | With a loopback port present, expand Sources | It appears under `MIDI sources` as an ordinary tickable port | US5·6 |
| 24 | Look at the event table with `Data` visible | A statement explains that Windows delivers short messages already assembled and that System Exclusive bytes are exact | US6·1, FR-037 |
| 25 | Hide the `Data` column, then show it again | The statement goes and returns with the column | US6·5, FR-038 |
| 26 | Find `Spy on output to destinations` | Present, empty, stating unavailability — exactly as before | FR-048 |

### macOS — no regressions

| # | Do this | Expect | Closes |
|---|---|---|---|
| 27 | Re-run every acceptance scenario from feature 002 | All still pass, unmodified | US3·1, SC-007 |
| 28 | Launch with settings written by the previous version | Loads and applies exactly as before; nothing reset | US3·2, FR-050 |
| 29 | Play a device that transmits with running status; read `Data` | The bytes off the cable, headless as before — no restored status byte | US3·3, FR-041 |
| 30 | Look for the byte-fidelity statement | **Absent** — on macOS it is not true | US6·2, FR-038 |
| 31 | Tick `Act as a destination for other programs` | The monitor still appears in other applications' destination lists and lists what they send | US3·4, FR-036 |
| 32 | Compare the window against each reference screenshot | Labels, control types, order, indentation, default states all match | US3·5, FR-045 |

### Cross-platform

| # | Do this | Expect | Closes |
|---|---|---|---|
| 33 | Capture the same SysEx dump from the same device on both machines | `Data` content identical byte for byte | SC-008 |
| 34 | Play with running status on both; compare rows | Windows rows carry the restored status byte, macOS rows do not; neither shows an invented byte | US6·4 |
| 35 | Compare each window against its reference screenshot | Matches on both, differences limited to native chrome, font, and control rendering | SC-011 |
| 36 | Copy a macOS settings file onto the Windows machine and launch | Loads; its keys match no local port; nothing crashes and nothing is discarded | Spec edge case |

---

## Definition of done

From the constitution's Development Workflow, applied to this feature:

- [ ] `cargo clippy --all-targets -- -D warnings` clean **on both platforms**
- [ ] `cargo fmt --check` clean; webview formatter clean
- [ ] `npm run typecheck` clean under `strict`
- [ ] The app builds, launches, and is exercised by hand on both platforms — the table above
- [ ] Every new or modified public item and module carries a doc comment explaining *why*
- [ ] No test artifacts introduced
- [ ] No dead code, no unnamed magic values, no `unwrap`/`expect`/`any` outside `main`
- [ ] Every `unsafe` block documents the invariant that makes it sound
- [ ] IPC change regenerated `src/bindings.ts` from the Rust declarations — not hand-edited
- [ ] Side-by-side screenshot comparison done on both platforms, with each platform-forced deviation
      documented at the code with its reason
- [ ] No platform conditional anywhere except `src-tauri/src/platform.rs` and the two adapter crates'
      `#![cfg(target_os = …)]` attributes
- [ ] `README.md` no longer says the application is macOS-only
