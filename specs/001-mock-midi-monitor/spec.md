# Feature Specification: Mock MIDI Monitor

**Feature Branch**: `001-mock-midi-monitor`

**Created**: 2026-08-26

**Status**: Draft

**Input**: User description: "build a mock midi monitor that looks exactly the the one on the screenshots images with same functrionlites to select sources to monitor by name, and avaliabe filters. on data itself have time, source , message, chan and Data, and an option to hide some of the row  and an option to filter in/out specfic Data by their hex code prefix"

## Overview

A desktop MIDI monitoring window that reproduces the layout and behaviour of the reference
screenshots (`screenshots/main-screen.png`, `screenshots/sources.png`, `screenshots/filters.png`).
The application is a **mock**: it observes no real MIDI hardware. A built-in simulator produces a
realistic stream of MIDI events attributed to named virtual sources, so the entire monitoring,
filtering, and inspection experience is exercisable on any machine with no devices attached.

Beyond the reference behaviour, the event list adds two capabilities: the user can choose which
columns of an event row are shown, and can include or exclude events by matching a hexadecimal
prefix against the event's raw MIDI bytes.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Watch a live stream of MIDI events (Priority: P1)

A user opens the application and immediately sees a live, scrolling list of MIDI events. Each row
shows when the event arrived, which source produced it, what kind of message it was, its channel,
and its data. The list grows as new events arrive; the user can cap how many events are retained
and can clear the list to start a fresh observation.

**Why this priority**: This is the product. Without a visible event stream there is nothing to
select sources for, nothing to filter, and nothing to inspect. It is a complete, demonstrable
application on its own.

**Independent Test**: Launch the app with no other feature implemented. Confirm events appear
continuously with plausible timestamps, message names, channels, and data; change the retention
count and confirm the list is trimmed; press Clear and confirm the list empties and then refills.

**Acceptance Scenarios**:

1. **Given** the application has just launched, **When** the user looks at the window, **Then** a
   table with the columns Time, Source, Message, Chan, and Data is visible and new event rows are
   appearing without any user action.
2. **Given** events are streaming, **When** the user reads any row, **Then** Time is shown as
   `HH:MM:SS.mmm`, Source shows the originating source's display name, Message shows a
   human-readable MIDI message name (e.g. `Note On`, `Channel Pressure`), Chan shows a number from
   1 to 16, and Data shows the message's value(s) in a form appropriate to that message type.
3. **Given** the retention limit is set to 1000 and 1000 events are listed, **When** further events
   arrive, **Then** the oldest events are discarded so the list never exceeds 1000 rows.
4. **Given** the list contains events, **When** the user activates Clear, **Then** all rows are
   removed immediately and newly arriving events begin populating the empty list.
5. **Given** the user types a new value into the retention field, **When** the value is committed,
   **Then** the list is trimmed to the new limit immediately if it currently holds more events.

---

### User Story 2 - Choose which sources to monitor by name (Priority: P2)

A user expands the Sources section and sees a bordered list of named, checkable sources arranged in
groups. Unchecking a source stops its events from entering the monitor; checking a group toggles
every source beneath it. Only checked sources contribute to the event list.

**Why this priority**: With several sources producing traffic simultaneously, isolating one by name
is the first thing a user reaches for. It requires the event stream (P1) to exist but nothing else.

**Independent Test**: With multiple sources emitting, uncheck all but one and confirm the Source
column shows only that name; re-check a group and confirm its members' events return.

**Acceptance Scenarios**:

1. **Given** the Sources section is collapsed, **When** the user activates its disclosure control,
   **Then** the section expands to reveal the source list, and activating it again collapses it.
2. **Given** the source list is visible, **When** the user reads it, **Then** it shows the group
   `MIDI sources` containing individually named sources, a standalone entry `Act as a destination
   for other programs`, and the group `Spy on output to destinations` containing its own named
   sources — each entry with its own checkbox, and group members visually indented beneath their
   group.
