# Feature Specification: A Preferences Surface, With Display Formats Implemented

**Feature Branch**: `006-display-preferences`

**Created**: 2026-09-23

**Status**: Draft

**Input**: User description: "i want to add to the app a setting mode, with three screen display sources and other. and for now only implement the logic for Display. the featues and view that should be avaliable are as show at the screenshot here screenshots/setting.jpg . extract everying there"

## Overview

The monitor shows every message it receives in exactly one way. Time is wall-clock. Notes are
names. Controllers are bare numbers. Values are decimal. Those choices were made once, in code,
and the user cannot change any of them — which is wrong for the two people this application is
for: the one who wants hexadecimal because that is what the device's manual prints, and the one
who wants raw host timestamps because they are measuring latency rather than reading a log.

This feature adds a **third mode of the application, alongside Monitor and Send**, reproducing
the preferences window in `screenshots/setting.jpg`. That window has three tabs — `Display`,
`Sources`, `Other`. All three appear. **Only `Display` is implemented in this feature**, by the
user's explicit instruction; `Sources` and `Other` are named now and filled later.

The `Display` tab holds six controls, and every one of them changes how the monitor renders
what it has already received:

- **Time format** — wall-clock time, or the host clock in three forms.
- **Note format** — a note name under either octave convention, or the raw number in decimal
  or hexadecimal.
- **Controller format** — the controller's standard name, or its number in decimal or
  hexadecimal.
- **Data format** — decimal or hexadecimal, for every remaining value.
- **Program number** — programs counted from 1, or from 0.
- **Expert mode** — a single checkbox that suppresses the three conveniences the monitor
  currently applies without asking, so the user sees what actually arrived rather than what the
  application decided it meant.

Three things this feature is deliberately **not**.

It is not a change to the monitor's surface. The reference images `screenshots/data.png`,
`screenshots/filters.png`, and `screenshots/sources.png` remain authoritative for every control
they depict: nothing is relabelled, reordered, restyled, or repurposed. What changes is the
*contents* of the `Time`, `Data`, and `Message` cells — which is the entire point of a display
preference — not the table that holds them.

It is not a capture-time setting. Changing a format does not ask the user to re-run the traffic
that prompted the question. The rows already on screen re-render.

It is not the `Sources` or `Other` tab. Those are scoped out here by instruction, and the
specification says only what must be true of them so that they do not mislead a user who clicks
them.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Read the traffic in hexadecimal because the manual is in hexadecimal (Priority: P1)

The user is chasing a controller message against a device manual that prints everything in hex.
The monitor shows `7` where the manual says `07`, and `100` where the manual says `64`. They
open the preferences mode, choose `Hexadecimal number` under `Data format`, and return to the
monitor. Every value in the rows already captured is now hexadecimal, and so is everything that
arrives next. They can read the table and the manual side by side without converting in their
head.

**Why this priority**: This is the shortest path to the feature's value, it stands entirely
alone, and it is the reason a display-preferences surface exists at all. With only this built
the feature already earns its place. It also exercises the whole spine the other stories reuse —
a preference is chosen, it is persisted, and the monitor re-renders retained rows to match.

**Independent Test**: With events retained in the monitor, open preferences, switch
`Data format` to `Hexadecimal number`, return to the monitor, and confirm that the retained rows
and newly arriving rows both show hexadecimal values. Switch back and confirm they return to
decimal.

**Acceptance Scenarios**:

1. **Given** the monitor holds retained events rendered in decimal, **When** the user selects
   `Hexadecimal number` under `Data format`, **Then** the values in the retained rows are shown
   in hexadecimal without the events being re-received or cleared.
2. **Given** `Data format` is `Hexadecimal number`, **When** a new message arrives, **Then** its
   values are shown in hexadecimal.
3. **Given** `Data format` is `Hexadecimal number`, **When** the user quits and relaunches the
   application, **Then** `Hexadecimal number` is still selected and the monitor still renders in
   hexadecimal.
4. **Given** `Data format` is `Hexadecimal number`, **When** the user switches it back to
   `Decimal number`, **Then** every retained and newly arriving value is shown in decimal again.

---

### User Story 2 - Reach the preferences without disturbing the monitor (Priority: P1)

