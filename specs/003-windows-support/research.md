# Phase 0 Research: Windows Support

**Feature**: `003-windows-support` | **Date**: 2026-08-28

Every decision below was verified against published documentation or published source, not recalled.
Each carries the link that settles it. Three questions were required to be settled before design (D1,
D2, D3); the remainder fell out of them.

---

## D1: Which Windows MIDI API preserves the byte stream — classic WinMM, used deliberately

**Decision**: Use the classic Windows multimedia MIDI input API — `midiInOpen` with
`CALLBACK_FUNCTION`, handling `MIM_DATA`, `MIM_LONGDATA`, `MIM_ERROR`, and `MIM_LONGERROR` — reached
through the official `windows` crate. Do **not** use Windows MIDI Services, and do **not** use
`midir`.

**What Windows is actually promised, stated plainly** (this is the answer the spec's Decision 2
demanded):

| What arrives | What Windows gives us | What the user is promised |
|---|---|---|
| Short messages (Note, Control, Program, Pitch, System Common/Real-Time) | A packed doubleword: status byte, data 1, data 2 — **running status already expanded by the system** | The complete message Windows delivered. Never a byte we invented. Not necessarily the byte count the cable carried. |
| System Exclusive | A real byte buffer (`MIM_LONGDATA`, `MIDIHDR.dwBytesRecorded`) | Byte-for-byte exactly as transmitted — identical to macOS. |
| Bytes forming no valid message | `MIM_ERROR`, with **the invalid message itself packed into `dwParam1`** | Listed under `Invalid` with the bytes Windows supplied. |
| Invalid or incomplete SysEx | `MIM_LONGERROR`, with the buffer | Listed under `Invalid` / reported incomplete, with its bytes. |

**Rationale**:

The running-status question is settled by Microsoft's own wording on the `MIM_DATA` page:

> "MIDI messages received from a MIDI input port have running status disabled; each message is
> expanded to include the MIDI status byte."

and

> "This message is not sent when a MIDI system-exclusive message is received."

So the loss is real, bounded, and precisely characterised: **short messages only, and only the
omitted status byte**. It is not a loss of content — the message and every data byte are intact. This
is what makes the spec's Decision 2 honest rather than a hedge: nothing is dropped and nothing is
invented; only the wire's compression is undone before we can see it.

The compensating discovery is important and improves on the spec's floor. `MIM_ERROR` is documented
as carrying "the invalid MIDI message that was received, packed into a doubleword value with the
first byte of the message in the low-order byte", and `MIM_LONGERROR` carries the offending SysEx
buffer. **Windows does hand over the malformed bytes.** Spec FR-027 allowed for a row saying "the
bytes were not made available"; in practice that fallback is needed only for the residual ambiguity
noted under *Known limitations* below, not as the normal case.

**Alternatives considered and rejected**:

- **Windows MIDI Services** (the new MIDI 2.0 stack). Its overview states: *"We translate between MIDI
  1.0 byte format and MIDI Universal MIDI Packet (UMP) — inside the service, all messages are UMP."*
  Reaching the original byte stream from UMP words means translating back, which is exactly the
  reconstruction that research D1 of feature 002 rejected on the macOS side when it declined
  CoreMIDI's MIDI 2.0 receive path. Rejecting it here too is the consistent answer, not a new one. It
  is also unavailable on the platform floor this feature commits to — see D2.
- **WinRT `Windows.Devices.Midi`**. Delivers already-parsed `IMidiMessage` objects. It is the same
  post-driver view as WinMM with an extra abstraction on top, and it surfaces no equivalent of
  `MIM_ERROR` — so the `Invalid` category, which is the whole reason a monitor exists, would go dark.
  Rejected on the same ground as `midir` below. It is still used for nothing here; D3 shows WinMM
  alone covers identity.
- **`midir`**. Rejected again, for the same reason and with the same kind of evidence as feature 002's
  D1 — this time from its WinMM backend rather than its CoreMIDI one. In
  `src/backend/winmm/handler.rs` it: does not handle `MM_MIM_ERROR` at all; rejects any short message
  whose status bit is clear (`if status & 0x80 == 0 { return; }`); and on `MM_MIM_LONGERROR` requeues
  the buffer while **silently discarding the data**, which never reaches the user callback. A monitor
  built on it could not satisfy FR-027. `midir` also offers no hot-plug notification, which is a
  second, independent disqualification (see D4).

