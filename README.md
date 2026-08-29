# MIDI Monitor

A desktop app for macOS and Windows that shows the MIDI messages arriving on your machine, live.

Connect a keyboard, a control surface, or any MIDI interface. Play it, and every message
appears in a table: the time, the source, the message type, the channel, and the raw bytes.

## What it does

- **Lists your real MIDI ports.** The Sources list matches what your operating system
  reports. Nothing is invented. If no device is attached, the list is empty and says so.
- **Follows devices as you plug and unplug them.** The list updates while the app runs.
  Your selection comes back when a device returns.
- **Shows what actually arrived.** Malformed bytes and incomplete messages are listed too,
  not hidden. That is the point of a monitor.
- **Receives from other apps** — macOS only. Turn on `Act as a destination for other
  programs`, and the monitor appears as a MIDI destination that other software can send to.
- **Filters the view.** By source, by message type, by channel, or by hex prefix rules. You
  can also hide columns and set how many events to keep.
- **Pauses without losing anything.** Press `Pause` and the list stops moving. Every event
  already received stays exactly where it is, however long you leave it and however fast the
  device is sending — because pausing stops the monitor *recording*, so nothing new can push
  those rows past the retention limit. The trade is that traffic arriving during a pause is
  not recorded; the row says so while it is paused.
- **Builds a list of data rules.** Under `Data starts with`, add a prefix as either
  `Show only` or `Hide`, and add as many as you need. Each rule is listed and each can be
  deleted on its own. Prefixes match nibbles, so `9` covers `90` through `9F`.
  A rule that repeats or contradicts one already listed is refused, and the message names
  both rules — the list is kept free of contradictions rather than silently picking a winner.
  One consequence follows from that and is deliberate: while a `Show only` rule is in the
  list, `Hide` rules have nothing left to remove.

Watching another app's *outgoing* traffic is not built yet. That control is on screen and
tells you it is unavailable.

## What differs between the two platforms

Every difference is stated in the window itself, on the control it affects. There are two.

**`Act as a destination for other programs` does not work on Windows.** Windows has no
built-in way for an application to publish a MIDI destination other programs can send to,
and doing it anyway would mean installing a system-wide driver. The row stays where it is,
cannot be switched on, and says so. To monitor what another program sends, install a MIDI
loopback utility — its ports show up under `MIDI sources` like any other port.

**The `Data` column shows assembled messages on Windows.** Every byte shown is a byte the
app actually received, in the order received. But Windows expands running status before any
application sees it, so a message sent without a repeated status byte reaches the app with
that byte already restored — the byte *count* is the message Windows delivered, not what the
cable carried. System Exclusive is exact on both platforms. The window says this next to the
`Data` column on Windows, and does not say it on macOS, where it is not true.

## Requirements

Rust (stable) and Node 20 or newer on both platforms, plus:

**macOS** — Xcode Command Line Tools.

**Windows** — Windows 10 (1809 or later) or Windows 11, 64-bit; the MSVC Rust toolchain;
Microsoft C++ Build Tools ("Desktop development with C++"); and the WebView2 Runtime, which
ships with Windows 11 and current Windows 10 — install it if the window never appears.

No MIDI permission prompt exists on Windows. A Bluetooth MIDI device must be paired in
Windows Settings first; until it is, Windows does not report it and the app correctly does
not list it.

### No MIDI hardware?

**macOS** — open **Audio MIDI Setup**, then *Window → Show MIDI Studio → IAC Driver*, and put
the device online. That gives you a real port to test with.

**Windows** — the IAC Driver has no Windows equivalent. Recent Windows 11 has built-in
loopback endpoints that appear under `MIDI sources` with nothing to install. Otherwise
install a MIDI loopback utility (loopMIDI is the common one) and drive it with any program
that can send to a MIDI port.

Do **not** reach for `Microsoft GS Wavetable Synth`. Every Windows machine has it and it
looks like the obvious candidate, but it is a MIDI *output*. This app lists inputs, so it
will never appear there.

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

**Build both adapters.** A macOS developer never compiles `midi-windows` and a Windows
developer never compiles `midi-macos`, so a change can look clean on one platform and break
the other. From Windows, the macOS crates can be checked without a Mac:

```bash
rustup target add x86_64-apple-darwin
cargo clippy -p midi-core -p midi-macos --target x86_64-apple-darwin -- -D warnings
```

The Tauri shell itself cannot be cross-checked this way — one of its macOS dependencies
needs a C compiler for that target — so the shell still has to be built on each platform.

## Layout

| Path | What is inside |
|---|---|
| `crates/midi-core` | Domain and application logic. No platform code. |
| `crates/midi-macos` | The CoreMIDI adapter. Real device access on macOS. |
| `crates/midi-windows` | The WinMM adapter. Real device access on Windows. |
| `src-tauri` | The Tauri shell. Commands, events, settings. |
| `src` | The React user interface. |
| `specs` | Feature specs, plans, and manual test steps. |
