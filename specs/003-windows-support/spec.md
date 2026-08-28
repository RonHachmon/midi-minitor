# Feature Specification: Windows Support

**Feature Branch**: `003-windows-support`

**Created**: 2026-08-28

**Status**: Draft

**Input**: User description: "The app runs only on macOS. Make it run on Windows as well, with the same monitoring experience: the Sources list shows the MIDI ports Windows actually reports, the list follows devices as they are plugged in and unplugged while the app runs, and every row in the event table is a message that really arrived. Nothing is invented on either platform. A device selection saved on Windows must come back when that device returns, exactly as it does on macOS, and macOS behaviour must not regress in any way. Where Windows genuinely cannot do what macOS does, the app says so on the control itself rather than hiding the control or pretending it works — the same honesty the `Spy on output to destinations` group already shows by staying on screen and reporting that it is unavailable. Two capabilities are known to be at risk and this spec must decide each one explicitly rather than leaving it to implementation: acting as a destination that other programs can send to, which Windows has no built-in equivalent for; and showing raw bytes exactly as transmitted, which Windows drivers may normalise before the application ever sees them. For each, state what a Windows user is actually promised."

## Overview

The monitor runs on macOS only. This feature makes it run on Windows too, with the same monitoring
experience: the Sources list shows the MIDI input ports Windows actually reports, the list follows
devices as cables are plugged and unplugged while the application runs, and every row in the event
table is a message that really arrived. Nothing is invented on either platform, and nothing about
macOS changes.

The window is the same window. Every control the reference screenshots depict keeps its verbatim
label, position, control type, grouping, indentation, and default state on both platforms. The only
permitted visual differences are the ones the platform imposes and the user already expects — native
window chrome, the native system font, native checkbox and radio rendering.

Two capabilities do not survive the move intact, and this specification decides both rather than
leaving them to implementation. They are decided in the same spirit the `Spy on output to
destinations` group already established: **a control the screenshots depict is never removed, never
hidden, and never left to look broken. It stays where it is and says what it cannot do.** Hiding a
control would make the two platforms visibly different products; leaving it enabled but inert would
be a lie a user only discovers after wasting time on it. Saying so on the control is the only option
that is both honest and identical in layout across platforms.

The rest of this specification is mostly about a single promise: **a user cannot tell, from the
truthfulness of what they see, which operating system they are on.** Where that promise cannot be
kept, the window says so on the spot.

## Platform Capability Decisions *(mandatory — the decisions this feature exists to make)*

### Decision 1 — `Act as a destination for other programs`: unavailable on Windows, stated on the control

**What a Windows user is promised**: nothing about this row will work, and the row will tell them so
before they waste time on it. The row stays exactly where the reference layout puts it, with its
verbatim label and its checkbox, in the same position it occupies on macOS. On Windows the checkbox
cannot be switched on, and the row states that Windows provides no built-in way for an application to
publish a MIDI destination that other programs can send to. The statement names the workable
alternative: install a MIDI loopback utility, after which its ports appear under `MIDI sources` and
are monitored exactly like any other port.

**What a Windows user is not promised**: that this application will ever appear in another program's
list of MIDI destinations. It will not, on this platform, in this feature.

**Why not, and why the alternatives were rejected**:

- *Ship or require a loopback driver.* Publishing a destination on Windows requires a system-wide
  driver component, installed with elevation, that changes the machine's MIDI configuration for every
  application on it. That is a far larger intervention than a monitor has any business making, and it
  would turn a monitoring tool into a system modification. Rejected.
- *Depend on the newer Windows MIDI service that does support application-created endpoints.* Its
  endpoints are reliably visible only to programs using that same newer service, and the service is
  not present on every supported Windows installation. The row would therefore work for a minority of
  users and silently fail for the rest — precisely the "pretending it works" this feature forbids.
  Rejected.
- *Hide the row on Windows.* Forbidden: the reference screenshots are the design authority and a
  control they depict may not be removed. Rejected.

**On macOS this row is unchanged** — it publishes the destination and lists what arrives, exactly as
it does today.