3. **Given** a source is checked, **When** the user unchecks it, **Then** no further events from
   that source appear in the list, while events from other checked sources continue to appear.
4. **Given** a group checkbox is checked, **When** the user unchecks the group, **Then** every
   source within that group is unchecked and contributes no events.
5. **Given** a group's members are individually toggled so some are checked and some are not,
   **When** the user looks at the group checkbox, **Then** it shows a mixed/indeterminate state
   distinct from both fully-checked and fully-unchecked.
6. **Given** a source was unchecked and later re-checked, **When** its events resume, **Then** only
   events occurring after re-checking appear — events missed while unchecked are not back-filled.

---

### User Story 3 - Filter by message type and channel (Priority: P3)

A user expands the Filter section and sees checkboxes for every category of MIDI message, arranged
in three columns, plus a choice between watching all channels or one specific channel. Unchecking a
message type removes those events from the list; choosing a single channel restricts the list to
channel-bearing messages on that channel.

**Why this priority**: Real MIDI streams are dominated by high-rate messages (Clock, Active Sense,
Aftertouch). Suppressing them by category is what makes the list readable. Depends only on the
event stream.

**Independent Test**: Uncheck `Real Time` and confirm Clock and Active Sense events disappear while
Note events continue; select One Channel = 5 and confirm only channel 5 events remain.

**Acceptance Scenarios**:

1. **Given** the Filter section is collapsed, **When** the user activates its disclosure control,
   **Then** the section expands to reveal the filter controls, and activating it again collapses it.
2. **Given** the filter panel is visible, **When** the user reads it, **Then** it presents the
   parent category `Voice Messages` over the sub-types Note On/Off, Aftertouch (Poly), Control,
   Program, Channel Pressure, and Pitch Wheel; the parent category `System Common` over Time Code,
   Song Position Pointer, Song Select, and Tune Request; the parent category `Real Time` over Clock,
   Start/Stop/Continue, Active Sense, and Reset; and the standalone categories `System Exclusive`
   and `Invalid`.
3. **Given** all filter checkboxes are checked, **When** the user unchecks a sub-type such as
   `Clock`, **Then** Clock events stop appearing and every other message type is unaffected.
4. **Given** a parent category is checked, **When** the user unchecks it, **Then** all of its
   sub-types are unchecked and none of their events appear; re-checking the parent re-checks all of
   its sub-types.
5. **Given** a parent category's sub-types are partly checked, **When** the user looks at the parent
   checkbox, **Then** it shows a mixed/indeterminate state.
6. **Given** `All Channels` is selected, **When** the user selects `One Channel` and enters 5,
   **Then** only channel-bearing messages on channel 5 appear, and messages that carry no channel
   (System Common, Real Time, System Exclusive) are excluded while One Channel is active.
7. **Given** `One Channel` is selected, **When** the user selects `All Channels`, **Then** events on
   every channel appear again and the channel number field becomes inactive while retaining its
   value.

---

### User Story 4 - Include or exclude events by hexadecimal data prefix (Priority: P4)

A user needs to isolate one specific kind of message that the category filters cannot express — for
example only controller 7 messages, or everything except a particular note. The user enters one or
more hexadecimal prefixes and chooses whether matching events are the only ones shown, or the only
ones hidden. Prefixes are matched against the event's raw MIDI bytes.

**Why this priority**: A precision tool used after the broad category filters have narrowed the
stream. Valuable but not needed to make the monitor useful.

**Independent Test**: Enter the prefix `B0 07` in include mode and confirm only channel-1 controller
7 events remain; switch to exclude mode and confirm exactly those events are the only ones missing.

**Acceptance Scenarios**:

1. **Given** the data-prefix filter is empty, **When** the user looks at the event list, **Then**
   no events are hidden on account of this filter.
2. **Given** the user enters the prefix `90`, **When** include mode is active, **Then** only events
   whose raw bytes begin with `90` are listed.
