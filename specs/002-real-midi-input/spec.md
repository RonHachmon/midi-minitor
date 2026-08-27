# Feature Specification: Real MIDI Input

**Feature Branch**: `002-real-midi-input`

**Created**: 2026-08-26

**Status**: Draft

**Input**: User description: "currenlty the app moks data. not the next step is for it to connect to a real midi libary and display real data real sources etc.. nothing mocked"

## Overview

The monitor currently observes a simulator. This feature replaces that simulator with the machine's
actual MIDI system: the Sources list shows the ports genuinely present on the machine, and every row
in the event table is a message that really arrived from a real device or a real application.

Nothing about the monitoring experience is invented after this change. The simulated event source is
removed from the product — not disabled, not hidden behind a switch, not kept as a fallback. When no
device is connected, the honest result is an empty list and a clear statement that nothing is
attached, rather than manufactured traffic.

Two realities that a simulator was free to ignore now govern the work. First, **the set of sources is
no longer fixed**: devices are plugged in and unplugged while the application runs, and the list must
follow. Second, **real traffic is messier than generated traffic**: messages arrive split across
packets, in bursts far denser than a generator produces, and sometimes malformed. The monitor must
report what actually arrived, including the malformed parts, because a MIDI monitor that quietly
tidies up its input is useless for the debugging it exists to do.

The visible design is unchanged. Every control the reference screenshots depict keeps its label,
position, control type, and grouping; the only new surface is what real hardware forces — an
indication of why a list is empty, and an indication when a previously selected device is no longer
attached.

**Scope**: macOS, and real input from ports the system reports plus a destination other applications
can send to. Observing another application's *outgoing* traffic — the `Spy on output to destinations`
group — is deferred to a later feature; that group stays on screen and says so, because a control the
reference screenshots depict may not be removed, but it may be honest about what it cannot yet do.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Watch messages from a real MIDI device (Priority: P1)

A user connects a MIDI device — a keyboard, a control surface, an interface — opens the application,
and sees the messages that device is actually sending. Pressing a key on the keyboard puts a Note On
row in the table within a moment of the key going down. Moving a knob produces Control rows. Nothing
appears when the user is not playing.

**Why this priority**: This is the entire point of the feature. Every other story here refines or
extends it, and none of them is meaningful until real bytes are reaching the table.

**Independent Test**: Attach any MIDI device, launch the application, and play it. Confirm rows appear
in response to physical input and stop when input stops. Confirm nothing appears while the device is
idle and untouched.

**Acceptance Scenarios**:

1. **Given** a MIDI device is connected and selected, **When** the user plays a note, **Then** a row
   appears naming that device as the Source, `Note On` as the Message, the correct channel, and the
   note and velocity actually transmitted.
2. **Given** a device is connected and selected, **When** the user does not touch it and it transmits
   nothing, **Then** no rows appear — the list does not grow on its own.
3. **Given** the user moves a continuous controller, **When** the messages arrive, **Then** each is
   listed with the controller number and value the device actually sent, in the order sent.
4. **Given** the user plays a chord and releases it, **When** the rows are read, **Then** the Note On
   and Note Off messages appear in the order the device transmitted them, one row per message, with
   none merged, dropped, or reordered.
5. **Given** a device that transmits Clock while a sequencer runs, **When** Clock is admitted by the
   filters, **Then** Clock rows appear at the device's real transmission rate rather than a
   generated one.
6. **Given** an event is listed, **When** the user inspects its raw bytes, **Then** they are the
   bytes the device transmitted, unaltered.

---

### User Story 2 - See the machine's real MIDI ports in the Sources list (Priority: P1)

A user expands the Sources section and sees the ports actually present on this machine, named as the
operating system names them, grouped as the reference layout groups them. A machine with three
devices attached shows those three; a machine with none shows an empty list that says so.

**Why this priority**: Co-equal with US1 — real events with a fabricated device list would still be a
mock. A user cannot select what to monitor without a truthful list, and the two together are the
smallest change that removes simulation from the product.

**Independent Test**: On a machine with known devices attached, compare the Sources list against the
operating system's own MIDI configuration utility. Then disconnect every device, relaunch, and
confirm the list is empty with a stated reason.