### Decision 2 — Raw bytes: exact on both platforms for System Exclusive, faithful-but-assembled on Windows for everything else

**What a Windows user is promised**: every byte the `Data` column shows is a byte the application
actually received from Windows, in the order received, unaltered. The application adds nothing, drops
nothing, re-encodes nothing, and never displays a byte it did not receive. System Exclusive transfers
are byte-for-byte identical to what the device transmitted, the same as on macOS.

**What a Windows user is not promised**: that for messages other than System Exclusive, the byte
count shown equals the byte count that travelled on the cable. Windows assembles short messages
before any application can see them, so a message a device sent using running status — omitting a
repeated status byte to save bandwidth — reaches this application with its status byte already
restored. The `Data` column shows the complete message Windows delivered. On macOS the same physical
transmission is shown as the data bytes that actually crossed the cable.

**Two consequences that must be stated, not discovered**:

1. The application cannot mark which individual rows were assembled this way, because it never sees
   the original and has no way to know. The statement is therefore a standing one attached to the
   `Data` column, not a per-row annotation.
2. The hexadecimal prefix filter matches against the bytes shown. The same physical playing can
   therefore match a prefix on one platform and not the other. This follows from the filter being
   honest about what it filters, and is stated rather than papered over.

**Why this rather than the alternatives**:

- *Reconstruct the wire encoding on Windows by re-deriving where running status "would have" been.*
  This would put invented bytes in a monitor's raw-byte column — a fabrication, and exactly the kind a
  monitor exists to protect its user from. Rejected outright.
- *Hide the `Data` column on Windows.* Forbidden by the same rule as Decision 1, and it would remove
  the most useful column in the window over a difference that affects only byte count. Rejected.
- *Say nothing and let the difference go unnoticed.* A user comparing the same device on two machines
  would conclude one of them is wrong. Rejected.

**On macOS this is unchanged**: the bytes shown are the bytes off the cable, running status included.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Monitor a real MIDI device on Windows (Priority: P1)

A user on Windows installs and launches the application, expands the Sources section, ticks their
keyboard, and plays it. Rows appear naming that device, with the right message types, channels, and
data. Nothing appears when they are not playing.

**Why this priority**: This is the feature. Every other story here refines it, and none of them
matters until real Windows traffic reaches the table.

**Independent Test**: On a Windows machine with any MIDI device attached, launch the application,
tick the device, and play it. Confirm rows appear in response to physical input and stop when input
stops.

**Acceptance Scenarios**:

1. **Given** a MIDI device is attached to a Windows machine and selected, **When** the user plays a
   note, **Then** a row appears naming that device as the Source, `Note On` as the Message, the
   correct channel, and the note and velocity actually transmitted.
2. **Given** a device is selected and untouched, **When** it transmits nothing, **Then** no rows
   appear — the list does not grow on its own.
3. **Given** the user moves a continuous controller, **When** the messages arrive, **Then** each is
   listed with the controller number and value actually sent, in the order sent.
4. **Given** the user plays and releases a chord, **When** the rows are read, **Then** every Note On
   and Note Off appears in transmission order, one row per message, none merged, dropped, or
   reordered.
5. **Given** a device sends a System Exclusive dump, **When** it is listed, **Then** it is one row
   carrying the true byte count, with the bytes exactly as transmitted.
6. **Given** any message type the Filter panel names arrives, **When** the filter for it is unchecked,
   **Then** it stops appearing — every filter control governs real Windows traffic.

---

### User Story 2 - See the machine's real MIDI ports on Windows (Priority: P1)

A user expands the Sources section on Windows and sees the MIDI input ports Windows actually reports,
named as Windows names them, grouped as the reference layout groups them. A machine with no MIDI
devices shows an empty list that says so.

**Why this priority**: Co-equal with US1. Real events under a fabricated device list would still be a
mock, and a user cannot choose what to monitor without a truthful list.

**Independent Test**: On a Windows machine with known devices attached, compare the Sources list
against the ports any other MIDI application on the same machine lists. Then detach every device,
relaunch, and confirm the list is empty with a stated reason.