3. **Given** the same prefix `90`, **When** the user switches to exclude mode, **Then** every event
   is listed except those whose raw bytes begin with `90`.
4. **Given** the user enters a single hex digit such as `9`, **When** the filter applies, **Then**
   it matches every event whose first byte begins with that nibble — `90` through `9F` — so
   partial-byte prefixes are supported.
5. **Given** the user enters several prefixes, **When** the filter applies, **Then** an event
   matches if it matches **any** of the entered prefixes.
6. **Given** the user types a character that is not a hexadecimal digit or separating whitespace,
   **When** the entry is evaluated, **Then** the application shows the entry as invalid and states
   why, and continues applying the last valid filter rather than hiding everything.
7. **Given** a prefix is entered in lower case with or without spaces between bytes, **When** the
   filter applies, **Then** matching is unaffected by letter case and by whitespace between bytes.
8. **Given** the user wants to know what to type, **When** the raw bytes of listed events are
   needed, **Then** the user can view each event's raw MIDI bytes in hexadecimal.

---

### User Story 5 - Hide columns that are not of interest (Priority: P5)

A user working on a single source, or on messages that carry no channel, wants to reclaim
horizontal space. The user chooses which of the five columns are shown; hidden columns disappear
from the header and from every row, and the remaining columns take up the freed width.

**Why this priority**: A comfort refinement. The monitor is fully usable without it.

**Independent Test**: Hide the Source and Chan columns and confirm the header and all rows show only
Time, Message, and Data, with no leftover gap.

**Acceptance Scenarios**:

1. **Given** all five columns are shown, **When** the user hides the Chan column, **Then** the Chan
   header and every row's Chan value are no longer displayed and the remaining columns reflow to use
   the full width.
2. **Given** a column is hidden, **When** the user shows it again, **Then** it reappears in its
   original left-to-right position, not at the end.
3. **Given** the user attempts to hide every column, **When** the last visible column would be
   hidden, **Then** the application prevents it and at least one column remains visible.
4. **Given** a column is hidden, **When** filters that reference that column's data are applied,
   **Then** filtering behaviour is unchanged — hiding a column affects display only, never which
   events are listed.

---

### Edge Cases

- **No sources selected**: every source is unchecked — the list stops growing and communicates that
  nothing is being monitored rather than appearing frozen or broken.
- **All message categories unchecked**: no events pass the filter — the list stops growing and the
  emptiness is explained rather than silently ambiguous with "no traffic".
- **Retention limit set to zero, blank, negative, or non-numeric**: the value is rejected or
  normalised to the nearest valid limit; the application never enters a state where it retains
  nothing without saying so, and never accepts a value it will not honour.
- **Retention limit set very high**: an upper bound exists so a user cannot request more retained
  events than the application can hold responsively.
- **Retention limit lowered below the current row count**: the excess oldest events are discarded
  immediately, not gradually as new events arrive.
- **Channel number outside 1–16**: the entry is rejected or clamped to the valid range; the filter
  never applies a channel that cannot exist.
- **Burst of events far exceeding the display rate**: the window stays responsive and controls stay
  usable; the list may coalesce visual updates but the retention cap and event ordering remain
  correct.
- **Clear pressed while events are streaming**: the list empties and immediately resumes filling;
  no events are duplicated or lost from the ongoing stream.
- **Odd-length hex prefix** (e.g. `B0 0`): treated as a partial final byte and matched on the
  leading nibble, consistent with the single-digit case.
- **Hex prefix longer than a message's byte count**: that message simply does not match; it is not
  an error.
- **Two sources sharing a display name**: each remains independently selectable and its events are
  attributed to the correct entry.
- **Filter changes are not retroactive**: events already listed are re-evaluated against the current
  filters so the visible list always reflects the settings on screen, rather than showing a mix of
  events admitted under older settings.
- **Window resized very narrow**: columns remain readable, with content truncated or scrolled rather
  than overlapping.

## Requirements *(mandatory)*

### Functional Requirements

#### Window & Layout