**Acceptance Scenarios**:

1. **Given** devices are attached, **When** the user expands the Sources section, **Then** every MIDI
   input port the system reports is listed under `MIDI sources`, named exactly as the system names
   it.
2. **Given** the system reports no MIDI input ports, **When** the user expands the Sources section,
   **Then** the list is empty and states that no MIDI devices were found, distinguishing this from a
   device that is present but silent.
3. **Given** two attached devices report the same display name, **When** the user reads the list,
   **Then** both appear as separate entries, each independently checkable, and each event is
   attributed to the correct one.
4. **Given** a device exposing several ports, **When** the user reads the list, **Then** each port is
   a separate selectable entry named as the system names it, rather than one entry per physical
   device.
5. **Given** the user checks and unchecks entries, **When** events arrive, **Then** only events from
   checked ports enter the monitor — the behaviour established in the previous feature, now applied
   to real ports.

---

### User Story 3 - Keep monitoring across plug and unplug (Priority: P2)

A user unplugs a device mid-session and plugs in a different one. The Sources list updates on its own:
the removed device's entry goes away or is marked unavailable, the new device appears, and monitoring
of everything else carries on uninterrupted. Reconnecting the original device restores it — still
checked, if it was checked before.

**Why this priority**: Devices are physically handled during a debugging session; that is the normal
case, not an edge case. But a user can still get real value from US1 and US2 by launching with the
device already attached, so this refines rather than enables.

**Independent Test**: With the application running and a device selected, unplug it and confirm the
list updates and the application keeps running. Plug it back in and confirm it returns still selected
and its events resume.

**Acceptance Scenarios**:

1. **Given** the application is running, **When** a MIDI device is connected, **Then** it appears in
   the Sources list without the user restarting or manually refreshing anything.
2. **Given** a selected device is transmitting, **When** it is unplugged, **Then** the application
   continues running, other sources keep being monitored, and the list reflects the device's
   departure.
3. **Given** a device was selected when it was unplugged, **When** it is reconnected, **Then** it is
   still selected and its events resume without the user re-checking it.
4. **Given** a device is unplugged, **When** the user looks at events it produced before the
   disconnect, **Then** those events remain in the list and remain correctly attributed.
5. **Given** the last remaining device is unplugged, **When** the user looks at the window,
   **Then** the application states that no MIDI devices are available rather than appearing frozen.

---

### User Story 4 - Receive MIDI from other applications on this machine (Priority: P3)

A user wants to watch what another application on the same machine is sending. They enable
`Act as a destination for other programs`; the monitor then appears in that other application's list
of MIDI destinations, and anything sent to it is listed.

**Why this priority**: It is one of the two entries the reference layout shows, and it is genuinely
achievable — but it serves inter-application debugging, a narrower need than watching attached
hardware.

**Independent Test**: Enable the entry, then in any other MIDI-capable application on the machine
select the monitor as a destination and send messages to it. Confirm they are listed.

**Acceptance Scenarios**:

1. **Given** `Act as a destination for other programs` is enabled, **When** another application on
   the machine enumerates MIDI destinations, **Then** this monitor appears among them under a name
   identifying it.
2. **Given** another application sends messages to that destination, **When** they arrive, **Then**
   they are listed and attributed to the `Act as a destination for other programs` entry.
3. **Given** the entry is unchecked, **When** the user looks at the effect, **Then** messages sent to
   the monitor from other applications no longer enter the list.
4. **Given** the destination cannot be created, **When** the failure occurs, **Then** the application
   reports why and keeps monitoring everything else.

---

### User Story 5 - Learn why output spying is not available yet (Priority: P4)

A user expands the Sources section, sees the `Spy on output to destinations` group the reference
window shows, and finds it empty with a plain statement that observing output to destinations is not
available in this version. The user is not left wondering whether the group is broken, whether their
devices failed to appear under it, or whether they have misconfigured something.

**Why this priority**: Lowest, and deliberately small. Actually observing another application's
outgoing MIDI is **deferred to a later feature** — it requires privileged system-level support that
must be installed separately and permitted by the user, which is out of proportion to the rest of this
work. What this feature owes the user is honesty about that: the group stays on screen because the
reference screenshots are the design authority, so it must explain itself rather than sit there
looking empty and broken.