The user is watching live traffic. They open preferences, look at a setting, and come back. The
monitor kept running the whole time — nothing was missed, nothing was cleared, the filters and
source selections they had set are exactly as they left them, and if they had paused the monitor
it is still paused.

**Why this priority**: It is P1 alongside the first story because a preferences surface that
costs the user their session is worse than no preferences surface. The application already made
this promise when the send screen was added, and this feature must not be the one that breaks
it. It is independently testable and delivers value on its own: the mode is reachable and
leaving it is free.

**Independent Test**: Start the monitor with traffic arriving, switch to the preferences mode,
wait, and switch back. Confirm that events which arrived while away are present, and that
filters, column choices, source selections, retention, and paused state are unchanged.

**Acceptance Scenarios**:

1. **Given** traffic is arriving and the monitor is capturing, **When** the user switches to the
   preferences mode and back, **Then** the messages that arrived while the preferences mode was
   showing are present in the monitor.
2. **Given** the monitor is paused, **When** the user switches to the preferences mode and back,
   **Then** the monitor is still paused and holds the same retained rows.
3. **Given** filters, visible columns, source selections, and a retention limit have been set,
   **When** the user visits the preferences mode and returns, **Then** all of them are unchanged.
4. **Given** the user is in the preferences mode, **When** they look at the mode switcher,
   **Then** every control the monitor's reference screenshots depict is unchanged in label,
   type, order, and default.

---

### User Story 3 - Choose how notes, controllers, and programs are named (Priority: P2)

The user's sequencer calls middle C `C3`; the monitor calls it `C4`. Their manual lists
controller 7 as `Volume`; the monitor shows `7`. Their synthesiser's patch list starts at
program `1`; the monitor shows `0`. In the preferences mode they set `Note format`,
`Controller format`, and `Program number` to match the equipment in front of them, and the
monitor's vocabulary now agrees with everything else on the desk.

**Why this priority**: P2 rather than P1 because it is the same mechanism as the first story
applied to three more controls, so it depends on that spine existing — but it is where most of
the daily friction actually is, because an octave that is off by one or a program that is off by
one silently misleads rather than merely inconveniences.

**Independent Test**: Capture a Note On, a Control Change, and a Program Change. Change each of
the three settings in turn and confirm the corresponding cell changes as specified and the other
two cells do not.

**Acceptance Scenarios**:

1. **Given** a Note On for note number 60 is retained, **When** `Note format` is
   `Note (Middle C = C3)`, **Then** the note reads `C3`; **and when** it is
   `Note (Middle C = C4)`, **Then** it reads `C4`.
2. **Given** a Note On for note number 60 is retained, **When** `Note format` is
   `Decimal number`, **Then** the note reads `60`; **and when** it is `Hexadecimal number`,
   **Then** it reads as that number in hexadecimal.
3. **Given** a Control Change for controller 7 is retained, **When** `Controller format` is
   `Standard name`, **Then** the controller is shown by its standard name; **and when** it is
   `Decimal number`, **Then** it reads `7`.
4. **Given** a Control Change for a controller number that has no standard name is retained,
   **When** `Controller format` is `Standard name`, **Then** the controller is still identified
   unambiguously rather than shown blank.
5. **Given** a Program Change carrying program value 0 is retained, **When** `Program number` is
   `1 – 128 (Standard)`, **Then** it reads `1`; **and when** it is `0 – 127 (Less common)`,
   **Then** it reads `0`.
6. **Given** `Note format` is changed, **When** the user looks at the Data column, **Then** the
   velocity alongside the note is still rendered per `Data format` and is unaffected.

---

### User Story 4 - Time the traffic rather than read it (Priority: P2)

The user is not reading messages, they are measuring them — how far apart two events really
were, whether a device's clock is drifting. Wall-clock time to the millisecond is too coarse and
the wrong unit. They choose one of the three host-time formats and the `Time` column becomes the
host clock in the form they asked for.

**Why this priority**: P2 because it serves a narrower purpose than the format settings and the
monitor remains fully usable without it, but it is the only setting on the tab that a
reformatting of existing values cannot satisfy — it needs the arrival moment recorded in a form
the application does not currently keep, so it is called out as its own story rather than folded
into the first.

**Independent Test**: Capture events, switch `Time format` through each of its four options, and
confirm the `Time` column changes form each time, with the same event keeping a self-consistent
value across the three host-time forms.

