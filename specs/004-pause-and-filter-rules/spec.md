# Feature Specification: Pause the View and Build Several Data Prefix Rules

**Feature Branch**: `004-pause-and-filter-rules`

**Created**: 2026-08-29

**Status**: Draft

**Input**: User description: "i want to add to the app several functional one of them is the ability to puase without clearing alrady recived data, and second an ability to add and delete several "include only start with/ exclude start with" and siplay an error when you try to add a filer rule that clas with one existing"

## Overview

Two additions to the monitor, plus the error reporting the second one needs.

**Pausing.** Today the only way to stop the list moving is to stop watching a source or to
press `Clear`, and both destroy what is on screen. A monitor is read while traffic is
arriving, so the user needs to freeze the list, read a row, and let it run again — without
losing a single event they had already received. While paused the monitor takes nothing new
in; what had already arrived stays exactly as it was, and traffic that passes during the pause
is not recorded.

**Several data prefix rules.** Today the `Data starts with` row is one text field plus one
`Show only` / `Hide` choice that applies to every prefix typed into it. Every prefix shares
that one mode, dropping one means retyping the rest, and nothing on screen lists what is
actually in force. This feature turns that one entry into a list of rules, each added on its
own, deleted on its own, and carrying its own kind.

**Clash reporting.** Once rules are a list, two rules can repeat or contradict each other.
Adding one that clashes with a rule already in the list must be refused with an explanation,
and the list must be left exactly as it was. The list is kept free of contradictions by
refusing them at the door rather than by arbitrating between them afterwards.

Nothing in this feature changes what the reference screenshots depict. `Sources`, the `Filter`
checkbox columns, the channel radio pair, `Remember up to … events`, `Clear`, and the five
table columns keep their labels, order, control types, and defaults.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Freeze the list to read what has already arrived (Priority: P1)

A controller is sending a dense stream. Something interesting goes past. The user pauses the
monitor, and the rows stop moving. Every event that had already been received is still listed,
still scrollable, still readable — the same rows, in the same order, with the same timestamps.
Nothing arriving during the pause is taken in, so nothing can push those rows out. The user
reads what they needed, then resumes, and the monitor carries on from the traffic arriving
after that moment.

**Why this priority**: This is the primary request and it stands alone. It delivers value with
no other part of this feature built, and without it a fast stream is unreadable — the only
current alternatives throw away the very data the user is trying to read.

**Independent Test**: Attach any source that produces continuous traffic, let rows accumulate,
pause, and confirm the visible rows stop changing and none of the previously received rows
disappear. Resume and confirm the monitor is live again.

**Acceptance Scenarios**:

1. **Given** the monitor is running and showing received events, **When** the user pauses,
   **Then** the visible rows stop changing and every row that was on screen is still there.
2. **Given** the monitor is paused, **When** the user scrolls the list or hides a column,
   **Then** the retained events remain and none are discarded.
3. **Given** the monitor is paused and the source keeps sending, **When** the user waits long
   enough that the retention cap would have been reached several times over, **Then** the
   retained events are still exactly the ones that had arrived before the pause, unchanged and
   in the same order.
4. **Given** the monitor is paused, **When** the user resumes, **Then** the list becomes live
   again, the events retained before the pause are still listed, and new rows appear from
   traffic arriving after the resume. Traffic that passed during the pause is not recovered.
5. **Given** the monitor is paused, **When** the user changes a filter (a message-kind
   checkbox, the channel rule, or a prefix rule), **Then** the visible rows are re-derived from
   the retained events under the new filter and the monitor stays paused.
6. **Given** the monitor is paused, **When** the user presses `Clear`, **Then** the retained
   events are discarded as they always were and the monitor stays paused.
7. **Given** the monitor is paused, **When** the application is quit and relaunched,
   **Then** the monitor starts running, not paused.

---

### User Story 2 - Keep a list of data prefix rules, adding and deleting them one at a time (Priority: P1)

The user types a hex prefix, chooses whether it is a `Show only` rule or a `Hide` rule, and
adds it. It appears in a list of rules with its kind shown. They add another. Each rule can be
deleted on its own, leaving the others in force, without retyping anything. With no rules in
the list, every event passes the data filter, exactly as an empty field does today.