**Acceptance Scenarios**:

1. **Given** devices are attached, **When** the user expands the Sources section, **Then** every MIDI
   input port Windows reports is listed under `MIDI sources`, named exactly as Windows names it.
2. **Given** Windows reports no MIDI input ports, **When** the user expands the Sources section,
   **Then** the list is empty and states that no MIDI devices were found — distinguishably from a
   device that is present but silent.
3. **Given** two attached ports report the same display name, **When** the user reads the list,
   **Then** both appear as separate entries, each independently checkable, and each event is
   attributed to the correct one.
4. **Given** a device exposing several ports, **When** the user reads the list, **Then** each port is a
   separate selectable entry named as Windows names it.
5. **Given** the MIDI system cannot be reached at all, **When** the user looks at the panel, **Then**
   it says so, distinguishably from having found no devices, and every other control stays usable.
6. **Given** another program already holds a port open exclusively, **When** the user reads that
   port's row, **Then** it is listed, remains tickable, and states that another program is using it.

---

### User Story 3 - macOS is exactly as it was (Priority: P1)

A user on macOS updates to this version and notices nothing. Every device, every message, every
control, every saved setting, and every raw byte behaves precisely as it did before.

**Why this priority**: Co-equal with US1 and US2 — a Windows port bought with macOS regressions is a
loss, not a gain. It is called out as its own story because "we did not break it" is only credible
when someone has actually checked.

**Independent Test**: On macOS, walk every acceptance scenario of the previous feature and confirm
each still passes without alteration.

**Acceptance Scenarios**:

1. **Given** the previous feature's macOS behaviour, **When** every one of its acceptance scenarios is
   re-run on this version, **Then** all of them still pass unchanged.
2. **Given** a macOS user has settings saved by the previous version, **When** they launch this
   version, **Then** those settings load and apply exactly as before, with nothing reset or migrated
   away.
3. **Given** a device transmitting with running status on macOS, **When** its rows are read, **Then**
   the `Data` column shows the bytes off the cable exactly as it did before — Decision 2 changes
   nothing on macOS.
4. **Given** `Act as a destination for other programs` on macOS, **When** the user ticks it, **Then**
   the monitor still appears in other applications' destination lists and still lists what they send.
5. **Given** any control the reference screenshots depict, **When** it is compared side by side with
   the reference image on macOS, **Then** its label, type, position, grouping, indentation, and
   default state are unchanged.

---

### User Story 4 - Keep monitoring across plug and unplug on Windows (Priority: P2)

A user unplugs a device mid-session on Windows and plugs in a different one. The Sources list updates
on its own: the removed device goes or is marked unavailable, the new device appears, and everything
else carries on. Reconnecting the original brings it back still ticked. Quitting and relaunching
brings back the same ticks.

**Why this priority**: Devices are physically handled during a debugging session, so this is the normal
case rather than an edge case — but a Windows user already gets real value from US1 and US2 by
launching with the device attached, so this refines rather than enables.

**Independent Test**: With the application running on Windows and a device ticked, unplug it, confirm
the list updates and the application keeps running; plug it back in and confirm it returns ticked and
its events resume; then quit, relaunch, and confirm it is still ticked.

**Acceptance Scenarios**:

1. **Given** the application is running on Windows, **When** a MIDI device is attached, **Then** it
   appears in the Sources list without a restart or a manual refresh.
2. **Given** a selected device is transmitting, **When** it is unplugged, **Then** the application
   keeps running, other sources keep being monitored, and the list reflects the departure.
3. **Given** a device was ticked when it was unplugged, **When** it is reattached, **Then** it is still
   ticked and its events resume without the user re-ticking it.
4. **Given** a device was ticked on Windows, **When** the user quits and relaunches, **Then** it is
   ticked again — matched to the same physical port, not to a list position.
5. **Given** two attached ports share a display name and one is ticked, **When** the user relaunches,
   **Then** the same one is ticked and the other is not.
6. **Given** a ticked device is unplugged and does not return before the user quits, **When** they
   relaunch later with it attached, **Then** it is ticked — the remembered selection was kept, not
   discarded.