**Acceptance Scenarios**:

1. **Given** events are retained, **When** `Time format` is `Clock time`, **Then** the `Time`
   column shows wall-clock time as the monitor's reference screenshots depict it.
2. **Given** events are retained, **When** `Time format` is `Host time (integer)`, **Then** the
   `Time` column shows the host clock value as a whole number.
3. **Given** events are retained, **When** `Time format` is `Host time (seconds)`, **Then** the
   `Time` column shows the host clock value expressed in seconds.
4. **Given** events are retained, **When** `Time format` is `Host time (nanoseconds)`, **Then**
   the `Time` column shows the host clock value expressed in nanoseconds.
5. **Given** an event is retained, **When** the user switches between the three host-time
   formats, **Then** the three values shown are consistent conversions of one another for that
   same event.

---

### User Story 5 - Turn off the application's helpfulness (Priority: P3)

The user suspects the monitor is lying to them by being helpful. Their device sends a Note On
with velocity zero and the monitor calls it a Note Off; they need to know which one actually
came down the wire. They tick `Expert mode`, and the three conveniences listed beneath the
checkbox stop being applied: values are shown raw rather than formatted, a Note On with velocity
zero is reported as what it was, and a zero timestamp is left as zero instead of being replaced
with the time of receipt.

**Why this priority**: P3 because it is the narrowest audience on the tab and everything above it
is useful without it — but it is the setting that makes the monitor trustworthy for the one user
who has a reason to distrust it, which is why it is in the reference window at all.

**Independent Test**: Send a Note On with velocity zero, confirm the monitor reports it as a
Note Off, tick `Expert mode`, and confirm the same retained row now reports it as a Note On.

**Acceptance Scenarios**:

1. **Given** a Note On with velocity zero is retained and `Expert mode` is unticked, **When** the
   user reads its `Message` cell, **Then** it is reported as a note release, as the monitor does
   today.
2. **Given** that same retained event, **When** the user ticks `Expert mode`, **Then** it is
   reported as a Note On with velocity zero, without the event being re-received.
3. **Given** `Expert mode` is ticked, **When** the user reads any data value, **Then** it is
   shown raw rather than formatted according to the settings above it.
4. **Given** an event carrying a zero timestamp and `Expert mode` unticked, **When** the user
   reads its `Time` cell, **Then** it shows the time the message was received; **and when**
   `Expert mode` is ticked, **Then** it shows zero.
5. **Given** `Expert mode` is ticked, **When** the user unticks it, **Then** all three
   conveniences are applied again to retained and arriving events alike.

---

### User Story 6 - Find out that Sources and Other are not built yet (Priority: P3)

The user clicks `Sources`, then `Other`. Both tabs are there — the reference window has three
and so does this one. Neither presents a control that looks operable and does nothing. Each says
plainly that its settings are not available yet, so the user learns the truth in one click
instead of setting something that is silently ignored.

**Why this priority**: P3 because it adds no capability — but the alternative, shipping two tabs
full of inert controls, is the failure mode the application has an explicit rule against, so it
is specified rather than left to judgement.

**Independent Test**: Open the preferences mode, click `Sources` and then `Other`, and confirm
each is selectable, each states that it is not yet available, and neither offers a control that
can be operated.

**Acceptance Scenarios**:

1. **Given** the preferences mode is showing, **When** the user looks at the tabs, **Then**
   `Display`, `Sources`, and `Other` are all present, in that order, with `Display` selected.
2. **Given** the preferences mode is showing, **When** the user selects `Sources` or `Other`,
   **Then** that tab is selected and states that its settings are not yet available.
3. **Given** the user is on `Sources` or `Other`, **When** they look at the panel, **Then** it
   offers no operable control.
4. **Given** the user selects `Sources` or `Other` and then `Display` again, **Then** the
   `Display` tab shows exactly the settings that were in effect, unchanged.

---

### Edge Cases

- **A setting is changed while the monitor is paused.** The retained rows re-render in the new
  format; pausing governs whether new events are admitted, not how retained ones are drawn.
- **A setting is changed while the monitor is at its retention limit.** Re-rendering must not
  admit, drop, or reorder any event; the same rows are shown differently.
