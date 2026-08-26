# Quickstart & Validation Guide: Mock MIDI Monitor

**Date**: 2026-08-26 · **Plan**: [plan.md](./plan.md) · **Spec**: [spec.md](./spec.md)

This is the project's **verification procedure**. The constitution's Verification Standard forbids
test files, test dependencies, doctests, and CI test steps — verification here is a clean compile
plus running the app and working through the scenarios below. There is no `npm test` to run, by
design.

## Prerequisites

| Requirement | Status on this machine | Notes |
|-------------|------------------------|-------|
| Node.js ≥ 20 | **Present** — v24.18.0 | verified |
| npm | **Present** — 11.16.0 | verified |
| Rust toolchain ≥ 1.77 | **MISSING** | `cargo`, `rustc`, `rustup` all absent; no `%USERPROFILE%\.cargo\bin` |
| MSVC C++ build tools | Unknown | Windows linker for Rust; `rustup` prompts for it |
| WebView2 runtime | Present by default on Windows 11 | Tauri's webview on Windows |

**Install Rust before anything else** — every gate below runs through `cargo`:

```text
winget install Rustlang.Rustup
rustup default stable
```

Open a new terminal afterwards so `PATH` picks up `~/.cargo/bin`, then confirm with
`cargo --version`. No MIDI hardware, driver, or virtual port is needed at any point (FR-004,
SC-011).

## First run

```text
npm install
npm run tauri dev
```

`beforeDevCommand` starts Vite on port 5173 and Tauri loads it; `vite.config.ts` sets
`clearScreen: false` so Rust compiler errors stay on screen, and ignores `src-tauri/**` so Vite does
not rebuild on Rust edits.

On the first debug build `tauri-specta` writes `src/bindings.ts`. If TypeScript reports that module
missing, the Rust side has not built yet — fix the Rust error first; the generated file is an
output, never something to author by hand.

## Verification gates

All four must pass before any change is done. Gate 4 is the one that cannot be automated, and is
therefore the one to actually perform rather than assume.

```text
cargo clippy --workspace --all-targets    # gate 1: zero warnings
cargo fmt --check                         # gate 2
npx tsc --noEmit                          # gate 3: strict, zero errors
npm run tauri dev                         # gate 4: run it and work the scenarios below
```

Warnings are errors here. With no test suite, `clippy` and `tsc` are the regression net, so a
tolerated warning is a hole in it.

## Scenario walkthrough

Each maps to a user story in [spec.md](./spec.md). Perform them in order; each assumes a fresh
launch unless stated.

### S1 — Live event stream (User Story 1, P1)

1. Launch. Within 10 seconds rows are appearing unprompted (**SC-002**).
2. Read any row: `Time` is `HH:MM:SS.mmm`, `Source` is a name from the Sources list, `Message` is a
   readable name, `Chan` is 1–16 or blank, `Data` matches the message type.
3. Timestamps **ascend downward** — newest at the bottom, as in `screenshots/main-screen.png`.
4. Set `Remember up to` to `20`. The list trims to 20 immediately and stays there (**SC-006**).
5. Press `Clear`. The list empties and refills.
6. Enter `0`, then `abc`, then `999999999` in the retention field. Each is rejected or normalised;
   none is silently accepted.

### S2 — Source selection (User Story 2, P2)

1. Expand `Sources`: the triangle turns `▶` → `▼` and the bordered list appears.
2. The tree matches `screenshots/sources.png` exactly — `MIDI sources` over `IAC Driver Bus 1` and
   `MidiKeys`, then `Act as a destination for other programs`, then `Spy on output to destinations`
   over `IAC Driver Bus 1`, children indented.
3. Uncheck `MidiKeys`. Its events stop **and its existing rows disappear** (FR-026). Others continue.
4. Uncheck the `MIDI sources` group: both children uncheck.
5. Re-check one child: the group shows a **mixed** state, visually distinct from checked/unchecked.
6. Uncheck everything: the app says nothing is being monitored rather than looking frozen (FR-027).
7. Time how long it takes to isolate a single named source — target under 15 seconds (**SC-003**).

### S3 — Message and channel filters (User Story 3, P3)

1. Expand `Filter`. Compare against `screenshots/filters.png`: three columns, correct headings,
   correct entries in the correct order, `System Exclusive` and `Invalid` standalone, `All Channels`
   selected, channel field inactive showing `1`, every box checked.