7. **Given** events arrived from a device that has since been unplugged, **When** the user reads them,
   **Then** they remain listed and correctly attributed.

---

### User Story 5 - Learn on the control that acting as a destination is unavailable on Windows (Priority: P2)

A Windows user expands the Sources section, sees the `Act as a destination for other programs` row
where the reference layout puts it, and finds that it cannot be switched on and says why — Windows
has no built-in way for an application to publish a MIDI destination — along with what they can do
instead. They are not left clicking a checkbox that does nothing, and not left wondering whether they
have misconfigured something.

**Why this priority**: This is Decision 1 made visible. It is the difference between a user who
understands the platform in ten seconds and a user who spends twenty minutes hunting for the monitor
in another application's destination list.

**Independent Test**: On Windows, expand the Sources section. Confirm the row is present with its
verbatim label in its reference position, cannot be switched on, and carries a plain statement of why
and of the alternative — and that everything else in the window works normally.

**Acceptance Scenarios**:

1. **Given** the Sources section is expanded on Windows, **When** the user reads the `Act as a
   destination for other programs` row, **Then** it is present with its verbatim label, in its
   reference position, with its checkbox visible.
2. **Given** that row on Windows, **When** the user attempts to switch it on, **Then** it does not
   switch on, and it states that Windows provides no built-in way for an application to publish a MIDI
   destination that other programs can send to.
3. **Given** that statement, **When** the user reads it, **Then** it names the alternative — install a
   MIDI loopback utility, whose ports then appear under `MIDI sources` and are monitored like any
   other port.
4. **Given** that row is unavailable on Windows, **When** the user uses the rest of the application,
   **Then** discovery, monitoring, filtering, columns, retention, and persistence all work normally.
5. **Given** that row is unavailable on Windows, **When** the user watches the event table, **Then** no
   event is ever attributed to it — no traffic is invented to make the row look alive.
6. **Given** a loopback utility is installed on Windows, **When** the user expands the Sources section,
   **Then** its ports appear under `MIDI sources` as ordinary ports and can be ticked and monitored
   like any other.

---

### User Story 6 - Know what the `Data` column is showing on Windows (Priority: P3)

A Windows user reading raw bytes sees, with the `Data` column, a plain statement that Windows delivers
short messages already assembled, so the column shows the complete message Windows delivered rather
than the exact bytes that crossed the cable — and that System Exclusive transfers are exact. A user
comparing the same device against a macOS machine understands the difference instead of concluding one
of the two is lying.

**Why this priority**: Lowest of the six, because it changes nothing a user can act on — but it is the
whole of Decision 2's honesty obligation, and without it the difference is discovered as an apparent
bug.

**Independent Test**: On Windows, look at the event table with the `Data` column visible. Confirm the
statement is present and readable without hunting for it. Confirm the same statement is absent on
macOS.

**Acceptance Scenarios**:

1. **Given** the `Data` column is visible on Windows, **When** the user looks at the event table,
   **Then** a statement is present with that column explaining that Windows delivers short messages
   already assembled and that System Exclusive bytes are exact.
2. **Given** the same application on macOS, **When** the user looks at the event table, **Then** that
   statement is absent, because on macOS it is not true.
3. **Given** a device transmits a System Exclusive dump, **When** the same dump is captured on a
   Windows machine and a macOS machine, **Then** the bytes shown are identical.
4. **Given** a device transmits using running status, **When** the rows are compared across the two
   platforms, **Then** the Windows rows carry the restored status byte, the macOS rows do not, and both
   are the bytes each platform actually delivered — neither shows a byte the application invented.
5. **Given** the `Data` column is hidden through the column controls, **When** the user hides it,
   **Then** the statement goes with it, and returns when the column returns.

---

### Edge Cases

- **A Windows machine with no MIDI devices at all**: the Sources list is empty and says so, the event
  table is empty and says why, and every control remains operable. Never presented as an error.
- **The Windows MIDI system cannot be reached**: stated as such, distinguishably from "no devices
  found", and the application stays usable.