- **FR-001**: The application MUST present a single main window whose layout matches the reference
  screenshots: a Sources disclosure section, a Filter disclosure section, a retention control row
  reading `Remember up to [N] events` with a `Clear` button aligned to the opposite edge, and below
  them an event table filling the remaining space.
- **FR-002**: The Sources and Filter sections MUST each be independently collapsible, showing a
  right-pointing disclosure indicator when collapsed and a down-pointing one when expanded, and MUST
  both start collapsed as in the reference main screen.
- **FR-003**: Expanding or collapsing a section MUST resize the event table to occupy the remaining
  space without the window itself changing size.

#### Event Stream (Mock)

- **FR-004**: The application MUST generate simulated MIDI events attributed to named virtual
  sources, with no dependency on real MIDI hardware, drivers, or ports being present.
- **FR-005**: Generated traffic MUST be plausible: musically coherent Note On / Note Off pairs with
  velocities, periodic Real Time messages such as Clock and Active Sense, occasional Control,
  Program, Pitch Wheel, Channel Pressure, Aftertouch, System Common, and System Exclusive messages,
  spread across more than one channel and more than one source.
- **FR-006**: Every generated event MUST carry an arrival timestamp, an originating source, a MIDI
  message type, a channel where the message type defines one, and its raw MIDI bytes.
- **FR-007**: Generated traffic MUST include at least one example of every message category listed
  in the Filter panel, so that every filter control is demonstrably effective.

#### Event Table

- **FR-008**: The event table MUST display the columns Time, Source, Message, Chan, and Data in that
  left-to-right order, with a persistent header row.
- **FR-009**: Time MUST be displayed as `HH:MM:SS.mmm` in 24-hour form with millisecond precision.
- **FR-010**: Source MUST display the originating source's name as shown in the Sources list.
- **FR-011**: Message MUST display the human-readable MIDI message name (`Note On`, `Note Off`,
  `Channel Pressure`, `Control`, `Program`, `Pitch Wheel`, `Aftertouch (Poly)`, `Clock`, and so on).
- **FR-012**: Chan MUST display the channel number 1–16 for channel-bearing messages and MUST be
  blank for messages that carry no channel.
- **FR-013**: Data MUST display the message's payload in a form appropriate to its type — note name
  and velocity for note messages, controller number and value for Control, a single value for
  Channel Pressure and Program, and a byte count or byte listing for System Exclusive.
- **FR-014**: New events MUST be appended so the list reads oldest-to-newest top-to-bottom, matching
  the reference screenshot's ascending timestamps.
- **FR-015**: The user MUST be able to view any listed event's raw MIDI bytes in hexadecimal, so the
  values used by the data-prefix filter are discoverable from the interface.

#### Retention & Clearing

- **FR-016**: The user MUST be able to set the maximum number of retained events via the
  `Remember up to [N] events` field, which MUST default to 1000.
- **FR-017**: When the retained event count reaches the limit, the application MUST discard the
  oldest events so the count never exceeds it.
- **FR-018**: Lowering the limit MUST immediately discard the excess oldest events.
- **FR-019**: The retention field MUST accept only positive whole numbers within a stated supported
  range, rejecting or normalising anything else and never silently accepting a value it will not
  honour.
- **FR-020**: The `Clear` control MUST remove all retained events immediately, after which newly
  arriving events continue to be listed.

#### Source Selection

- **FR-021**: The Sources section MUST present a bordered, scrollable list of checkable entries
  organised as: the group `MIDI sources` with named source children, the standalone entry `Act as a
  destination for other programs`, and the group `Spy on output to destinations` with named source
  children — matching the reference sources screenshot.
- **FR-022**: Group entries MUST be expandable and collapsible, with their children visually
  indented, and MUST start expanded.
- **FR-023**: Each entry MUST have a checkbox; only events from checked sources MUST enter the
  monitor.
