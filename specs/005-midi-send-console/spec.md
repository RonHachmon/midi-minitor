# Feature Specification: A Send Screen With Built-In Requests and a Chosen Identity

**Feature Branch**: `005-midi-send-console`

**Created**: 2026-08-29

**Status**: Draft

**Input**: User description: "i want another screen that through it i can send midi reuqest, if could disguise myslf as one of the sources it would be great. makig the screen really user firendly . with some build in request i could send."

## Overview

The application listens. It has never spoken. This feature gives it a second screen whose only
job is to **send** MIDI, so the same window that shows what arrives can also produce traffic to
look at — a keyboard the user does not have, a controller left at the studio, a transport
command to find out whether the other program is listening.

Four things make up the feature.

**A second screen.** Sending is a different activity from watching, with its own controls and
its own vocabulary, so it gets its own surface. The monitoring screen the reference screenshots
depict is not touched: no control on it is relabelled, reordered, restyled, or repurposed, and
the user can move between the two screens and back without losing what the monitor has retained
or changing what it is doing.

**Built-in requests.** The screen ships with a library of ready-made messages — the ones people
actually reach for when testing a MIDI setup. A user picks one and sends it. No typing, no
lookup, no manual. This is the fastest path from "is the other end listening?" to an answer.

**Composing without hexadecimal.** For anything the library does not cover, the user builds a
message from named controls — the message type by its name, the channel by its number, the
values by what they mean (note, velocity, controller, program, bend). The exact bytes that will
go out are shown before anything is sent, so the screen teaches rather than hides. Typing raw
bytes stays available for the person who already knows what they want, but it is never the only
way.

**Where it goes, and who it appears to be from.** Traffic leaves through exactly one target at a
time, and the user picks it. A target is either a MIDI destination the operating system already
reports, or — the "disguise" — **a MIDI source this application publishes under a name the user
chooses**. Other programs on the machine list that published source among their MIDI inputs and
can receive from it, so the monitor can stand in for a device that is not there.

Publishing a source is a macOS capability. Windows offers no way for an application to publish
one without installing a system-wide driver — the same limitation the existing
`Act as a destination for other programs` row already reports — so on Windows the control says so,
and sending to destinations continues to work in full. The application's rule holds unchanged: a
platform limitation reaches the user as plain text on the control it affects, and a control that
cannot do anything on this platform is never left operable-but-inert.

One thing publishing a source does **not** do is configure the program at the other end. It makes
the monitor appear as a device; teaching that program what the messages mean is still done there.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Send a ready-made request without knowing anything about MIDI (Priority: P1)

The user opens the send screen. A list of named requests is already there — `Note On`,
`All Notes Off`, `Start`, `Stop`, `Program Change`, and the rest of the everyday set. They pick
one, press send, and it goes. The name told them what it does; they did not have to know a
status byte, a channel encoding, or a value range to use it.

**Why this priority**: This is the shortest path to the feature's value and it stands entirely
alone. With only this built, someone can prove their MIDI chain works end to end. It is also the
part the request named explicitly ("with some build in request i could send").

**Independent Test**: Open the send screen with a destination selected, choose any built-in
request, send it, and confirm the receiving end shows exactly the message the request's name
promised.

**Acceptance Scenarios**:

1. **Given** the send screen is open and a destination is chosen, **When** the user selects a
   built-in request and sends it, **Then** the message is transmitted and the screen confirms it
   was sent.
2. **Given** a built-in request is selected, **When** the user looks at the screen before
   sending, **Then** the request's name, a plain-language description of what it does, and the
   exact bytes it will send are all visible.
3. **Given** a built-in request that carries adjustable values (a note, a channel, a
   controller), **When** the user changes one of those values, **Then** the displayed bytes
   update immediately and the next send uses the changed values.
4. **Given** no destination has been chosen, **When** the user tries to send, **Then** the
   screen explains that a destination is needed and nothing is transmitted.
5. **Given** the user has sent a built-in request, **When** they send the same request again,
   **Then** it is sent again with the same values, with no re-entry of anything.

---

### User Story 2 - Build a message from named controls rather than hexadecimal (Priority: P1)