- **A port held exclusively by another program on Windows**: Windows grants MIDI input ports
  exclusively, so this is common rather than rare. The port is listed, remains tickable, and states
  that another program is using it. When that program releases the port, monitoring begins without the
  user re-ticking anything.
- **Windows offers no stable identity for a port**: the saved selection is matched on the port's name.
  If two ports present at that moment share that name and nothing else distinguishes them, the
  remembered selection is applied to **neither**, and both rows say the previous selection could not be
  matched. Guessing one would silently monitor a device the user never chose.
- **A Bluetooth MIDI device on Windows**: it appears only once Windows itself has paired it. Until then
  the application honestly reports it does not exist, rather than implying the application failed to
  find it.
- **A device replugged into a different socket on Windows**: recognised as the same device and returned
  with its selection intact.
- **A port present at launch that saved settings do not mention**: listed and unticked, on both
  platforms, so a newly attached device never floods a list the user had narrowed.
- **A very large System Exclusive dump on Windows**: listed as one event with its true byte count; one
  larger than the application retains is marked truncated with its true size, never silently shortened
  or dropped — matching macOS.
- **A System Exclusive transfer interrupted by an unplug on Windows**: reported as incomplete rather
  than held forever waiting for an end that will not come.
- **Windows reports invalid incoming data without supplying the bytes**: the row appears under
  `Invalid` and states that the bytes were not made available, rather than showing bytes the
  application made up.
- **Sustained dense traffic on Windows** — Clock, Active Sense, and Aftertouch from several devices at
  once: no message is dropped from the record, ordering is preserved, retention holds, the window stays
  responsive; and if the application cannot keep pace it says so rather than losing events without a
  trace.
- **Settings written on one platform and read on the other** (a synced or copied profile): settings
  that are meaningful on both apply; a saved selection naming a port that does not exist here is
  retained, not discarded, and simply matches nothing.
- **A user comparing the two platforms side by side**: every difference they can see is one this
  specification names, and each is explained on the control it affects.

## Requirements *(mandatory)*

### Functional Requirements

#### The Application Runs on Windows

- **FR-001**: The application MUST install and launch on Windows and MUST present the same window,
  with the same controls, as it presents on macOS.
- **FR-002**: The application MUST continue to install and launch on macOS, and MUST NOT require any
  Windows-only component to do so; the reverse MUST also hold.
- **FR-003**: Every capability specified by the previous feature — the five columns and their formats,
  retention and clearing, source checkboxes and group tri-states, message-type and channel filters, the
  hexadecimal prefix filter, column visibility, and settings persistence — MUST work on Windows, except
  where Decision 1 or Decision 2 states otherwise.

#### Nothing Is Invented, on Either Platform

- **FR-004**: Every event listed MUST originate from the operating system's MIDI system. The
  application MUST NOT generate, synthesise, or fabricate any event on any platform, under any
  condition.
- **FR-005**: Every source listed MUST correspond to a port the operating system reports, or to a
  capability row the reference layout depicts. The application MUST NOT invent, pad, or substitute
  ports on either platform.
- **FR-006**: When no events are arriving on Windows, the application MUST distinguish, in the
  interface, between "no devices available", "no source selected", "every message filtered out", and
  "connected but silent" — as it already does on macOS.

#### Device Discovery on Windows

- **FR-007**: The application MUST list the MIDI input ports Windows reports as present, using the
  names Windows provides, without modification.
- **FR-008**: Each port MUST be a separately selectable entry, including when several ports belong to
  one physical device and when several ports share a display name.
- **FR-009**: When Windows reports no MIDI input ports, the list MUST be empty and MUST state that no
  MIDI devices were found.
- **FR-010**: When the Windows MIDI system cannot be reached, the application MUST state that,
  distinguishably from having found no devices, and MUST remain usable.

#### Connection and Disconnection on Windows

- **FR-011**: The application MUST detect ports appearing and disappearing while it runs on Windows,
  and MUST update the Sources list without a restart or a manual refresh.