**Independent Test**: Expand the Sources section on a machine with devices attached. Confirm the
`Spy on output to destinations` group is present, contains no sources, and carries a statement that
the capability is unavailable — and that everything else in the window works normally.

**Acceptance Scenarios**:

1. **Given** the Sources section is expanded, **When** the user reads the
   `Spy on output to destinations` group, **Then** the group is present with its verbatim label and
   states that observing output to destinations is not available in this version.
2. **Given** that group is unavailable, **When** the user uses any other part of the application,
   **Then** device discovery, monitoring, filtering, and persistence all work normally.
3. **Given** that group is unavailable, **When** the user looks at the event list, **Then** no event
   is ever attributed to it — the application does not invent traffic to fill an empty group.

---

### Edge Cases

- **No MIDI devices at all**: the Sources list is empty and says so, the event table is empty and says
  why, and every other control remains usable. The application must never present this as an error or
  as ordinary silence.
- **The system's MIDI service is unavailable or denies access**: the application states that it cannot
  reach the MIDI system and what the user can do, rather than showing an empty list that looks like
  "no devices".
- **A device is claimed exclusively by another application**: the port is listed but cannot be opened;
  the application says which port failed and why, and keeps monitoring the rest.
- **Sustained real traffic far above the display rate** — dense Clock, Active Sense, and Aftertouch
  from several devices at once: no message is dropped from the record, ordering is preserved, the
  retention cap still holds, and the window stays responsive.
- **A message split across several arrivals**: a message whose bytes arrive in more than one delivery
  is listed once, whole, not as fragments.
- **A very large System Exclusive dump**: it is listed as a single event with its true byte count; a
  dump larger than the application will hold is reported as truncated rather than silently shortened
  or dropped.
- **An incomplete System Exclusive transfer** — a device unplugged or stops mid-dump: the partial
  transfer is reported rather than held indefinitely waiting for an end that never comes.
- **Running status** (a device omitting repeated status bytes): each message is listed in full with
  its implied status, not as headless data.
- **Genuinely malformed input** — a status byte where data was expected, an unrecognised byte, a
  truncated message: listed under the `Invalid` category with its raw bytes, never silently discarded
  and never allowed to corrupt the messages around it.
- **A device replugged into a different port**, or renamed by the system: its saved selection is
  recognised as the same device where the system provides a stable identity for it; where it does not,
  the behaviour is stated rather than guessed at.
- **Two ports with identical names**: selection, attribution, and saved settings distinguish them.
- **A device present at launch that the saved settings do not mention**: it is listed and starts
  unselected, so a newly attached device never silently floods a list the user had narrowed.
- **Selected device unplugged and never returned**: its saved selection is retained rather than
  discarded, so a device that comes back next session comes back checked.
- **Timestamps under load**: rows carry the time the message arrived, and a burst does not make later
  messages appear earlier than ones that preceded them.

## Requirements *(mandatory)*

### Functional Requirements

#### Real Input Replaces Simulation

- **FR-001**: Every event the application lists MUST originate from the machine's MIDI system. The
  application MUST NOT generate, synthesise, or fabricate any event under any condition.
- **FR-002**: The simulated event source MUST be removed from the product. No user-facing control,
  configuration value, command-line option, or build variant may reintroduce simulated traffic.
- **FR-003**: When no events are arriving, the application MUST show an empty or unchanging list and
  MUST distinguish, in the interface, between "no devices available", "no source selected", "every
  message filtered out", and "connected but silent".

#### Device Discovery

- **FR-004**: The application MUST list the MIDI input ports the operating system reports as present,
  using the names the system provides for them, without modification.
- **FR-005**: Each port MUST be a separately selectable entry, including when several ports belong to
  one physical device and when several ports share a display name.
- **FR-006**: The Sources list MUST populate from the real system at launch, with no fixed or
  built-in list of source names remaining in the product.
- **FR-007**: When the system reports no MIDI input ports, the list MUST be empty and MUST state that
  no MIDI devices were found.
- **FR-008**: When the MIDI system cannot be reached or refuses access, the application MUST state
  that, distinguishably from having found no devices, and MUST remain usable.