The user needs something the library does not cover: a specific control change on a specific
channel with a specific value. They choose the message type by name, set the channel, set the
values using controls labelled with what those values mean, watch the byte preview change as
they do, and send. At no point were they required to type a hexadecimal digit.

**Why this priority**: "Really user friendly" was an explicit requirement, and the library alone
cannot cover the whole message set. This is what makes the screen usable by someone who does not
already know the protocol — the difference between a send screen and a hex terminal.

**Independent Test**: Compose a control change on a chosen channel with a chosen value using
only the named controls, send it, and confirm the receiving end shows exactly that message.

**Acceptance Scenarios**:

1. **Given** the send screen, **When** the user chooses a message type by name, **Then** only
   the value controls that message actually carries are shown, each labelled with what it means.
2. **Given** a message being composed, **When** any value or the channel changes, **Then** the
   byte preview updates immediately to show precisely what would be sent.
3. **Given** a value control, **When** the user attempts to set a value outside what the message
   permits, **Then** the value is prevented or corrected at entry and the screen states the
   permitted range; an out-of-range message is never transmitted.
4. **Given** a composed message, **When** the user sends it, **Then** the bytes transmitted are
   byte-for-byte the bytes the preview showed.
5. **Given** a user who prefers to enter raw bytes, **When** they use the raw entry, **Then**
   they can send arbitrary bytes, and anything that is not a valid message is refused with an
   explanation before transmission rather than sent blindly.
6. **Given** the user moves to the monitoring screen and returns, **When** the send screen
   reopens, **Then** what they were composing is still there.

---

### User Story 3 - Appear to other programs as a MIDI device of your own naming (Priority: P2)

The user turns on the disguise and gives it a name. Other software on the machine — a DJ
application, a DAW, anything that lists MIDI inputs — now shows that name among its devices and
can receive from it. The user picks it as the send target, sends, and the other program receives
the traffic as though a controller of that name were plugged in.

**Why this priority**: It is the "would be great" half of the request rather than the "i want"
half: sending to a real destination works and is useful without it. It also carries a platform
constraint that must not be allowed to gate the two stories above.

**Independent Test**: Publish the source with a chosen name, open any other application that
lists MIDI inputs, confirm the name appears there, send a message, and confirm that application
receives it.

**Acceptance Scenarios**:

1. **Given** the send screen, **When** the user publishes the source under a name, **Then** other
   applications on the machine list a MIDI source under that name and can receive from it.
2. **Given** the source is published, **When** the user selects it as the target and sends,
   **Then** an application receiving from it gets exactly the message that was sent.
3. **Given** the source is published, **When** the user stops publishing it, **Then** it
   disappears from other applications' device lists, and sending to MIDI destinations is
   unaffected.
4. **Given** a published source with a chosen name, **When** the application is quit and
   relaunched, **Then** the source is published again under the same name.
5. **Given** a published source, **When** the user changes its name, **Then** they are warned
   first that a receiving program keeps its settings against the name it saw, and on confirming,
   the source appears under the new name.
6. **Given** Windows, **When** the user opens the send screen, **Then** the publish control states
   that it is unavailable on this platform and what to do instead, cannot be switched on, and
   every other part of the send screen works.

---

### User Story 4 - See what was sent next to what arrived (Priority: P2)

Having sent something, the user wants proof. The screen keeps a record of what it has sent —
each entry with its time, what it was, and the exact bytes — so a send that produced no reaction
downstream can be told apart from a send that never happened.

**Why this priority**: Without it, a silent failure and a successful send that the other end
ignored look identical, which is exactly the confusion a monitor exists to remove. It depends on
sending existing, so it follows the stories above.

**Independent Test**: Send several messages and confirm each one is listed with its time and its
bytes, and that a failed send is listed as failed rather than omitted.

**Acceptance Scenarios**:

1. **Given** a message has been sent, **When** the user looks at the send screen, **Then** the
   send is listed with the time, what was sent, and the bytes.
2. **Given** a send fails, **When** the user looks at the send screen, **Then** the failure is
   listed with the reason, and it is distinguishable from a successful send.
3. **Given** a listed send, **When** the user chooses to send it again, **Then** the identical
   bytes are transmitted without recomposing anything.