- **FR-012**: A disconnection MUST NOT interrupt monitoring of any other source and MUST NOT stop the
  application.
- **FR-013**: Events already listed from a now-disconnected source MUST remain listed and correctly
  attributed.
- **FR-014**: A reconnected port MUST return with its selection state intact and MUST resume delivering
  events without the user re-ticking it.
- **FR-015**: A port that cannot be opened MUST be listed with the reason stated on its row, MUST remain
  tickable, and the application MUST continue monitoring every port it could open.
- **FR-016**: When a port that could not be opened because another program held it becomes available,
  the application MUST begin monitoring it without the user re-ticking it.

#### Selection Identity and Persistence on Windows

- **FR-017**: A saved source selection MUST be re-applied to the same physical port on a later Windows
  launch, matched on the most stable identity Windows offers for that port — one that survives quitting,
  replugging into a different socket, and rebooting, and that distinguishes two ports sharing a display
  name.
- **FR-018**: A saved selection for a port not currently present MUST be retained, not discarded, so the
  port returns ticked when it is reattached.
- **FR-019**: A port present at launch that the saved settings do not mention MUST start unticked.
- **FR-020**: Where Windows offers no identity beyond a port's name, the selection MUST be matched on
  the name; and where two ports present at that moment share that name with nothing else to distinguish
  them, the selection MUST be applied to neither and both rows MUST state that the previous selection
  could not be matched.
- **FR-021**: All settings that persist on macOS — filter selections, channel mode, hexadecimal prefix
  filter and mode, column visibility, retention limit, source selection — MUST persist identically on
  Windows.

#### Message Fidelity on Windows

- **FR-022**: Each listed event MUST carry the bytes exactly as the application received them from
  Windows, unaltered in value or order. The application MUST NOT add, remove, reorder, or re-encode any
  byte.
- **FR-023**: The application MUST reassemble a message whose bytes arrive across more than one delivery
  into a single listed event.
- **FR-024**: System Exclusive messages MUST be listed as one event carrying the true byte count of the
  transfer, with the bytes exactly as transmitted — identically on both platforms.
- **FR-025**: A System Exclusive transfer exceeding the size the application retains MUST be listed and
  marked truncated, stating the true size, rather than dropped or silently shortened.
- **FR-026**: A System Exclusive transfer that never completes MUST be reported as incomplete rather
  than withheld indefinitely.
- **FR-027**: Bytes that do not form a valid MIDI message MUST be listed under the `Invalid` category
  and MUST NOT affect the interpretation of surrounding messages. Where Windows reports invalid data
  without supplying the offending bytes, the row MUST state that the bytes were not made available
  rather than display bytes the application did not receive.
- **FR-028**: Every message type the Filter panel names MUST be recognised on Windows and mapped to its
  corresponding filter entry, so every filter control governs real Windows traffic.
- **FR-029**: Each event MUST carry the time it arrived at millisecond precision, and events MUST be
  ordered by arrival — a burst MUST NOT produce out-of-order rows.
- **FR-030**: No message MUST be dropped from the record because of arrival rate; if the application
  cannot keep pace it MUST say so rather than lose events without a trace.

#### Decision 1 — Acting as a Destination

- **FR-031**: On Windows, the `Act as a destination for other programs` row MUST remain present with its
  verbatim label, its checkbox, and its reference position and indentation.
- **FR-032**: On Windows, that checkbox MUST NOT be switchable on, and the row MUST state that Windows
  provides no built-in way for an application to publish a MIDI destination that other programs can send
  to.
- **FR-033**: That statement MUST name the alternative available to the user: installing a MIDI loopback
  utility, whose ports then appear under `MIDI sources` and are monitored like any other port.
- **FR-034**: On Windows, no event MUST ever be attributed to that row, and the application MUST NOT
  populate it with placeholder or substitute traffic.
- **FR-035**: The row's unavailability on Windows MUST NOT affect discovery, monitoring, filtering,
  columns, retention, or persistence.
- **FR-036**: On macOS, that row MUST continue to publish the destination and list what arrives, with no
  change in behaviour.

#### Decision 2 — Raw Bytes