#### Connection & Disconnection

- **FR-009**: The application MUST detect ports appearing and disappearing while it runs, and MUST
  update the Sources list without the user restarting or manually refreshing.
- **FR-010**: A disconnection MUST NOT interrupt monitoring of any other source and MUST NOT stop the
  application.
- **FR-011**: Events already listed from a now-disconnected source MUST remain listed and correctly
  attributed.
- **FR-012**: A reconnected port MUST return with its previous selection state intact and MUST resume
  delivering events without the user re-checking it.
- **FR-013**: A port that cannot be opened MUST be reported with the reason, and the application MUST
  continue monitoring every port it could open.

#### Message Fidelity

- **FR-014**: Each listed event MUST carry the raw bytes exactly as received, unaltered in value or
  order.
- **FR-015**: The application MUST reassemble a message whose bytes arrive across more than one
  delivery into a single listed event.
- **FR-016**: The application MUST interpret running status, listing each message with its implied
  status byte and full meaning.
- **FR-017**: System Exclusive messages MUST be listed as one event carrying the true byte count of
  the transfer.
- **FR-018**: A System Exclusive transfer exceeding the size the application retains MUST be listed
  and marked as truncated, stating the true size, rather than dropped or silently shortened.
- **FR-019**: A System Exclusive transfer that never completes MUST be reported as incomplete rather
  than withheld indefinitely.
- **FR-020**: Bytes that do not form a valid MIDI message MUST be listed under the `Invalid` category
  with their raw bytes, and MUST NOT affect the interpretation of surrounding messages.
- **FR-021**: Every message type the Filter panel names MUST be recognised and mapped to its
  corresponding filter entry, so every filter control governs real traffic.
- **FR-022**: Each event MUST carry the time it arrived, at millisecond precision, and events MUST be
  ordered by arrival — a burst MUST NOT produce out-of-order rows.
- **FR-023**: No message MUST be dropped from the record because of arrival rate; if the application
  cannot keep pace it MUST say so rather than lose events without a trace.

#### Acting as a Destination

- **FR-024**: The application MUST be able to offer itself to other applications on the machine as a
  MIDI destination, under the existing `Act as a destination for other programs` entry.
- **FR-025**: Messages sent to that destination MUST be listed and attributed to that entry.
- **FR-026**: Unchecking the entry MUST stop those messages from entering the monitor.
- **FR-027**: Failure to offer the destination MUST be reported with its reason and MUST NOT prevent
  the rest of the application from working.

#### Observing Output to Destinations (Deferred)

- **FR-028**: The `Spy on output to destinations` group MUST remain present with its verbatim label,
  MUST contain no sources, and MUST state that observing output to destinations is not available in
  this version.
- **FR-029**: No event MUST ever be attributed to that group in this feature. The application MUST NOT
  populate it with placeholder, sample, or substitute sources.
- **FR-030**: The group's unavailability MUST NOT affect device discovery, monitoring, filtering, or
  persistence.

#### Selection Identity & Persistence

- **FR-031**: A saved source selection MUST be re-applied to the same physical port on a later launch,
  using the most stable identity the system offers for that port rather than an identifier that
  changes between sessions or between physical connectors.
- **FR-032**: A port present at launch that the saved settings do not mention MUST start unselected.
- **FR-033**: A saved selection for a port that is not currently present MUST be retained, not
  discarded, so the port returns selected when it is reattached.
- **FR-034**: Ports sharing a display name MUST be distinguished by saved settings, so selecting one
  does not select the other on a later launch.
- **FR-035**: All settings that persisted before this feature — filter selections, channel mode, hex
  prefix filter and mode, column visibility, retention limit — MUST continue to persist unchanged.

#### The Reference Surface Is Unchanged

- **FR-036**: Every control the reference screenshots depict MUST keep its verbatim label, control
  type, position, grouping, indentation, and default state. Real input MUST NOT cause any of them to
  be relabelled, reordered, restyled, or repurposed.
- **FR-037**: All behaviour specified by the previous feature — the five columns and their formats,
  retention and clearing, source checkboxes and group tri-states, message-type and channel filters,
  the hexadecimal prefix filter, and column visibility — MUST continue to work identically, now
  applied to real events.