4. **Given** the monitoring screen is watching a source that receives what this screen sends,
   **When** a message is sent, **Then** it appears in the monitor's event list under the identity
   in force, subject to the monitor's filters exactly as any other event is.

---

### User Story 5 - Keep the requests used often (Priority: P3)

The user has composed a message they will need again. They save it under a name of their own,
and it joins the list beside the built-in requests. They can rename and delete what they saved.
The built-in requests cannot be lost.

**Why this priority**: Genuinely valuable for repeat use, and entirely additive. Every earlier
story is complete and shippable without it.

**Independent Test**: Save a composed message, relaunch the application, and confirm it is still
listed and still sends the same bytes.

**Acceptance Scenarios**:

1. **Given** a composed message, **When** the user saves it with a name, **Then** it appears in
   the request list alongside the built-in requests and is marked as the user's own.
2. **Given** a saved request, **When** the application is quit and relaunched, **Then** it is
   still listed and sends the same bytes.
3. **Given** a saved request, **When** the user deletes it, **Then** it is gone and no built-in
   request is affected.
4. **Given** the request list, **When** the user attempts to delete or overwrite a built-in
   request, **Then** the built-in library is left intact.

---

### Edge Cases

- **No destination exists at all.** A machine with nothing to send to must say so on the send
  screen, in the same honest way the empty Sources list already does, rather than showing an
  empty picker with no explanation.
- **The chosen destination disappears mid-session.** Unplugging the device, or the other program
  closing its port, must be reported on the next send attempt with a reason — not swallowed, and
  not reported as a malformed message.
- **Windows, where a source cannot be published.** The control is unavailable and says why, and
  sending to MIDI destinations still works in full. Losing the disguise must never cost the user
  the ability to send. The screen names the way through — a MIDI loopback utility, whose port both
  this application and the receiving program can use — rather than leaving the user stuck.
- **The receiving program lists the published source but ignores what arrives.** This is the first
  confusion a user will hit, and it is not a fault: most programs must be told to listen to a
  device and taught what each message means before anything happens. The send screen must not
  claim a delivery it cannot observe — it reports that the message was transmitted, which is what
  it actually knows.
- **Renaming the published source.** A receiving program remembers its settings against the device
  name it saw. Changing the name presents that program with a new, unconfigured device. The user
  must be warned before the rename, not left to discover it when their mapping stops working.
- **The chosen name matches a device already on the machine.** Permitted — that is what a disguise
  is — but two entries then carry one name everywhere, including in this monitor's own Sources
  list. The user must be able to tell which is which.
- **The source is published with nothing receiving from it.** Not an error and not reported as
  one. A published source with no subscriber is an ordinary state; messages sent to it reach
  nobody, and the screen does not pretend otherwise.
- **Sending to a source the monitor is watching.** The sent message arrives back in the event
  list. That is the intended way to exercise the filters, and it must not be mistaken for a loop
  or suppressed as an echo.
- **Sending while the monitor is paused.** Sending is unaffected by pause; pause governs what is
  retained, not what is transmitted. Traffic sent during a pause is not retained, for the same
  reason no other traffic is.
- **A held note.** Sending a `Note On` and never its `Note Off` leaves a note sounding on the
  receiving instrument. Releasing it must not require hunting — this is what a panic or
  all-notes-off request is for.
- **Rapid repeated sending.** Repeatedly triggering a send must not queue an unbounded backlog
  or leave the send screen unresponsive.
- **A raw entry that is not a valid message.** Refused with an explanation before transmission.
  Note the asymmetry with the monitor, which deliberately *displays* invalid data it receives:
  showing what arrived malformed is the point of a monitor; emitting malformed bytes on purpose
  is not the point of a send screen.
- **System Exclusive of significant length.** A long message must either send whole or fail with
  a reason. A partial transmission reported as success is the one outcome worse than failing.
- **The send screen open with no sources selected on the monitor.** The two screens are
  independent: sending does not require anything to be monitored, and monitoring does not require
  anything to be sendable.

## Requirements *(mandatory)*

### Functional Requirements

#### The Second Screen

- **FR-001**: The application MUST offer a second screen dedicated to sending MIDI, reachable
  from the existing window without disturbing what the monitor is doing.