- **FR-037**: On Windows, the application MUST state, with the `Data` column, that Windows delivers short
  messages already assembled — so the column shows the complete message Windows delivered rather than
  the exact bytes carried on the cable — and that System Exclusive transfers are exact.
- **FR-038**: That statement MUST be visible whenever the `Data` column is visible, and MUST NOT be shown
  on macOS, where it is not true.
- **FR-039**: The application MUST NOT reconstruct, infer, or annotate which individual messages Windows
  assembled. It cannot know, and a per-row claim it cannot support would be a fabrication.
- **FR-040**: The hexadecimal prefix filter MUST match against the bytes shown on the platform in use,
  and MUST NOT compensate for the platform difference by matching bytes that were not received.
- **FR-041**: On macOS, the `Data` column MUST continue to show the bytes as they arrived off the cable,
  running status included, with no change in behaviour.

#### Honesty Is Stated on the Control

- **FR-042**: Any capability unavailable on the platform in use MUST be reported on the control that
  offers it — never by hiding the control, never by removing it, and never by leaving it operable but
  inert.
- **FR-043**: Each such statement MUST say what is unavailable and why, in plain language, and MUST name
  an alternative where one exists.
- **FR-044**: A capability available on the platform in use MUST NOT carry an unavailability statement —
  statements are platform-specific and MUST NOT appear where they do not apply.

#### The Reference Surface Is Unchanged on Both Platforms

- **FR-045**: Every control the reference screenshots depict MUST keep its verbatim label, control type,
  position, grouping, indentation, and default state on both platforms. Windows support MUST NOT cause
  any of them to be relabelled, reordered, restyled, or repurposed.
- **FR-046**: Visual differences between the platforms MUST be limited to what the platform imposes —
  native window chrome, the native system font, native checkbox and radio rendering.
- **FR-047**: New interface text introduced by this feature MUST be confined to stating platform
  availability and MUST NOT alter any control the screenshots show.
- **FR-048**: The `Spy on output to destinations` group MUST remain present, empty, and stating its
  unavailability on **both** platforms, exactly as it does today. This feature MUST NOT change it.

#### macOS Must Not Regress

- **FR-049**: Every behaviour specified by the previous feature MUST continue to hold on macOS,
  unchanged and unqualified.
- **FR-050**: Settings written by the previous version on macOS MUST load and apply unchanged; no saved
  selection, filter, or preference may be reset or lost by this feature.

### Key Entities

- **MIDI Port**: A real input endpoint the operating system reports, on either platform. Attributes: the
  system-provided display name, the most stable identity that platform offers for it, the group it
  belongs to, whether it is currently present, and whether it is selected for monitoring.
- **Port Availability**: Whether a listed source can currently deliver — present and open, present but
  unopenable with a stated reason (notably "another program is using it", which is routine on Windows),
  or remembered from settings but absent.
- **MIDI System Access**: The application's connection to the platform's MIDI service, and if it failed,
  why — which is what separates "no devices" from "cannot see any devices".
- **Platform Capability**: A capability that exists on one supported platform and not another.
  Attributes: the control it belongs to, whether it is available on the platform in use, and the
  statement shown on that control when it is not. This is the entity that makes Decisions 1 and 2
  representable in one shape rather than as two special cases.
- **Received Message**: One message as it actually arrived. Attributes: arrival time at millisecond
  precision, originating port, the bytes exactly as received from the operating system, the interpreted
  type and channel where the type carries one, and whether interpretation succeeded.
- **Saved Selection**: The persisted record of which ports the user chose, keyed on port identity rather
  than list position, retained for ports that are not currently attached.

## Out of Scope

- **Linux.** No requirement here is specified or verified for Linux. It is a separate feature with its
  own decisions about which capabilities survive.
- **Shipping, bundling, or installing a MIDI loopback or virtual-port driver on Windows.** The
  application names the alternative (FR-033); it does not install system components. See Decision 1.
- **Observing output to destinations.** Still deferred, on both platforms, unchanged by this feature
  (FR-048).