**Why this priority**: The second explicitly requested capability, and independently valuable
on its own: what is being filtered becomes readable on screen, and a single rule can be dropped
from a set without disturbing the rest — neither of which the current single field allows.

**Independent Test**: Add two rules, confirm both are listed and both are in effect on the
visible rows, delete one, and confirm the other is still applied and the deleted one is not.

**Acceptance Scenarios**:

1. **Given** an empty rule list, **When** the user adds a valid prefix as a `Show only` rule,
   **Then** the rule is listed and only events whose raw bytes start with that prefix are shown.
2. **Given** a rule list holding one `Hide` rule, **When** the user adds a second, non-clashing
   `Hide` rule, **Then** both are listed and events matching either prefix are hidden.
3. **Given** a list of several rules, **When** the user deletes one, **Then** that rule is gone
   from the list, every other rule is untouched, and the visible rows are re-derived immediately.
4. **Given** a list of several rules, **When** the user deletes the last remaining rule,
   **Then** every retained event that passes the other filters is shown again.
5. **Given** the user types a prefix containing a character that is not a hexadecimal digit,
   **When** they try to add it, **Then** the rule is refused with an explanation, the list is
   unchanged, and the rules already in force keep running.
6. **Given** a list of rules, **When** the application is quit and relaunched, **Then** the same
   rules are listed, in the same order, with the same kinds.

---

### User Story 3 - Be told, and told why, when a new rule clashes with one already in the list (Priority: P2)

The user adds a rule that repeats or contradicts one already in the list. The monitor refuses
it, says which existing rule it clashes with and why, and leaves the list exactly as it was.
Nothing about the visible events changes. The user can correct the entry and add it again.

**Why this priority**: It protects the list built in User Story 2 from silently contradicting
itself, but User Story 2 is usable before it exists. It is a guard on that capability rather
than a capability of its own.

**Independent Test**: With a rule already in the list, attempt to add one that clashes with it,
and confirm an error naming the conflict is shown and the list is unchanged.

**Acceptance Scenarios**:

1. **Given** a rule list containing a rule, **When** the user tries to add a rule that clashes
   with it, **Then** the addition is refused, an error identifies the existing rule and the
   nature of the clash, and the list and the visible rows are unchanged.
2. **Given** a clash error is being shown, **When** the user edits the entry into a
   non-clashing rule and adds it, **Then** the rule is added and the error is no longer shown.
3. **Given** a clash error is being shown, **When** the user deletes the existing rule that was
   clashed with and adds the new rule again, **Then** the rule is added.
4. **Given** a rule list, **When** an addition is refused for any reason, **Then** no rule is
   partially added and the filter in force is exactly the one that was in force before.

---

### Edge Cases

- **Pausing with an empty list.** Pausing before any event has arrived freezes nothing; on
  resume the list fills as usual. Pause is not an error state.
- **Pausing with no source selected.** Pause and the existing "not monitoring" explanation are
  separate conditions and must both remain understandable — a paused monitor with no source
  selected is not described as merely paused.
- **The retention cap while paused.** Nothing new is retained while paused, so the cap cannot
  evict a frozen row however long the pause lasts or however dense the traffic is. This is the
  reason pausing stops retention rather than merely stopping the display.
- **A long pause on a dense stream.** Traffic that passes during a pause is gone; resuming does
  not backfill it. That is a deliberate consequence of freezing what is retained, and the user
  needs to be able to anticipate it from the control rather than discover it afterwards.
- **Lowering the retention cap while paused.** The cap is enforced immediately today. Frozen
  rows beyond a newly lowered cap are discarded like any other excess, and the list shortens.
- **Deselecting a source while paused.** Retained events from a deselected source are dropped
  today. That behaviour is unchanged by pause: the frozen list shortens accordingly.
- **A rule that is a prefix of an existing rule.** `9` and `90` overlap without being equal.
  Of the opposite kind that is a clash and is refused (`Show only 90` alongside `Hide 9`, or
  `Show only 90` alongside `Hide 9012`). Of the same kind it is permitted: `Show only 9`
  alongside `Show only 90` is redundant but says nothing contradictory.