- **FR-002**: Moving to the send screen and back MUST NOT clear retained events, change source
  selections, change filters, or change whether the monitor is paused.
- **FR-003**: The monitoring screen's controls that the reference screenshots depict MUST keep
  their labels, order, control types, grouping, and default states. This feature adds surface; it
  changes none of the depicted surface.
- **FR-004**: The user MUST be able to tell at a glance which screen they are on and how to
  return to the other one.
- **FR-005**: Monitoring MUST continue uninterrupted while the send screen is in use — no events
  dropped, no source closed and reopened, on account of the send screen being open.

#### Built-In Requests

- **FR-006**: The send screen MUST provide a library of ready-made requests, available on first
  launch with no setup.
- **FR-007**: The library MUST cover, at minimum, the everyday testing set: note on, note off,
  all notes off / panic, control change, program change, pitch bend, channel pressure, the
  transport commands (start, stop, continue), clock, active sensing, reset, and a device identity
  request as System Exclusive.
- **FR-008**: Every built-in request MUST show a name, a plain-language description of its
  effect, and the exact bytes it will transmit, before it is sent.
- **FR-009**: Where a built-in request carries values a user would reasonably vary — channel,
  note, velocity, controller number, value, program number — those MUST be adjustable on the
  screen, and the shown bytes MUST update as they are adjusted.
- **FR-010**: Sending a built-in request MUST require no more than choosing it and confirming the
  send.
- **FR-011**: The built-in library MUST NOT be modifiable or deletable by the user; it is always
  available in full.

#### Composing a Message

- **FR-012**: Users MUST be able to compose any message the application can name, choosing the
  message type by its name rather than by a numeric code.
- **FR-013**: The value controls shown MUST be exactly those the chosen message type carries — no
  control for a value the message does not have, and no missing control for one it does.
- **FR-014**: Every value control MUST be labelled with the meaning of the value, not with its
  position in the byte stream.
- **FR-015**: The screen MUST show the exact bytes that will be transmitted, updated as the
  composition changes, before any send occurs.
- **FR-016**: Values outside the range a message permits MUST be prevented or corrected at entry,
  with the permitted range stated. An out-of-range message MUST NOT be transmitted.
- **FR-017**: A raw byte entry MUST be available for users who prefer it, and MUST NOT be the
  only way to reach any capability of this screen.
- **FR-018**: Bytes entered raw that do not form a valid MIDI message MUST be refused with an
  explanation before transmission.
- **FR-019**: What the user is composing MUST survive moving between screens within a session.

#### Where Traffic Goes, and Who It Appears to Be From

- **FR-020**: Outgoing traffic MUST go to exactly one target at a time, chosen by the user. A
  target is either a MIDI destination the operating system reports, or the MIDI source this
  application publishes.
- **FR-021**: The list of available targets MUST reflect devices appearing and disappearing while
  the application runs, as the Sources list already does.
- **FR-022**: When no target is available, the screen MUST say so plainly rather than presenting
  an empty control.
- **FR-023**: The chosen target MUST persist across restarts and be recognised again after a
  device is unplugged and reattached.
- **FR-024**: Users MUST be able to publish a MIDI source under a name of their own choosing, which
  other programs on the machine list among their MIDI inputs and can receive from. The chosen name
  MUST be the name those programs see. It applies to the published source only — traffic sent to a
  MIDI destination carries whatever origin the operating system reports for this application, and
  the screen MUST NOT imply otherwise.
- **FR-025**: The name in force, and whether the source is currently published, MUST both be
  visible on the send screen without opening anything, and MUST both persist across restarts so
  the source is published again under the same name.
- **FR-026**: Users MUST be able to start and stop publishing. Stopping MUST remove the source from
  other programs' device lists and MUST NOT affect sending to MIDI destinations. Because a
  receiving program keeps its settings against the name it saw, changing the name MUST warn the
  user before it takes effect.
- **FR-027**: On a platform that cannot publish a MIDI source, the control MUST state that it is
  unavailable there and what the user can do instead, and MUST NOT be operable. Every other
  capability of the send screen MUST remain fully available on that platform.