- **A controller number with no standard name, under `Standard name`.** The controller is still
  identified — a name is an addition to the number, never a replacement that loses it.
- **A note number under `Note (Middle C = C3)` at the extremes.** Note 0 and note 127 both
  produce a name; the octave numbering runs negative at the bottom rather than failing.
- **A malformed message, under any format.** A row the decoder could not interpret still shows
  its reason and its byte count; display settings change how *values* are drawn, not whether
  uninterpretable traffic is shown.
- **A System Exclusive transfer, under `Hexadecimal number`.** Its `Data` cell continues to
  report the transfer's size rather than becoming an unbounded byte dump.
- **Settings written by an earlier version of the application.** A stored configuration with no
  display settings in it loads successfully, supplies the defaults, and preserves every other
  setting the user had — filters, columns, retention, source selections, and the send screen's
  state.
- **A stored display setting that is no longer recognised.** It falls back to that setting's
  default without discarding the rest of the stored configuration.
- **`Expert mode` interacting with the format settings.** `Expert mode` overrides them for data
  values while it is ticked; unticking it restores them exactly as they were, without the user
  having to re-choose.
- **The send screen while display settings change.** Its composer, request library, and publish
  log are unaffected; this feature governs how received traffic is read.

## Requirements *(mandatory)*

### Functional Requirements

#### The preferences mode

- **FR-001**: The application MUST offer a third mode alongside `Monitor` and `Send`, reached
  the same way those two are, presenting the preferences surface.
- **FR-002**: Entering or leaving the preferences mode MUST NOT stop capture, clear retained
  events, or alter the monitor's filters, visible columns, source selections, retention limit,
  or paused state.
- **FR-003**: Entering or leaving the preferences mode MUST NOT alter the send screen's target,
  publication, composed message, or saved requests.
- **FR-004**: The preferences surface MUST present exactly three tabs, labelled `Display`,
  `Sources`, and `Other`, in that order, with `Display` selected when the mode is first opened.
- **FR-005**: The `Sources` and `Other` tabs MUST be selectable, MUST state that their settings
  are not yet available, and MUST NOT present any operable control.
- **FR-006**: Selecting a different tab and returning MUST leave the settings in effect
  unchanged.
- **FR-007**: The feature MUST NOT relabel, reorder, restyle, remove, or repurpose any control
  depicted in `screenshots/data.png`, `screenshots/filters.png`, or `screenshots/sources.png`.

#### Fidelity to the reference image

- **FR-008**: The `Display` tab MUST present the following controls, with these labels
  reproduced verbatim, in this order and grouping:
  - `Time format` — a radio group of `Clock time`, `Host time (integer)`,
    `Host time (seconds)`, `Host time (nanoseconds)`.
  - `Note format` — a radio group of `Note (Middle C = C3)`, `Note (Middle C = C4)`,
    `Decimal number`, `Hexadecimal number`.
  - `Controller format` — a radio group of `Standard name`, `Decimal number`,
    `Hexadecimal number`.
  - `Data format` — a radio group of `Decimal number`, `Hexadecimal number`.
  - `Program number` `(Decimal)` — a radio group of `1 – 128 (Standard)`,
    `0 – 127 (Less common)`.
  - `Expert mode` — a checkbox.
- **FR-009**: Each radio group MUST behave as a radio group: exactly one option selected at all
  times, and selecting one deselects the others.
- **FR-010**: Beneath the `Expert mode` checkbox the surface MUST show the three explanatory
  lines from the reference image verbatim, as a bulleted list, indented beneath the checkbox:
  `Data formatted according to settings above`,
  `Note On with velocity 0 shows as Note Off`,
  `Zero timestamp shows time received`.
- **FR-011**: The first-launch defaults MUST be the states the reference image depicts:
  `Clock time`, `Note (Middle C = C3)`, `Standard name`, `Decimal number`,
  `1 – 128 (Standard)`, and `Expert mode` unticked.
- **FR-011a**: `screenshots/setting.jpg` is the authority for the note format default, in
  preference to the octave convention the monitor uses today. The monitor renders notes under
  the middle-C-equals-C4 convention at present, justified in its own documentation as matching
  `screenshots/data.png`; that justification is superseded. On first launch after this feature,
  a note the monitor rendered as `C2` MUST render as `C1`. This is a deliberate change to an
  existing rendering, and the reasoning MUST be recorded where the superseded justification
  lives so the next reader does not restore it as a defect.