- **FR-038**: New interface elements introduced by this feature MUST be confined to communicating
  states that only real hardware creates — no devices found, MIDI system unreachable, port could not
  be opened, source unavailable, output spying unavailable — and MUST NOT alter any control the
  screenshots show.

#### Platform Scope

- **FR-039**: The application MUST provide real MIDI input on macOS. Other operating systems are out
  of scope for this feature: no capability here may be specified, verified, or promised for a platform
  other than macOS.
- **FR-040**: Where the platform requires the user to grant permission before the application may
  receive MIDI, the application MUST surface the outcome of that decision — including a denial — and
  MUST remain usable rather than appearing to have found no devices.

### Key Entities

- **MIDI Port**: A real input endpoint the operating system reports. Attributes: the system-provided
  display name, a stable identity used to recognise it across sessions and reconnections, the group it
  belongs to, whether it is currently present, and whether it is selected for monitoring.
- **Port Availability**: Whether a listed source is currently attached and openable — present and
  open, present but unopenable with a stated reason, or remembered from settings but absent.
- **MIDI System Access**: The application's connection to the operating system's MIDI service.
  Attributes: whether it was established and, if not, why — which is what separates "no devices" from
  "cannot see any devices".
- **Received Message**: One message as it actually arrived. Attributes: arrival time at millisecond
  precision, originating port, raw bytes exactly as received, interpreted message type and channel
  where the type carries one, and whether interpretation succeeded or the bytes were malformed.
- **Incomplete Transfer**: A System Exclusive message whose bytes are still arriving, tracked per port
  until it completes, is abandoned, or exceeds the retained size.
- **Saved Selection**: The persisted record of which ports the user chose, keyed on port identity
  rather than list position, and retained for ports that are not currently attached.

## Out of Scope

- **Observing output to destinations.** Deferred to a later feature. The `Spy on output to
  destinations` group remains on screen, empty, stating its unavailability (FR-028 through FR-030).
  It is deferred rather than dropped because the reference screenshots are the project's design
  authority and a control they depict may not be removed; and it is deferred rather than delivered
  because it needs privileged system support installed outside the application, which is out of
  proportion to the rest of this work.
- **Operating systems other than macOS.** No requirement here is specified or verified for Windows or
  Linux. Cross-platform support, if wanted, is a separate feature with its own decisions about which
  capabilities survive the move.
- **Transmitting MIDI.** The monitor observes. Sending, echoing, and routing between ports are not
  in scope; the destination other applications can send *to* is inbound only.
- **Multiple monitor windows.** Unchanged from the previous feature: one window, one event list.
- **Persisting observed events.** Unchanged: settings persist, events do not. Recording or exporting
  a session to a file is not in scope.
- **Device configuration.** The application observes what the system reports; it does not create,
  rename, reconfigure, or route the system's MIDI setup.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With a MIDI device attached and selected, a user who plays a note sees the corresponding
  row appear within a quarter second of the physical action.
- **SC-002**: The sources a user sees match, name for name and count for count, what the operating
  system's own MIDI configuration utility reports for the same machine.
- **SC-003**: With every device idle, the event list does not grow at all over a five-minute
  observation — confirming no event is generated by the application itself.
- **SC-004**: On a machine with no MIDI devices, the application launches, states that no devices were
  found, and every control remains operable.
- **SC-005**: Connecting or disconnecting a device is reflected in the Sources list within two seconds,
  with no restart and no manual refresh.
- **SC-006**: After unplugging and reattaching a selected device, its events resume without the user
  touching any control.
- **SC-007**: A user who selected specific devices, quit, and relaunched finds exactly those devices
  selected, including after the devices were moved to different physical connectors.
- **SC-008**: A byte-for-byte comparison of a known transmitted sequence against the raw bytes the
  application lists shows no difference in value, order, or count.
- **SC-009**: Under sustained real traffic of at least 500 messages per second across at least two
  devices, every control responds to input within a quarter second, no message is missing from the
  record, and rows remain in arrival order.
- **SC-010**: A System Exclusive dump of at least 1000 bytes is listed as one event reporting its true
  byte count.