- **Hide rules alongside Show only rules.** Because overlapping opposite-kind rules are
  refused, no event can ever match both kinds. A `Show only` rule therefore already excludes
  everything a `Hide` rule could have caught, and while any `Show only` rule is in the list the
  `Hide` rules have nothing left to remove.
- **Case and whitespace.** `9a`, `9A`, and `9 A` are the same prefix. Clash detection compares
  normalised prefixes, so entering the same prefix in different notation is still a clash.
- **An empty entry.** Attempting to add a rule with no hexadecimal digits is refused with an
  explanation; it is not silently ignored and it does not add a blank rule.
- **Every event filtered out.** A rule list that admits nothing shows an empty table. The user
  must be able to tell that from "nothing has arrived" — the existing distinction between a
  filtered-empty list and an unmonitored one continues to apply.
- **Deleting a rule while paused.** Filtering is a view over retained events, so the frozen
  list is re-derived under the new rule set without resuming and without losing events.

## Requirements *(mandatory)*

### Functional Requirements

#### Pausing the View

- **FR-001**: Users MUST be able to pause the monitor and resume it, using a control that is
  available whenever the application is running.
- **FR-002**: The control MUST show which state the monitor is in, so that a frozen list is
  never mistaken for a stalled or broken one.
- **FR-003**: Pausing MUST NOT discard, reorder, or alter any event that has already been
  received. Timestamps, source, message, channel, and raw data of the frozen rows stay as they
  were.
- **FR-004**: While paused, the set of visible rows MUST NOT change in response to arriving
  MIDI traffic.
- **FR-005**: Resuming MUST return the monitor to live display without discarding the events
  already retained.
- **FR-006**: While paused, every filter — message kinds, channel, columns, and data prefix
  rules — MUST remain operable, and changing one MUST re-derive the visible rows from the
  retained events without resuming.
- **FR-007**: `Clear` MUST behave while paused exactly as it does while running: it discards
  retained events and leaves the paused state unchanged.
- **FR-008**: Pausing MUST NOT close, reopen, or otherwise disturb the connection to any
  selected source, and MUST NOT change which sources are selected.
- **FR-009**: While the monitor is paused, arriving events MUST NOT be retained. They are
  discarded on arrival, not buffered.
- **FR-010**: Because nothing is retained while paused, the retention cap MUST NOT evict any
  event that was retained before the pause, no matter how long the pause lasts or how dense the
  incoming traffic is. The events already received are what pausing preserves.
- **FR-011**: Resuming MUST NOT recover or backfill events that arrived during the pause. The
  list continues with traffic arriving after the resume, appended after the retained events.
- **FR-012**: The paused state MUST NOT persist across restarts: a freshly launched monitor is
  running.

#### The Data Prefix Rule List

- **FR-013**: The data prefix filter MUST be a list of individually added rules, replacing the
  single entry field and single mode selector in force today.
- **FR-014**: Each rule MUST carry exactly one prefix and exactly one kind: show only events
  matching it, or hide events matching it. A rule with no kind, or with two, MUST NOT be
  expressible.
- **FR-015**: Users MUST be able to add a rule by entering a prefix and choosing its kind.
- **FR-016**: Users MUST be able to delete any single rule without affecting the others.
- **FR-017**: Every rule currently in the list MUST be visible with its prefix and its kind, so
  that what is being filtered is readable without opening anything.
- **FR-018**: An empty rule list MUST admit every event, matching today's behaviour for an
  empty prefix field.
- **FR-019**: Prefixes MUST continue to be matched against the raw bytes as hexadecimal
  nibbles, so that `9` matches `90` through `9F` — unchanged from today.
- **FR-020**: A prefix entry MUST be normalised before it is stored or compared: whitespace
  removed and letters upper-cased, as today.
- **FR-021**: An entry that is empty of hexadecimal digits, or that contains any character that
  is neither a hexadecimal digit nor whitespace, MUST be refused with an explanation, and the
  rule list MUST be left unchanged.
- **FR-022**: An event MUST be shown when both of these hold, and hidden otherwise:
  1. it matches no `Hide` rule in the list; and
  2. either the list holds no `Show only` rule, or the event matches at least one of them.