- **FR-012**: No control MUST be added to the `Display` tab beyond those the reference image
  depicts.

#### What each setting governs

- **FR-013**: `Time format` MUST govern the `Time` column's contents for every event:
  `Clock time` shows wall-clock arrival time in the form the monitor shows today; the three
  `Host time` options show the arrival moment on the host clock as a whole number, in seconds,
  and in nanoseconds respectively.
- **FR-014**: The three `Host time` renderings of one event MUST be consistent conversions of a
  single underlying arrival moment.
- **FR-015**: `Note format` MUST govern how note numbers are rendered wherever a message carries
  one — Note On, Note Off, and Aftertouch (Poly) — producing a note name under the chosen octave
  convention, or the raw number in decimal or hexadecimal.
- **FR-016**: Under `Note (Middle C = C3)`, note number 60 MUST render as `C3`; under
  `Note (Middle C = C4)`, note number 60 MUST render as `C4`. Every note number from 0 to 127
  MUST produce a name under both conventions.
- **FR-017**: `Controller format` MUST govern how a Control Change's controller number is
  rendered: its standard name, or the number in decimal or hexadecimal.
- **FR-018**: Under `Standard name`, a controller number with no standard name MUST still be
  identified unambiguously rather than rendered blank or as a placeholder.
- **FR-019**: `Data format` MUST govern the rendering of every remaining data value the monitor
  displays — velocities, controller values, pressures, pitch bend amounts, song positions, song
  selections, time-code values — in decimal or hexadecimal.
- **FR-020**: `Program number` MUST govern how a Program Change's program is rendered: counted
  from 1 through 128, or from 0 through 127.
- **FR-021**: Each setting MUST govern only what it names; changing one MUST NOT alter the
  rendering the other five govern.
- **FR-022**: `Expert mode`, when ticked, MUST suppress all three of the conveniences listed
  beneath it: data values are shown raw rather than formatted per the settings above; a Note On
  with velocity zero is reported as a Note On rather than a note release; a zero timestamp is
  shown as zero rather than as the time of receipt.
- **FR-023**: Unticking `Expert mode` MUST restore all three conveniences and MUST restore the
  five format settings to the selections the user had made, without requiring them to be
  re-chosen.

#### Applying and remembering

- **FR-024**: Changing any display setting MUST take effect immediately, without the user
  confirming, dismissing, or reopening anything.
- **FR-025**: Changing any display setting MUST re-render the events the monitor has already
  retained, not only those that arrive afterwards. This holds for all six settings without
  exception, `Time format` included.
- **FR-025a**: Because a retained event must be able to render in a format chosen after it was
  captured, an event MUST carry enough of what arrived to satisfy any of the six settings at any
  later moment. An event captured while `Clock time` was selected MUST be able to show host time
  once the user selects it, without the event being re-received.
- **FR-026**: Re-rendering MUST NOT admit, drop, reorder, or duplicate any retained event, and
  MUST NOT alter the monitor's scroll position, selection, or paused state.
- **FR-027**: Changing a display setting MUST NOT alter which events pass the monitor's filters;
  filtering is over what arrived, and display settings govern only how it is drawn.
- **FR-028**: All six display settings MUST persist across a restart and MUST be in effect when
  the application next launches.
- **FR-029**: A stored configuration written before this feature existed MUST load successfully,
  supply the first-launch defaults for the six display settings, and preserve every other stored
  setting unchanged.
- **FR-030**: A stored display setting that cannot be recognised MUST fall back to that
  setting's default without discarding the rest of the stored configuration.

### Key Entities

- **Display settings** — the six choices the `Display` tab holds: time format, note format,
  controller format, data format, program numbering, and expert mode. One set for the
  application, not one per source; outlives the session.
- **Time format** — which of four renderings the `Time` column uses.
- **Note format** — which of four renderings a note number uses.
- **Controller format** — which of three renderings a controller number uses.
- **Data format** — which of two renderings every remaining value uses.
- **Program numbering** — whether programs are counted from 1 or from 0.
- **Expert mode** — whether the monitor's three interpretive conveniences are applied.
- **Standard controller name** — the conventional name for a controller number, where one
  exists.