- **SC-011**: Deliberately malformed input is listed under `Invalid` with its raw bytes, and the valid
  messages sent immediately before and after it are listed correctly.
- **SC-012**: Every checkbox in the Filter panel demonstrably changes the visible list when exercised
  against real traffic containing that message type.
- **SC-013**: Placed side by side with the reference screenshots, a reviewer finds the same controls
  with the same labels in the same arrangement — the only differences being the real device names in
  the Sources list and messages about hardware availability.
- **SC-014**: A search of the delivered product finds no code path, setting, or build option that
  produces an event not received from the MIDI system.
- **SC-015**: A user who expands the Sources section can say, without asking anyone, why the
  `Spy on output to destinations` group is empty — and over a full session no event is ever attributed
  to it.
- **SC-016**: With MIDI access denied at the operating-system level, the application states that
  access was refused rather than reporting that no devices were found, and every control remains
  operable.

## Assumptions

- **"Nothing mocked" is taken literally and includes deletion.** The simulator is removed from the
  product rather than retained behind a flag, an environment variable, or a debug build. A fallback
  that fabricates traffic when no device is present would reintroduce exactly what this feature
  exists to remove.
- **Input only.** The monitor observes; it does not transmit. Sending MIDI, echoing, or routing
  between ports is out of scope, with the single exception of offering a destination for other
  applications to send *to*, which the reference layout already shows.
- **Port names come from the system and are used verbatim.** The application does not rename, clean
  up, deduplicate, or prettify them, because a name that differs from what the system's own utilities
  show would defeat identifying a device.
- **Ports, not devices, are the unit of selection.** A device exposing several ports appears as
  several entries, matching how the system presents them.
- **The `MIDI sources` group holds the real input ports.** The reference layout's group structure is
  reused as-is: real input ports under `MIDI sources`, the destination other applications can send to
  as the standalone `Act as a destination for other programs` row, and the `Spy on output to
  destinations` group retained but empty pending the later feature that fills it.
- **macOS is the target, and being macOS-only simplifies two requirements.** Because the platform is
  fixed, offering a destination for other applications (FR-024 to FR-027) has one well-defined
  behaviour rather than a per-platform matrix, and port identity (FR-031) can rely on whatever stable
  identifier the macOS MIDI system provides. Nothing here is written to be portable, and no claim is
  made about how it would behave elsewhere.
- **"Unavailable" is not a mock.** Showing the `Spy on output to destinations` group with a statement
  that it is not yet available is honest reporting, not simulated content — FR-029 forbids putting any
  placeholder source or event in it. This is what keeps the deferral compatible with "nothing mocked".
- **Devices present but not mentioned in saved settings start unselected**, so attaching a chatty
  device does not flood a list the user had deliberately narrowed. This differs from the previous
  feature's first-launch behaviour, where every simulated source started selected — with real
  hardware, a first launch has no way to know which of the attached devices the user cares about.
  On a first launch with no saved settings at all, every discovered port starts selected, preserving
  the previous behaviour of showing traffic immediately.
- **Retained events remain session-only**; only settings persist, unchanged from the previous feature.
- **A library provides MIDI system access.** Per the project constitution, device access is obtained
  from an established, maintained library rather than hand-rolled against the operating system's
  interfaces. The choice of library is a planning decision, not a specification one.
- **Message interpretation stays the application's own.** The existing message model exists to mirror
  the Filter panel's vocabulary and to represent invalid data, which no protocol library is designed
  to produce. Real bytes are interpreted into that same model; the model itself is not replaced.
- **Reported rate limits are honest.** Where the application cannot keep pace with arriving traffic,
  it says so rather than dropping events silently — an under-reported monitor is worse than a slow
  one.
- **Timestamps use arrival time at the application.** Where the MIDI system supplies its own more
  precise timestamp, using it is an improvement, not a requirement; what the specification requires is
  correct ordering and millisecond display precision.
- **Permission prompts are the operating system's.** Where the platform requires the user to grant
  access to MIDI or to input devices, the application surfaces the outcome; it does not attempt to
  work around a denial.
- Per the project constitution, this specification defines acceptance scenarios as **manual
  verification steps performed against the running application** with real hardware attached. No
  automated test files, test dependencies, or test harnesses are to be produced for them.