- **FR-023**: The two conditions in FR-022 MUST NOT be able to disagree about the same event.
  The clash rule (FR-027) refuses any pair of opposite-kind rules where one prefix is a prefix
  of the other, and two prefixes can only match the same bytes when one is a prefix of the
  other — so no event can match both a `Show only` rule and a `Hide` rule. Contradictions are
  prevented when a rule is added rather than resolved when an event is judged, which is why
  no precedence between the two kinds needs to exist.
- **FR-024**: It follows from FR-022 and FR-023 that while any `Show only` rule is in the list,
  the `Hide` rules cannot change what is shown — everything they would remove has already been
  excluded for matching no `Show only` rule. This MUST be treated as expected behaviour, not as
  a defect to be worked around by changing the combination rule.
- **FR-025**: The rule list MUST persist across restarts, as the prefix filter does today, and
  a filter saved by an earlier version MUST come back as rules of the kind it was saved with.
- **FR-026**: Adding or deleting a rule MUST re-derive the visible rows from the events already
  retained, never by discarding retained events — narrowing and then widening the rules brings
  the same events back.

#### Refusing a Clashing Rule

- **FR-027**: A rule MUST be refused when it clashes with a rule already in the list. A rule
  being added clashes with an existing rule when either holds, comparing normalised prefixes:
  1. **The same prefix is already in the list**, under either kind. Adding `Show only 90` when
     `Show only 90` is present is a repeat; adding it when `Hide 90` is present is a direct
     contradiction. Both are refused.
  2. **The prefixes overlap and the kinds differ** — one prefix is a prefix of the other, and
     one rule is `Show only` while the other is `Hide`. `Show only 90` clashes with an existing
     `Hide 9` (the existing rule swallows the new one) and with an existing `Hide 9012` (the
     new one swallows the existing one).
- **FR-028**: Two rules of the **same** kind whose prefixes overlap MUST be permitted.
  `Show only 9` alongside `Show only 90` is redundant but says nothing contradictory, and
  refusing it would block a user from narrowing or broadening a set of same-kind rules.
- **FR-029**: The clash rule MUST be symmetric: if adding B to a list holding A is a clash, then
  adding A to a list holding B is the same clash.
- **FR-030**: When a rule is refused, the monitor MUST show an error that names the entered
  prefix, identifies the existing rule it clashes with, and states why the two cannot both be
  in the list.
- **FR-031**: A refused addition MUST leave the rule list, the filter in force, and the visible
  rows exactly as they were. There MUST be no partially added rule.
- **FR-032**: The error MUST be dismissible and MUST NOT block the user from correcting the
  entry, deleting the conflicting rule, or using any other part of the window.
- **FR-033**: Clash detection MUST compare normalised prefixes, so the same prefix entered in
  different case or spacing is detected as a clash.
- **FR-034**: Errors from this feature MUST be reported the same way existing failures are, so
  the user has one place to read what went wrong rather than several.

#### The Reference Surface Stays As It Is

- **FR-035**: The controls the reference screenshots depict — the `Sources` and `Filter`
  disclosure sections and their contents, `Remember up to`, `events`, `Clear`, `All Channels`,
  `One Channel`, and the `Time` / `Source` / `Message` / `Chan` / `Data` columns — MUST keep
  their labels, order, control types, grouping, and default states. This feature adds surface;
  it relabels, reorders, and restyles nothing.
- **FR-036**: Both new capabilities MUST be reachable without collapsing or expanding anything
  that was not already collapsible, and without adding a second place to do something the
  window already does.

### Key Entities

- **Capture State**: Whether the monitor is running or paused. While paused, nothing new is
  retained. Session-only; always running at launch. Independent of whether any source is
  selected and of whether any event is retained.
- **Data Prefix Rule**: One normalised hexadecimal prefix together with its kind — show only,
  or hide. The unit that is added, listed, and deleted.
- **Data Prefix Rule List**: The ordered set of rules currently in force. Empty by default,
  admitting everything. Persisted with the rest of the filter settings.
- **Rule Clash**: The relationship between a rule being added and a rule already in the list
  that makes them impossible to hold together — a repeated prefix, or an overlapping prefix of
  the opposite kind. Carries which existing rule is implicated and why, because that is what the
  error must say.

## Out of Scope

- Editing a rule in place. Rules are added and deleted; changing one means deleting it and
  adding the replacement.