2. Uncheck `Real Time`: all four children uncheck; Clock and Active Sense stop. The visible rate
   drops sharply (**SC-004**).
3. Re-check `Clock` alone: the parent shows **mixed**.
4. Toggle each of the 15 leaf checkboxes once. Every one visibly changes the list (**SC-007**).
5. Re-check `Clock` after unchecking it: **retained Clock events reappear immediately** rather than
   only new ones — filters are a view over the log, not an admission gate (D-04, FR-034).
6. Select `One Channel`, enter `5`: only channel-5 rows remain, and channel-less messages (Clock,
   SysEx, Time Code) are **excluded** (FR-033).
7. Enter `0` and `17`: rejected or clamped to 1–16.
8. Back to `All Channels`: the channel field goes inactive but keeps `5`.

### S4 — Hex prefix filter (User Story 4, P4)

1. View an event's raw bytes (FR-015) to see what to type.
2. Enter `90`, mode **Include**: only rows whose raw bytes start with `90` remain.
3. Switch to **Exclude**: exactly the complement — every other row, and only those (**SC-008**).
4. Enter `9` alone: matches `90`–`9F` (nibble prefix, FR-039).
5. Enter `b0 07`, then `B007`: identical results — case and spacing are irrelevant (FR-038).
6. Enter two prefixes: an event matching **either** passes (FR-037).
7. Enter `zz`: flagged invalid with a reason, and **the previous valid filter stays in effect** —
   the list does not empty (FR-041).
8. Clear the field: nothing is hidden, in either mode (FR-040).

### S5 — Column visibility (User Story 5, P5)

1. Hide `Chan`: header and cells vanish, remaining columns reflow with no leftover gap.
2. Hide `Source` too. Show `Chan` again — it returns **between Message and Data**, its original
   position (FR-044).
3. Try to hide all five: the last one refuses (FR-045).
4. With `Chan` hidden, apply `One Channel = 5`: filtering still works — visibility is display-only
   (FR-046).

### S6 — Persistence and fidelity

1. Change sources, filters, a hex prefix, hidden columns, and the retention limit. Quit and relaunch:
   every setting is restored; the event list starts empty (**SC-010**, FR-047).
2. **Screenshot comparison** (Constitution VI, Definition of Done gate 10) — open each reference
   image beside the running window and check:
   - Labels verbatim, character for character: `Remember up to`, `events`, `Clear`, `Time`,
     `Source`, `Message`, `Chan`, `Data`, `Act as a destination for other programs`,
     `Spy on output to destinations`, `Aftertouch (Poly)`, `Start/Stop/Continue`,
     `Song Position Pointer`, `All Channels`, `One Channel`.
   - Control types: disclosure triangles (not tabs/accordions), checkboxes, a radio pair (not a
     toggle), a bordered scrollable source list.
   - Order and grouping: column order; the three filter columns and their entry order; source tree
     indentation.
   - Defaults on a first launch: both sections collapsed, all filter boxes checked, `All Channels`,
     `1000`.
   - Nothing added to the referenced surface — no extra buttons, icons, or branding. The column menu
     from S5 is additive surface and must not alter the header at rest (Constitution VII).
   Any platform-forced deviation is documented in the relevant module's docs with its reason.

### S7 — Throughput

1. With filters wide open, confirm the simulator sustains ≥500 events/s.
2. While it runs: expand and collapse sections, toggle checkboxes, scroll the list, resize the
   window. Every control responds within a quarter second and scrolling stays smooth (**SC-005**).
3. Set retention to its maximum and scroll to the top: still smooth — the list is virtualized.

## Troubleshooting

| Symptom | Likely cause |
|---------|--------------|
| `cargo: command not found` | Rust not installed — see Prerequisites |
| `Cannot find module './bindings.ts'` | Rust side has not completed a debug build yet |
| TS type errors after changing a Rust DTO | `bindings.ts` is stale — rebuild the Rust side (gate 9) |
| Blank window, Vite reachable in a browser | Port mismatch between `devUrl` and Vite's `strictPort` |
| Link errors on Windows | MSVC C++ build tools missing |
| List stutters under load | Batching or virtualization not in effect — check the ~16 ms coalescing interval |