#### Sending, and Knowing What Was Sent

- **FR-028**: Every send MUST report its outcome. A send either succeeds and says so, or fails and
  names the target and the reason. No send completes silently or ambiguously. The report MUST state
  only what the application knows — that the message was transmitted — and MUST NOT claim that a
  receiving program received or acted on it.
- **FR-029**: A failed send MUST leave the composition and the request list unchanged, so the
  user can correct and retry without re-entering anything.
- **FR-030**: The screen MUST keep a record of sends made in the session, each with its time,
  what was sent, its bytes, and whether it succeeded.
- **FR-031**: Users MUST be able to re-send any recorded send without recomposing it.
- **FR-032**: A message sent by this application that reaches a source the monitor is watching
  MUST appear in the monitor's event list like any other event, subject to the same filters, and
  MUST NOT be specially suppressed.
- **FR-033**: Sending MUST be unaffected by whether the monitor is paused. Pause governs what is
  retained, never what is transmitted.
- **FR-034**: Repeated rapid sending MUST NOT accumulate an unbounded backlog or leave the screen
  unresponsive.
- **FR-035**: A message MUST be transmitted whole or not at all; a partial transmission MUST be
  reported as a failure, never as a success.
- **FR-036**: Failures from this screen MUST be reported the same way the application's existing
  failures are, so there is one place to read what went wrong.

#### Saved Requests

- **FR-037**: Users MUST be able to save a composed message as a named request.
- **FR-038**: Saved requests MUST be listed alongside the built-in ones and MUST be
  distinguishable from them.
- **FR-039**: Users MUST be able to rename and delete their own saved requests, without affecting
  any other saved request or any built-in one.
- **FR-040**: Saved requests MUST persist across restarts and MUST send the same bytes they were
  saved with.

### Key Entities

- **Send Screen**: The second surface, dedicated to producing MIDI. Independent of the monitoring
  screen's state; neither screen's activity disturbs the other's.
- **Request**: A named, sendable message — its name, its plain-language description, its message
  type, its adjustable values, and the bytes it produces. Either built in (always present, not
  editable) or saved by the user (nameable, editable, deletable).
- **Composition**: The message currently being built on the screen — a message type, a channel
  where the type carries one, and its values. The authority for the byte preview and for what a
  send transmits. Session-scoped.
- **Send Target**: Where outgoing traffic goes — either a MIDI destination the operating system
  reports, or the published source. Exactly one is in force at a time. Destinations appear and
  disappear while the application runs; the chosen target persists across restarts.
- **Published Source**: The MIDI source this application offers to the rest of the machine — its
  chosen name, and whether it is currently published. Other programs list it among their inputs
  and receive from it. Persisted, visible on the screen, and available only where the platform can
  publish one. The name is the identity receiving programs remember, which is why changing it is a
  warned action rather than a quiet edit.
- **Send Record**: One attempted send — its time, the request or composition behind it, the exact
  bytes, and its outcome including the reason on failure. The basis for re-sending, and for
  telling a silent failure apart from an ignored message.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user who has never opened the send screen can send their first MIDI message
  within 30 seconds of opening it, without typing a single hexadecimal digit and without
  consulting any documentation.
- **SC-002**: Every built-in request can be sent in at most two interactions: select it, send it.
- **SC-003**: 100% of sends produce a visible outcome — a confirmation, or a failure naming the
  destination and the reason. Zero sends complete silently or ambiguously.
- **SC-004**: For 100% of built-in and composed requests, the bytes shown before sending are
  byte-for-byte the bytes the receiving end reports.
- **SC-005**: A sent message reaches the receiving end without perceptible delay — a user
  watching both cannot distinguish the send from the arrival.
- **SC-006**: Opening, using, or leaving open the send screen loses zero monitored events and
  changes zero monitor settings.
- **SC-007**: Zero controls on the send screen are operable but inert. Anything the current
  platform cannot do says so, on the control, before the user tries it.
- **SC-008**: Zero controls the reference screenshots depict change label, order, type, grouping,
  or default state as a result of this feature.
- **SC-009**: A user can identify what every built-in request does from its name and description
  alone, without inspecting its bytes.
