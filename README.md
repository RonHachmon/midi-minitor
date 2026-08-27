# MIDI Monitor

A desktop app for macOS that shows the MIDI messages arriving on your machine, live.

Connect a keyboard, a control surface, or any MIDI interface. Play it, and every message
appears in a table: the time, the source, the message type, the channel, and the raw bytes.

## What it does

- **Lists your real MIDI ports.** The Sources list matches what macOS reports. Nothing is
  invented. If no device is attached, the list is empty and says so.
- **Follows devices as you plug and unplug them.** The list updates while the app runs.
  Your selection comes back when a device returns.
- **Shows what actually arrived.** Malformed bytes and incomplete messages are listed too,
  not hidden. That is the point of a monitor.
- **Receives from other apps.** Turn on `Act as a destination for other programs`, and the
  monitor appears as a MIDI destination that other software can send to.
- **Filters the view.** By source, by message type, by channel, or by a hex prefix. You can
  also hide columns and set how many events to keep.

Watching another app's *outgoing* traffic is not built yet. That control is on screen and
tells you it is unavailable.

## Requirements

- macOS
- Xcode Command Line Tools
- Rust (stable) and Node 20 or newer

No MIDI hardware? Open **Audio MIDI Setup**, then *Window → Show MIDI Studio → IAC Driver*,
and put the device online. That gives you a real port to test with.

## Run it

```bash
npm install
npm run tauri dev
```

The app window opens. Expand **Sources**, tick a device, and play.

## Build a release app

```bash
npm run tauri build
```

## Checks

The project has no test suite. The compiler and a manual pass are the safety net.

```bash
cargo clippy --all-targets -- -D warnings
cargo fmt --check
npm run typecheck
```

`src/bindings.ts` is generated when the app starts in debug mode. If `npm run typecheck`
fails with `Cannot find module './bindings'`, run `npm run tauri dev` once first.

## Layout

| Path | What is inside |
|---|---|
| `crates/midi-core` | Domain and application logic. No platform code. |
| `crates/midi-macos` | The CoreMIDI adapter. Real device access. |
| `src-tauri` | The Tauri shell. Commands, events, settings. |
| `src` | The React user interface. |
| `specs` | Feature specs, plans, and manual test steps. |