- **FR-024**: Toggling a group's checkbox MUST apply the same state to all of its children.
- **FR-025**: A group whose children are partly checked MUST display a mixed/indeterminate checkbox
  state distinguishable from both checked and unchecked.
- **FR-026**: Unchecking a source MUST take effect immediately for subsequently arriving events, and
  MUST also remove that source's already-listed events from the visible list so that the list always
  matches the current selection.
- **FR-027**: When no source is checked, the application MUST indicate that nothing is being
  monitored rather than presenting an unexplained empty or static list.

#### Message Filters

- **FR-028**: The Filter section MUST present, in three columns matching the reference filter
  screenshot: `Voice Messages` over Note On/Off, Aftertouch (Poly), Control, Program, Channel
  Pressure, Pitch Wheel; `System Common` over Time Code, Song Position Pointer, Song Select, Tune
  Request; `Real Time` over Clock, Start/Stop/Continue, Active Sense, Reset; plus standalone
  `System Exclusive` and `Invalid`.
- **FR-029**: Every filter entry MUST be a checkbox and MUST start checked.
- **FR-030**: Only events whose message type is checked MUST be listed.
- **FR-031**: Toggling a parent category MUST apply the same state to all of its sub-types, and a
  partly-checked parent MUST display a mixed/indeterminate state.
- **FR-032**: The filter panel MUST offer a mutually exclusive choice between `All Channels` and
  `One Channel`, with a channel number entry that is active only when `One Channel` is selected and
  that accepts only values 1–16.
- **FR-033**: When `One Channel` is selected, only channel-bearing messages on that channel MUST be
  listed; messages that carry no channel MUST be excluded for as long as `One Channel` is active.
- **FR-034**: Changing any filter MUST re-evaluate the events already listed, so the visible list
  always reflects the settings currently shown on screen.

#### Hexadecimal Data Prefix Filter

- **FR-035**: The user MUST be able to enter one or more hexadecimal prefixes that are matched
  against each event's raw MIDI bytes rendered as an uninterrupted hex string.
- **FR-036**: The user MUST be able to choose whether matching events are the only events shown
  (include mode) or the only events hidden (exclude mode).
- **FR-037**: An event MUST be considered a match when its hex byte string begins with any one of
  the entered prefixes.
- **FR-038**: Prefix matching MUST be case-insensitive and MUST ignore whitespace between bytes, so
  `b0 07`, `B007`, and `B0 07` are equivalent.
- **FR-039**: Prefixes of odd hex-digit length MUST match on the leading nibble of the final byte,
  so `9` matches `90` through `9F`.
- **FR-040**: An empty prefix entry MUST hide nothing, in either mode.
- **FR-041**: Entry containing anything other than hexadecimal digits and separating whitespace MUST
  be reported as invalid with a stated reason, and the last valid filter MUST remain in effect.

#### Column Visibility

- **FR-042**: The user MUST be able to choose which of the five columns are displayed.
- **FR-043**: Hiding a column MUST remove it from the header and from every row, with the remaining
  columns reflowing to use the freed width.
- **FR-044**: Showing a previously hidden column MUST restore it to its original left-to-right
  position.
- **FR-045**: At least one column MUST remain visible at all times.
- **FR-046**: Column visibility MUST affect display only and MUST NOT change which events are
  listed, including for filters that reference a hidden column's data.

#### Settings Persistence

- **FR-047**: Source selections, filter settings, the data-prefix filter and its mode, column
  visibility, and the retention limit MUST persist across application restarts. Retained events
  themselves MUST NOT persist.

### Key Entities

- **MIDI Event**: One observed message. Attributes: arrival timestamp (millisecond precision),
  originating source, message type, channel (absent for message types that carry none), raw MIDI
  bytes, and the display values derived from those bytes for the Data column.
- **Source**: A named origin of events. Attributes: display name, the group it belongs to
  (`MIDI sources`, `Spy on output to destinations`, or standalone), and whether it is selected for
  monitoring.
- **Source Group**: A named, collapsible container of sources whose checkbox reflects and controls
  its members' selection, including a mixed state.