- **SC-010**: A saved request sends identical bytes after an application restart, in 100% of
  cases.
- **SC-011**: With the source published on a platform that supports it, its chosen name is listed
  among the MIDI inputs of another application on the machine, and a message sent to it is received
  there — verified with a program that was never told anything about this application.
- **SC-012**: On a platform that cannot publish a source, every capability of the send screen other
  than publishing remains available, and the user can find out why publishing is unavailable and
  what to use instead without leaving the screen.

## Assumptions

Reasonable defaults chosen where the request did not specify. They are recorded so they can be
challenged now rather than discovered later.

- **The monitoring screen keeps its screenshot fidelity.** The reference images are the design
  authority for the monitoring surface only. The send screen has no reference image, so it is
  designed for usability under this specification, in the same visual language as the rest of the
  window.
- **"Another screen" means a distinct screen, not a rearranged one.** The monitor is not reduced,
  re-laid-out, or split to make room. How the user moves between the two is a design decision left
  to planning; that it must not disturb the depicted controls is not.
- **"Request" means a MIDI message to send**, in the everyday sense of "send a request to the
  device" — not a request/response transaction with a guaranteed reply. Some built-in requests (a
  device identity request) do provoke a reply, and that reply arrives through the monitor like any
  other traffic. The send screen does not wait for, correlate, or require a response.
- **Sending targets one place at a time.** Fan-out to several destinations at once is not assumed
  to be needed.
- **One published source at a time.** Publishing several differently named sources at once — so a
  receiving program, which keeps a separate mapping per device, could hold several mapping sets
  apart — is a plausible extension and is not assumed to be needed now.
- **The built-in library is a fixed set defined by this feature**, chosen for testing utility, and
  grows only by amending this specification — not by user editing (FR-011). Users extend it
  through saved requests instead (FR-037).
- **The user is sending on their own machine, to their own software and devices.** The publish
  control exists so traffic is recognisable while testing, and its ceiling is what the operating
  system's MIDI services already let any application do.
- **Configuring the receiving program is the user's job, not this application's.** Publishing a
  source makes the monitor appear as a device; most programs still require the user to enable that
  device and map each message to a function before anything happens there. This feature does not
  detect, drive, or verify that configuration, which is why success is measured at transmission
  rather than at the far end.
- **A name is not a device identity.** Programs that recognise particular hardware do so by device
  identity, not by a MIDI port's name, so the chosen name makes traffic *identifiable*, not
  *authoritative*. Anything the receiving program gates on real hardware stays gated.
- **Value ranges follow the MIDI specification** — seven-bit values 0–127, fourteen-bit bend,
  channels 1–16 as displayed — with no application-specific restriction beyond it.
- **Both platforms get the send screen; only publishing a source differs.** macOS can publish one;
  Windows cannot without a system-wide driver, which this project does not ship. That difference is
  surfaced as data on the control, consistent with how the application already reports
  `Act as a destination for other programs` on Windows. On Windows the user reaches another program
  through a MIDI loopback utility's port, which appears as an ordinary destination.
- **Persistence uses what the application already remembers across restarts** — the chosen
  target, the published source's name and whether it is published, and saved requests join the
  existing settings.

## Out of Scope

- **Sequencing and playback.** No timeline, no recording of an incoming stream for replay, no
  scheduled or looped sending beyond repeating a single request.
- **Editing or authoring MIDI files.** Nothing is loaded from or written to a `.mid` file.
- **Routing or through-put.** This feature does not forward monitored input to an output.
- **Waiting for replies.** No request/response correlation, timeout, or reply matching. Replies
  arrive in the monitor as ordinary traffic.
- **Spying on another program's output.** That control remains on screen, remains unavailable, and
  is unchanged by this feature.
- **Deliberately emitting malformed bytes.** The monitor displays invalid data it receives; the
  send screen refuses to transmit it (FR-018).
- **Configuring the program that receives the traffic.** No mapping is created, imported, detected,
  or suggested on the other side.
- **Publishing more than one named source at once.**
- **Shipping or installing a driver, or bundling a loopback utility**, to give Windows a capability
  the platform does not offer.
- **Remote or networked MIDI.** Destinations are what the local operating system reports.