**Sources**:
- [MIM_DATA message (Microsoft Learn / MicrosoftDocs win32)](https://github.com/MicrosoftDocs/win32/blob/docs/desktop-src/Multimedia/mim-data.md)
- [MIM_ERROR message (Microsoft Learn)](https://learn.microsoft.com/en-us/windows/win32/multimedia/mim-error)
- [MIM_LONGERROR message (Microsoft Learn)](https://learn.microsoft.com/nb-no/windows/win32/multimedia/mim-longerror)
- [midiInOpen function (Microsoft Learn)](https://learn.microsoft.com/en-us/windows/win32/api/mmeapi/nf-mmeapi-midiinopen)
- [Windows MIDI Services Overview](https://microsoft.github.io/MIDI/overview/)
- [midir WinMM input handler source](https://raw.githubusercontent.com/Boddlnagg/midir/master/src/backend/winmm/handler.rs)

### Known limitations recorded honestly

1. **Running status is expanded before we see it.** Consequence: the domain decoder's running-status
   branch is *inert* on Windows. It is not removed — it is correct, exercised on macOS, and would be
   exercised on Windows the moment a delivery path exposed the wire stream. Recorded, not worked
   around.
2. **`MIM_ERROR` length is ambiguous.** The packed doubleword carries the invalid message with the
   first byte in the low-order byte, but not how many of the four bytes are meaningful. Where the low
   byte is a recognisable status byte its length is implied and the bytes are reported; where it is
   not, only the byte Windows placed first is reported and the row says the remainder was not
   determinable. This is the residual case spec FR-027 already provides for.
3. **Port names are truncated by Windows.** `MIDIINCAPSW.szPname` is `MAXPNAMELEN` (32) characters,
   so long device names arrive clipped. This is Windows' own truncation, passed through unmodified as
   FR-007 requires — we do not extend, pad, or "repair" it. Microsoft acknowledges the resulting
   naming divergence in [microsoft/MIDI issue #522](https://github.com/microsoft/MIDI/issues/522).
   Recovering the untruncated friendly name would mean a device-property-store lookup keyed on the
   interface string from D3; that is a possible later refinement and is **not** in scope here.

---

## D2: An application-published virtual destination — not achievable on the platform floor

**Decision**: Do not attempt it. The `Act as a destination for other programs` row stays on screen
with its verbatim label and reports itself unavailable on Windows, exactly as the `SpyOnOutput` group
already does in `crates/midi-macos/src/source.rs`. This confirms the spec's Decision 1.

**Rationale**: Classic WinMM and WinRT MIDI 1.0 both offer input and output *consumption* only — no
API by which an application publishes an endpoint other programs can enumerate. The only driver-free
route is Windows MIDI Services, which does now include, in Microsoft's words, *"a built-in app-to-app
virtual MIDI 2.0 transport"* and *"built-in loopback support, so that apps can communicate with each
other … without any additional drivers or installs."*

That is genuinely new, and it still does not change the decision:

- Its in-box components began reaching **retail Windows 11 24H2 and 25H2** through a staged optional
  rollout in **late January / early February 2026** (KB5074105 and related Release Preview packages),
  *"may not be immediately visible on every device"*.
- The spec's platform floor is **Windows 10 1809+ and Windows 11**. Building the row on a component
  absent from Windows 10 entirely, and rolling out gradually even on Windows 11, would make it work
  for a minority and fail silently for everyone else — the outcome the feature explicitly forbids.

So the row reports unavailability, and the reason text names the alternative. Two forms of that
alternative now exist and the wording should cover both: on machines carrying Windows MIDI Services,
built-in loopback endpoints already appear as ordinary MIDI ports with nothing to install; elsewhere,
a third-party loopback utility provides the same thing. Either way those ports appear under
`MIDI sources` and are monitored like any other — spec FR-033.

> **Recommended spec refinement (non-blocking)**: FR-033 currently says the statement names
> "installing a MIDI loopback utility". Research shows a no-install form exists on recent Windows 11.
> Suggest broadening the wording to cover both. This makes the promise *more* true, contradicts
> nothing, and is left to the user's call rather than changed unilaterally.

**Revisit condition**: when the platform floor rises to Windows 11 with Windows MIDI Services
present as a guarantee rather than a probability, Decision 1 should be reopened. It is a decision
about what Windows can be made to do today, not a preference.

**Sources**:
- [Windows MIDI Services Overview](https://microsoft.github.io/MIDI/overview/)
- [Making music with MIDI just got a real boost in Windows 11 (Windows Experience Blog, 2026-02-17)](https://blogs.windows.com/windowsexperience/2026/02/17/making-music-with-midi-just-got-a-real-boost-in-windows-11/)

---

## D3: The stable device identifier — the device-interface string, via `DRV_QUERYDEVICEINTERFACE`

**Decision**: Identity is the **device-interface string**, obtained with `midiInMessage` and the
`DRV_QUERYDEVICEINTERFACESIZE` / `DRV_QUERYDEVICEINTERFACE` message pair. `SourceKey` gains a
`DeviceInterface(String)` variant. The existing `Endpoint(i32)` variant is left untouched.

**Rationale**: Windows offers no signed integer equivalent to CoreMIDI's unique-id property. The
WinMM device index is positional and shifts as devices come and go, so persisting it would restore
yesterday's choices onto today's numbering — precisely what `SourceKey`'s doc comment already warns
against for `SourceId`. The device-interface string is the documented stable handle:

> "The `DRV_QUERYDEVICEINTERFACE` message queries for the device-interface name of a waveIn, waveOut,
> midiIn, midiOut, or mixer device… the function writes a null-terminated Unicode string containing
> the device-interface name… If the device has no device interface, the string length is zero."

Microsoft's own Windows-music dev blog confirms this is the intended WinMM pattern, and confirms it
doubles as the hot-plug handle:

> "With WinMM MIDI 1.0, you had to use `DRV_QUERYDEVICEINTERFACE` to get the device id related to a
> port, and then register for add/remove notifications on that device interface."

It is also the same string WinRT exposes as `DeviceInformation.Id`, so the choice does not paint the
adapter into a corner if enumeration ever moves.

**Why a new variant rather than changing `Endpoint`**: three reasons, in order of weight.

1. **Persisted settings stay readable.** `SourceKey` derives `Serialize`/`Deserialize` and is written
   into `PersistedSettings.selected_sources`. Serde's externally-tagged default means existing macOS
   files contain `{"Endpoint": 123}`; adding a variant leaves those files valid and loading unchanged.
   Widening `Endpoint` itself would invalidate every macOS settings file — a direct FR-050 violation.
2. **Each platform's key says what it actually is.** A shared "opaque identity" field would force one
   platform to stringify a number or the other to hash a string. Both are lies about the value.
3. **Exhaustive matching does the migration.** Adding a variant makes the compiler enumerate every
   site that must handle it — the constitution's stated reason for forbidding catch-all arms.

**Cost, recorded in the plan's Complexity Tracking**: `SourceKey` currently derives `Copy`; a variant
holding a `String` cannot. Every use becomes `Clone` plus explicit `.clone()`. There are 14 use sites
across `midi-core` and `midi-macos` (enumerated in the plan). The change is mechanical and
compiler-driven.

**Effect on the generated TypeScript bindings: none.** `SourceKey`'s own doc comment states it never
crosses the IPC boundary, and this was verified — `grep -rn SourceKey src-tauri/` returns nothing.
The webview keys rows on `SourceId`, which is untouched.

**Fallback when the string is empty** (documented as possible: "If the device has no device
interface, the string length is zero"): fall back to the port name, and where two present ports share
that name with nothing else to distinguish them, apply the remembered selection to neither and say
so. This is spec FR-020, already specified; research confirms the case is real rather than
hypothetical.

**Sources**:
- [DRV_QUERYDEVICEINTERFACE Function (Microsoft Learn)](https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/drv-querydeviceinterface)
- [midiInMessage function (Microsoft Learn)](https://learn.microsoft.com/en-us/windows/win32/api/mmeapi/nf-mmeapi-midiinmessage)
- [Windows MIDI Services and the 10-MIDI driver limit (Microsoft devblogs)](https://devblogs.microsoft.com/windows-music-dev/windows-midi-services-and-the-10-midi-driver-limit/)

---

## D4: Hot-plug — `CM_Register_Notification`, no window and no message pump

**Decision**: Register for device-interface arrival and removal with `CM_Register_Notification`
(Windows 8 and later). Its callback feeds the **existing `Debouncer`, unchanged**, which triggers a
full rescan — structurally identical to how the CoreMIDI notification callback works today.

**Rationale**: The obvious route, `RegisterDeviceNotification` + `WM_DEVICECHANGE`, requires a window
handle, which would mean owning a message-only window and a message pump thread purely to hear about
cables. Microsoft's own documentation points away from it:

> "You can use `CM_Register_Notification` instead of `RegisterDeviceNotification` if your code targets
> Windows 8 or newer versions of Windows. The advantage of `CM_Register_Notification` is that it does
> not require a window handle to work."

The spec's floor is Windows 10 1809+, comfortably above that. This keeps the adapter self-contained
— it borrows nothing from the Tauri window, exactly as the macOS adapter borrows nothing from it.

**The `Debouncer` is reused verbatim.** Its reason for existing is identical on both platforms: one
physical connection produces a burst of notifications, and rebuilding the Sources panel per
notification makes it flicker. Its 150 ms window and generation-counter design are platform-free. It
must, however, **move** to be reachable from two adapters — see the plan's Complexity Tracking entry
on placement. Its code does not change.

**Filtering**: register a `CM_NOTIFY_FILTER` of type `CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE`. Every
notification kind is treated the same way — re-read the truth from the system — mirroring the comment
already in the macOS adapter: reacting to specific kinds would mean trusting the notification to
describe the change completely, and a rescan is cheap enough that trusting it buys nothing.

**One item to confirm at implementation** (does not affect the design): whether to pass the
all-interface-classes flag or filter on the audio/MIDI device interface class GUID. Both reach the
same rescan; the flag is simpler, a GUID filter is quieter. Start with the flag, narrow only if the
rescan rate proves noisy.

**Sources**:
- [RegisterDeviceNotificationW function (Microsoft Learn)](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerdevicenotificationw)
- [CM_Register_Notification (Microsoft Learn)](https://learn.microsoft.com/en-us/windows/win32/api/cfgmgr32/nf-cfgmgr32-cm_register_notification)

---

## D5: The callback restriction that shapes the adapter's threading

**Decision**: The WinMM input callback does the minimum — timestamp the arrival and hand the bytes to
a worker thread through a channel. Decoding, sink calls, and **all** further multimedia calls happen
on that worker thread.

**Rationale**: `MidiInProc`'s documented constraint is specific and load-bearing:

> "Applications should not call any multimedia functions from inside the callback function, as doing
> so can cause a deadlock. Other system functions can safely be called from the callback."

Allocating and locking are therefore permitted — but `midiInAddBuffer`, which a System Exclusive
transfer requires in order to return its buffer to the driver, is a multimedia function and must not
be called from the callback. That single fact forces the hand-off. Having forced it, doing decoding
on the worker too is free and keeps the callback short, which the Plug-and-Play guidance independently
asks for.

**Consequences the design must carry**:
- The **arrival timestamp is taken in the callback**, via the existing `Clock` port, not on the worker
  thread — otherwise queue latency would leak into displayed times (FR-029).
- Ordering is preserved because the channel is FIFO and every port feeds the same one.
- This is a genuine structural difference from the macOS adapter, which decodes inside the CoreMIDI
  callback. It is documented in the Windows adapter's module docs as required by the platform, not
  chosen.

**Source**: [MidiInProc callback function (Microsoft Learn)](https://learn.microsoft.com/en-us/previous-versions/dd798460(v=vs.85))

---

## D6: The binding layer — the official `windows` crate

**Decision**: `windows`, target-gated to Windows, in the new `midi-windows` crate only.

**Rationale**: Constitution Principle II requires reaching for a maintained library before writing
code; the alternative here is hand-declaring `extern "system"` signatures for `winmm.dll` and
`cfgmgr32.dll`, which is exactly the re-solving the principle forbids. `windows` is Microsoft's own
generated binding, generated from the API metadata.

**Coverage verified** against the published module index — all present, none missing:
`midiInOpen`, `midiInStart`, `midiInStop`, `midiInReset`, `midiInClose`, `midiInGetNumDevs`,
`midiInGetDevCapsW`, `midiInMessage`, `midiInAddBuffer`, `midiInPrepareHeader`,
`midiInUnprepareHeader`, `MIDIINCAPSW`, `MIDIHDR`, `MIM_DATA`, `MIM_ERROR`, `MIM_LONGDATA`,
`MIM_LONGERROR`, `CALLBACK_FUNCTION`, `DRV_QUERYDEVICEINTERFACE`, `DRV_QUERYDEVICEINTERFACESIZE`,
`MAXPNAMELEN` — all in `windows::Win32::Media::Multimedia`. `CM_NOTIFY_FILTER`,
`CM_NOTIFY_FILTER_TYPE` and `CM_Register_Notification` are in
`windows::Win32::Devices::DeviceAndDriverInstallation`.

**`windows` versus `windows-sys`**: `windows-sys` is the leaner choice — raw externs, structs and
constants, no COM machinery, and nothing here needs COM. Either satisfies the principle. Pin one at
implementation and record the choice in the crate's module docs; the design does not depend on which.

**Sources**:
- [windows::Win32::Media::Multimedia module index](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Media/Multimedia/)
- [windows::Win32::Devices::DeviceAndDriverInstallation module index](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Devices/DeviceAndDriverInstallation/index.html)
- [microsoft/windows-rs](https://github.com/microsoft/windows-rs)

---

## D7: `Availability::Absent` is unreachable on Windows, and that is correct

**Decision**: Leave the variant alone. The Windows adapter never constructs it.

**Rationale**: `Absent` exists because CoreMIDI keeps an endpoint listed while reporting it *offline*
— macOS's `endpoints.rs` maps that property onto the variant. Windows has no counterpart: a device
that goes away stops being enumerated, full stop.

The consequence is user-visible and worth stating so that nobody later "fixes" it: on Windows, a
remembered-but-unplugged device shows **no row at all** rather than a greyed row saying "Not
connected". That satisfies the spec rather than straining it — US4 scenario 2 asks only that the list
reflect the departure, and FR-018 keeps the selection in *settings*, where it is restored on
reattachment. Synthesising a row for a device Windows does not report would violate FR-005, which
forbids listing anything the operating system did not report.

No code change. The variant stays because macOS uses it, and exhaustive matching keeps both adapters
honest about it.

---

## D8: Testing a Windows build without MIDI hardware

**Decision**: The Windows quickstart replaces the macOS IAC Driver instructions with a loopback port
plus a sender. There is no Windows equivalent of the IAC Driver on the platform floor.

Two routes, in preference order, both documented in `quickstart.md`:

1. **Built-in loopback**, on Windows 11 machines carrying Windows MIDI Services — loopback endpoints
   exist with nothing to install (D2). Best when available.
2. **A third-party loopback utility** (loopMIDI is the common one) plus any program that can send to a
   MIDI port. Works on the whole platform floor.

`Microsoft GS Wavetable Synth`, which every Windows machine has, is deliberately **not** offered: it
is a MIDI *output*, and this application lists inputs. Suggesting it would send a tester looking for a
row that will never appear.

---

## Summary table

| Question | Decision | Confidence |
|---|---|---|
| D1 Byte-stream API | Classic WinMM (`midiInOpen`, `MIM_DATA`/`MIM_LONGDATA`/`MIM_ERROR`/`MIM_LONGERROR`); reject Windows MIDI Services (UMP) and `midir` (discards invalid data) | Settled by published docs and published source |
| D2 Virtual destination | Not achievable on the platform floor; row stays and reports unavailable, per `SpyOnOutput` precedent | Settled; revisit condition recorded |
| D3 Stable identity | Device-interface string via `DRV_QUERYDEVICEINTERFACE`; new `SourceKey::DeviceInterface(String)` variant | Settled; costs recorded in Complexity Tracking |
| D4 Hot-plug | `CM_Register_Notification` (no window handle), feeding the existing `Debouncer` unchanged | Settled |
| D5 Threading | Callback timestamps and hands off; worker thread decodes and calls multimedia functions | Settled by documented callback restriction |
| D6 Bindings | Official `windows` crate; all required items verified present | Settled; `windows` vs `windows-sys` left to implementation |
| D7 `Absent` | Unreachable on Windows by design; no code change | Settled |
| D8 No-hardware testing | Built-in loopback where present, otherwise a loopback utility plus a sender | Settled |