- **Transmitting MIDI.** The monitor observes. Sending, echoing, and routing are not in scope on either
  platform.
- **Reconstructing the wire encoding on Windows.** Explicitly rejected in Decision 2 as fabrication.
- **Code signing, notarisation, installers, and distribution.** The application must build and run on
  both platforms; how it is delivered to users is a separate concern.
- **MIDI 2.0 and Universal MIDI Packet transports.** The monitor observes MIDI 1.0 byte streams on both
  platforms.
- **Multiple monitor windows, persisting observed events, and device configuration.** Unchanged from the
  previous feature and still out of scope.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On a Windows machine with a device attached and ticked, a user who plays a note sees the
  corresponding row within a quarter second of the physical action.
- **SC-002**: The sources a Windows user sees match, name for name and count for count, the MIDI input
  ports any other MIDI application on the same machine lists.
- **SC-003**: With every device idle, the event list does not grow at all over a five-minute observation,
  on both platforms — confirming no event is generated by the application itself.
- **SC-004**: On a Windows machine with no MIDI devices, the application launches, states that no devices
  were found, and every control remains operable.
- **SC-005**: Attaching or detaching a device on Windows is reflected in the Sources list within two
  seconds, with no restart and no manual refresh.
- **SC-006**: A selection made on Windows returns ticked after quit-and-relaunch and after
  unplug-and-replug in ten out of ten consecutive attempts, including when two attached ports share a
  display name.
- **SC-007**: Every acceptance scenario of the previous feature still passes on macOS, unmodified — zero
  regressions.
- **SC-008**: The same System Exclusive dump from the same device produces byte-for-byte identical `Data`
  content on a Windows machine and a macOS machine.
- **SC-009**: A user who has read no documentation can state, from the window alone, which capabilities
  are unavailable on their platform and why, for every one of them.
- **SC-010**: Zero rows in the event table, across a full session on either platform, are attributed to a
  source that is not present, and zero rows carry a byte the application did not receive from the
  operating system.
- **SC-011**: A side-by-side comparison of the running window against each reference screenshot matches
  on both platforms for label text, control type, order, indentation, and default state, with differences
  limited to native chrome, font, and control rendering.

## Assumptions

- **Supported Windows versions**: Windows 10 (version 1809 or later) and Windows 11, 64-bit. Older
  Windows releases and ARM64 are not verified by this feature. Chosen because these are the versions
  still receiving support and the ones a MIDI user is realistically running.
- **No permission prompt on Windows**: Windows does not gate MIDI input access for desktop applications,
  so the permission handling macOS requires has no Windows counterpart. A Bluetooth MIDI device must be
  paired through Windows itself before it appears — the application reports it as absent until then,
  which is the truth.
- **Windows grants MIDI input ports exclusively**, so "another program is using this port" is a routine
  state on Windows rather than the rarity it is on macOS. This is why FR-015 and FR-016 make a held port
  a first-class, recoverable state instead of an error.
- **Windows delivers short messages already assembled and System Exclusive transfers as real byte
  buffers.** This is the platform fact Decision 2 rests on. If a Windows delivery path is later found
  that exposes the wire stream without discarding malformed bytes, Decision 2 should be revisited — it is
  a decision about what Windows can be made to tell us, not a preference.
- **Windows exposes a per-port identity stable across replug and reboot** for the ports it reports.
  FR-020 exists for the case where it does not, rather than assuming it never happens.
- **The reference screenshots remain the design authority on both platforms.** They were captured on
  macOS; on Windows they govern labels, control types, order, indentation, and default states, while
  native chrome, font, and control rendering follow the platform.
- **The two platforms share one behaviour specification.** Every difference a user can observe is one this
  document names in Decision 1, Decision 2, or an availability statement — there is no third category of
  quiet divergence.
- **`Spy on output to destinations` stays deferred on both platforms.** This feature neither advances nor
  retreats from the previous feature's position on it.
- **Verification is by hand on both platforms**, per the project's verification standard: the application
  is built and run on a Windows machine and on a macOS machine, and the scenarios above are exercised
  through the interface on each.