- **Preferences tab** — one of `Display`, `Sources`, `Other`; which one is showing is a property
  of the moment, not a stored setting.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user who wants values in hexadecimal can get them in under 15 seconds from a
  running monitor, with no more than four interactions and no typing.
- **SC-002**: Changing any display setting is reflected in the already-retained rows within one
  second, with no visible reload, flicker of the table, or loss of position.
- **SC-003**: 100% of the controls, labels, groupings, order, and default states in
  `screenshots/setting.jpg` are reproduced on the `Display` tab, verified by side-by-side
  comparison.
- **SC-004**: Every one of the 128 note numbers renders a value under all four `Note format`
  options, and every one of the 128 controller numbers renders an identifying value under all
  three `Controller format` options — no blanks, no placeholders, no failures.
- **SC-005**: A user who leaves the monitor capturing, visits the preferences mode, and returns
  loses zero events and finds zero monitor settings changed.
- **SC-006**: All six settings survive a restart with 100% fidelity, and a configuration written
  by the previous version loads with zero loss of any other setting.
- **SC-007**: A user who clicks `Sources` or `Other` learns in one interaction that those
  settings are not yet available, and finds zero controls there that appear operable.
- **SC-008**: With `Expert mode` ticked, a Note On with velocity zero is distinguishable from a
  Note Off in 100% of cases.

## Assumptions

- **The preferences surface is a third mode of the one window, not a separate window.** The
  reference image shows a macOS preferences window with its own title bar reading
  `MIDI Monitor Preferences`. The user asked for a "setting mode", the application already
  switches between a `Monitor` mode and a `Send` mode, and native window chrome is exactly the
  kind of platform detail the project's design-authority rule permits deviating on. The window's
  title bar and its close/minimise/zoom controls are therefore not reproduced; everything inside
  it is.
- **`Expert mode` suppresses the three listed behaviours rather than enabling them.** The three
  lines beneath the checkbox describe what the monitor does for the user when the checkbox is
  unticked — which matches what the application does today, since it already converts a Note On
  with velocity zero into a note release. Ticking `Expert mode` turns that help off and shows
  what arrived. The alternative reading, that the list describes what ticking the box turns on,
  would make the unticked state — the default the reference image shows — the one where data is
  *not* formatted according to the settings above it, which would make the five settings above
  it inert by default.
- **One set of display settings applies to the whole application.** The reference image shows no
  per-source or per-column scoping.
- **The `Sources` tab of this preferences window is not the monitor's existing source panel.**
  The monitor already has a `Sources` section from `screenshots/sources.png`; this tab is a
  separate, unimplemented surface, and this feature does not move, duplicate, or alter the
  existing panel.
- **Standard controller names are the conventional MIDI ones** — the names published for the
  defined controller numbers — with a number that has no defined name still shown by number.
- **The host clock is the operating system's high-resolution timer for the machine the
  application is running on**, and the three `Host time` options are three renderings of one
  captured value rather than three separately captured values.
- **The existing hexadecimal prefix filter is unaffected.** It matches against raw bytes and
  always has; `Data format` governs display only, so a filter entered before a format change
  keeps matching the same events.
- **A retained event is a record of what arrived, not a set of finished cells.** Because any of
  the six settings may be chosen after an event was captured (FR-025, FR-025a), an event must
  carry what arrived — including its arrival moment on the host clock — from the instant it is
  captured, whether or not the setting that needs it is currently selected. `Time format` is the
  setting that makes this visible, since host time is information the monitor does not retain
  today, but the rule is general.
- **The two reference images disagree on note octaves, and `screenshots/setting.jpg` wins.**
  `screenshots/data.png` shows `C2` and `B2`; the monitor produces those under the
  middle-C-equals-C4 convention and its code cites that image as the reason. This feature adopts
  the preferences image's `Note (Middle C = C3)` default instead, so the monitor's first-launch
  note names shift down one octave and `screenshots/data.png` no longer matches the monitor's
  default state for the Data column's note names. That image remains authoritative for
  everything else it depicts — columns, their order, the disclosure sections, the retention row,
  and the `Clear` button. A user who wants the old naming selects `Note (Middle C = C4)`.
- **No migration of stored settings is required**, only a default for absent display settings —
  the same additive shape the send screen's settings used.