- Reordering rules, grouping them, or naming them.
- Enabling and disabling a rule without deleting it.
- Matching on anything other than a leading hexadecimal prefix of the raw bytes — no wildcards,
  ranges, regular expressions, or byte-position matching.
- Filtering by any field other than raw data through this list. Message kind, channel, and
  source keep their existing controls.
- Exporting, saving, or copying the frozen list while paused.
- Automatically pausing on a trigger condition, or pausing after a set duration.
- Buffering traffic that arrives while paused, or catching up on it after resuming. Pausing
  stops retention; the gap is intended.
- Any change to what the reference screenshots depict.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With a source producing continuous traffic, a user can freeze the list and read a
  specific row without any previously received event disappearing — verified by comparing the
  visible rows immediately before and after pausing.
- **SC-002**: Pausing and resuming loses zero already-received events: the retained count on
  resuming equals the count at the moment of pausing, for a pause of any length under traffic
  of any density.
- **SC-003**: A user can drop one prefix from a set of several without retyping any of the
  others, in a single interaction.
- **SC-004**: Deleting one rule from a list of several leaves every other rule in force, with
  the visible rows updating on the same interaction rather than after any further action.
- **SC-005**: Every refused rule — clashing, malformed, or empty — leaves the visible rows
  identical to what they were before the attempt, in 100% of attempts.
- **SC-006**: For every refused clashing rule, the message shown identifies both the entered
  prefix and the existing rule it clashes with, so the user can act on it without inspecting the
  list themselves.
- **SC-007**: Rules and their kinds survive quit and relaunch unchanged, in both order and
  content.
- **SC-008**: A side-by-side comparison of the window against `screenshots/data.png` and
  `screenshots/filters.png` shows no change to any control they depict.
- **SC-009**: No rule list reachable through the interface can hold two rules that disagree
  about the same event — every such combination is refused at the point of adding it.

## Assumptions

- **"Include only start with" and "exclude start with" name the two rule kinds already in the
  window.** The existing `Show only` and `Hide` wording is reused rather than replaced, so the
  vocabulary stays consistent with the row this feature grows out of.
- **The `Data starts with` row is not part of the reference screenshots.** `screenshots/filters.png`
  shows no prefix filter; that row was added by feature 001 as new surface. Replacing its single
  field and radio pair with a rule list therefore changes added surface, not the design
  authority, and is permitted by constitution principle VII.
- **Pause stops retention, not the connection.** Freezing the display alone would still let the
  retention cap evict the frozen rows on a dense stream, which is exactly what the user asked
  not to happen. So paused means nothing new is retained — while ports stay open, sources stay
  selected, and the cap keeps its meaning. The cost is that traffic passing during a pause is
  not recorded, which is accepted.
- **Filtering remains a view over retained events, not an admission gate.** Adding or deleting a
  rule re-derives what is shown from what is retained; it never throws retained events away.
  This is why rules can be changed while paused without losing data.
- **The rule list is persisted with the other filter settings**, using the mechanism that
  already persists the prefix filter. Retained events remain session-only.
- **Errors are reported through the window's existing single error region.** The application
  already has one place to explain a failure, and a clash is one more failure to explain there.
- **A practical number of rules is small** — a handful, not hundreds. No paging, searching, or
  virtualisation of the rule list is assumed to be needed.
- **Contradictions are prevented, not arbitrated.** Rather than defining which kind wins when
  two rules disagree about an event, the clash rule makes that pair unaddable. The consequence,
  recorded as FR-024, is that `Hide` rules cannot affect anything while a `Show only` rule is in
  the list — a `Show only` rule already excludes everything a non-overlapping `Hide` rule could
  have caught. A list is therefore useful as a set of `Show only` rules or as a set of `Hide`
  rules; mixing them is permitted but the `Hide` half is inert.
- **A migrated single-mode filter cannot clash with itself.** The prefixes saved by an earlier
  version all share one mode, and same-kind overlaps are permitted, so an existing saved filter
  always converts into a valid rule list.
- Per the project constitution, acceptance scenarios in this specification are **manual
  verification steps performed against the running application**. No automated test files, test
  dependencies, or test harnesses are to be produced for them.