- **Message Type**: A category of MIDI message. Attributes: display name, parent category
  (`Voice Messages`, `System Common`, `Real Time`, or standalone), whether it carries a channel, and
  whether it is currently admitted by the filter.
- **Filter Settings**: The complete set of what is currently admitted — message-type selections,
  channel mode and channel number, hex prefixes with include/exclude mode.
- **Event Log**: The ordered collection of retained events, bounded by the retention limit, that
  discards oldest-first and can be cleared.
- **Column**: One field of an event row. Attributes: header label, left-to-right position, and
  whether it is visible.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Placed side by side with the reference screenshots, a reviewer identifies the same
  controls with the same labels in the same arrangement in all three states — main window, expanded
  Sources, expanded Filter.
- **SC-002**: Within 10 seconds of launch and with no configuration, a user sees a continuously
  updating list of events populated across all five columns.
- **SC-003**: A user who has never seen the application can isolate the traffic of one named source
  in under 15 seconds, using only the on-screen controls.
- **SC-004**: A user can suppress the highest-rate message categories and reduce the visible event
  rate by at least 90% in under 10 seconds.
- **SC-005**: With a sustained simulated rate of at least 500 events per second, the window remains
  responsive: every control reacts to input within a quarter second and the list continues to update
  without stalling.
- **SC-006**: The retained event count never exceeds the configured limit, verified by setting the
  limit to a small number and counting the rows during a sustained burst.
- **SC-007**: Every filter control in the Filter panel demonstrably changes the visible list — for
  each control, toggling it off removes at least one category of event that was previously visible
  and toggling it on restores it.
- **SC-008**: A user can restrict the list to a single kind of message via a hex prefix in under 20
  seconds, and inverting to exclude mode produces exactly the complementary set of events.
- **SC-009**: After hiding any combination of columns and restarting the application, the same
  columns are hidden and no row shows a gap where a hidden column was.
- **SC-010**: Every setting changed in a session is still in effect after quitting and relaunching.
- **SC-011**: The application starts and runs correctly on a machine with no MIDI devices, drivers,
  or ports available.

## Assumptions

- **"Mock" means simulated traffic, not a non-functional shell.** All controls genuinely operate on
  a live, generated event stream; only the origin of the MIDI data is simulated. No real MIDI I/O is
  in scope.
- **"Hide some of the row" is read as column visibility.** The phrase follows the list of the row's
  five fields, and hiding whole rows by content is already covered by the source, message-type, and
  hex-prefix filters — so this is interpreted as choosing which of the five fields a row displays.
- The virtual sources are a fixed set defined by the application, named to match the reference
  screenshots (`IAC Driver Bus 1`, `MidiKeys`, and the `Spy on output to destinations` entry).
  Simulated device hot-plug — sources appearing or disappearing at runtime — is out of scope.
- `Act as a destination for other programs` and `Spy on output to destinations` are reproduced as
  selectable mock source entries matching the reference layout; they perform no real inter-
  application MIDI routing.
- The reference screenshots are macOS; the application reproduces the same layout, labels, grouping,
  and control types, adapted to its own platform's native look rather than imitating macOS chrome
  pixel-for-pixel.
- The window title in the reference reads `MIDI Monitor Example`; the application titles its window
  `MIDI Monitor`.
- Hex prefixes are matched against raw MIDI bytes including the status byte, which is why `90`
  selects channel-1 Note On. The Data column's rendered form (e.g. `C2 127`) is a display of those
  same bytes.
- `Invalid` in the filter panel refers to malformed or unrecognised MIDI data; the simulator
  produces occasional invalid messages so the control is exercisable.
- Retained events are session-only; only settings persist. The window is single-instance with one
  event list — multiple simultaneous monitor windows are out of scope.
- Per the project constitution, this specification defines acceptance scenarios as **manual
  verification steps performed against the running application**. No automated test files, test
  dependencies, or test harnesses are to be produced for them.
