# MIDI Monitor

A desktop application for macOS and Windows that displays live MIDI messages.

Connect a keyboard, controller, or MIDI interface and start playing. The application displays incoming messages in a real-time table with time, source, message type, channel, and raw byte data.

## What it does

- **Detects active MIDI ports:** Displays exact ports reported by your operating system. If no device is connected, the list shows as empty.
- **Supports hot-plugging:** Automatically updates device lists when hardware is connected or disconnected. Preserves selected preferences when a device reconnects.
- **Shows all incoming data:** Displays complete, incomplete, and malformed MIDI messages for thorough debugging.
- **Acts as a MIDI destination (macOS only):** Enable *Act as a destination for other programs* to receive MIDI directly from other applications.
- **Filters incoming messages:** Filter data by source, type, channel, or hexadecimal prefix. Customize visible columns and row buffer limits.
- **Pauses data capture:** Use `Pause` to freeze the display without losing stored rows. Incoming messages sent during a pause are not recorded.
- **Manages filter rules:** Add custom prefix rules under *Data starts with* using `Show only` or `Hide`. Invalid, conflicting, or duplicate rules are automatically rejected with an explanation.
- **Sends MIDI messages:** Open the secondary panel to transmit 15 built-in preset requests (e.g., `Note On`, `All Notes Off`) or create custom messages with manual channel and value settings. Includes an outgoing activity log to easily resend past messages.
- **Publishes a virtual source (macOS only):** Enable *Publish a source other programs can receive from* to broadcast custom MIDI outputs to other music software.

> **Note:** Monitoring outgoing traffic from other software is not currently supported.

---

## Platform Differences

All platform limitations are clearly indicated inside the user interface.

### 1. Destination Mode (macOS only)
*Act as a destination for other programs* is unavailable on Windows because the operating system requires a custom driver to create virtual destinations. On Windows, use a third-party MIDI loopback utility to route software traffic into the monitor.

### 2. Message Format
Windows automatically expands running status bytes before passing data to applications. The byte count displayed on Windows reflects what the operating system delivers rather than raw cable traffic. System Exclusive (SysEx) messages are identical across both platforms.

### 3. Virtual Sources (macOS only)
Windows cannot publish virtual sources without custom system drivers. On Windows, select a port from a third-party loopback utility to send data to other applications.

*Publishing a virtual source does not configure external applications automatically. Target software must be configured manually to listen to the port.*

---

## Requirements

Requires **Rust (stable)** and **Node.js 20+** on both platforms.

### macOS
- Xcode Command Line Tools

### Windows
- Windows 10 (1809+) or Windows 11 (64-bit)
- MSVC Rust toolchain
- Microsoft C++ Build Tools (*Desktop development with C++*)
- WebView2 Runtime (included in Windows 11 and recent Windows 10 updates)

*Bluetooth MIDI devices must be paired in Windows Settings before they appear in the app.*

---

## Testing Without MIDI Hardware

- **macOS:** Open **Audio MIDI Setup**, go to *Window → Show MIDI Studio → IAC Driver*, and set the device to **Online** to create a test port.
- **Windows:** Use built-in Windows 11 loopback endpoints or install a loopback tool like **loopMIDI**.

> Do not use `Microsoft GS Wavetable Synth`. It is an output device and will not appear in input lists.

---

## Development Setup

### Run the App
```bash
npm install
npm run tauri dev
