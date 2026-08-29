# Quickstart: Verifying Pause and the Data Prefix Rules

**Feature**: `004-pause-and-filter-rules` | **Date**: 2026-08-29

This project has no test suite by constitutional design. Verification is compiling and running the
application and exercising the change by hand. This guide is the manual pass that closes the feature.

Nothing here is platform-specific — the feature adds no platform conditional — so a full pass on one
platform plus a launch-and-smoke on the other is sufficient. Run the gates on both.

---

## Prerequisites

- Rust (stable) and Node 20 or newer. On macOS, Xcode Command Line Tools; on Windows, the MSVC
  toolchain, Microsoft C++ Build Tools, and the WebView2 Runtime.
- **A source producing continuous traffic.** Most of the pause scenarios need a stream fast enough
  that the list moves on its own — a keyboard sending Clock, a controller with a knob to sweep, or a
  loopback port driven by any program that sends MIDI. A device that only sends when you press a key
  will not exercise SC-002 at all.
- **A settings file written by an earlier build**, for the migration check. If you have been running
  this application before, you already have one; if not, check out the previous commit, run it, type
  `90 B007` into `Data starts with`, choose `Hide`, quit, and come back.

## Gates — all four must pass before the manual pass counts

```bash
cargo clippy --workspace --all-targets   # zero warnings
cargo fmt --check                        # clean
npm run typecheck                        # tsc --noEmit, strict, zero errors
npm run tauri dev                        # the window opens
```

`tsc` is doing real work here. Removing `set_data_prefix_filter` and changing `FilterViewDto` makes
every stale webview call site a compile error, and `describeError`'s exhaustive switch fails until the
three new error variants have sentences. If the webview compiles without you having touched
`ipc.ts`, the bindings were not regenerated — check that the debug build wrote `src/bindings.ts`.

---

## 1 — Pause keeps what has already arrived (User Story 1, SC-001, SC-002)

1. Select a source that streams continuously. Let the list fill past a few hundred rows.
2. Note the **first** visible row's timestamp and the retained count.
3. Press `Pause`.
   - **Expect**: rows stop moving immediately. The control now reads `Resume` and the row states that
     arriving events are not being recorded.
4. Scroll up and down; hide a column from the column menu; scroll again.
   - **Expect**: every row is still there. Nothing is discarded (FR-003, scenario 2).
5. **Leave it paused, with the source still sending, for longer than it would take to fill the
   retention cap** — with `Remember up to` set to 1000 and a busy controller, a minute is plenty. Set
   it to 50 first if you want this to take seconds.
   - **Expect**: the first visible row's timestamp is *unchanged*, and the retained count is
     unchanged. This is the requirement (FR-010) — a frozen row must not be evicted by traffic that
     arrives while paused.
6. Press `Resume`.
   - **Expect**: the list is live again, the rows from before the pause are still listed, and new
     rows append after them. The gap in timestamps across the pause is expected and correct
     (FR-011) — traffic during a pause is not recorded.

## 2 — Everything else still works while paused (FR-006, FR-007)

With the monitor paused and rows on screen:

1. Untick `Clock` in the Filter panel, then tick it again.
   - **Expect**: the clock rows disappear and come back, from the retained events, without resuming.
2. Switch `All Channels` to `One Channel`, then back.
   - **Expect**: the list narrows and widens. Still paused.
3. Add and delete a prefix rule (see section 3).
   - **Expect**: the visible rows change; the monitor stays paused.
4. Press `Clear`.
   - **Expect**: the list empties, and the monitor is **still paused** — `Clear` does not resume.
5. Deselect every source while paused.
   - **Expect**: the empty list says no sources are selected, not merely that it is paused. Reselect,
     and confirm nothing starts arriving until you resume.

## 3 — Add and delete several rules (User Story 2, SC-003, SC-004)

1. Resume, and let some traffic in. Expand `Filter`.
2. Type `9` in `Data starts with`, choose `Show only`, press `Add`.
   - **Expect**: the rule is listed as a `Show only` rule for `9`, and the table shows only events
     whose raw bytes start with `9` — Note On and Note Off, not Clock.
3. Delete it. Type `FE`, choose `Hide`, `Add`. Then `F8`, `Hide`, `Add`.
   - **Expect**: both are listed; Active Sense and Clock are hidden; everything else is shown.
4. Delete the `FE` rule using its delete control.
   - **Expect**: it is gone from the list, `F8` is still listed and still in force, and the table
     updates on that one interaction — no second click, no retyping (SC-004).
5. Delete the last rule.
   - **Expect**: every retained event that passes the other filters is shown again.
6. Type `zz`, `Add`.
   - **Expect**: refused with an explanation; the list and the table are unchanged.
7. Clear the field and press `Add` with it empty.
   - **Expect**: refused with an explanation; no blank rule appears.
8. Type `9 a` (with a space, lowercase) and add it as `Hide`.
   - **Expect**: it is listed as `9A`, normalised (FR-020).

## 4 — Clashes are refused, and say why (User Story 3, SC-005, SC-006, SC-009)

Start from an empty rule list each time.

| Add this | Then try this | Expect |
|---|---|---|
| `Show only 90` | `Show only 90` | Refused — the same prefix is already listed |
| `Show only 90` | `Hide 90` | Refused — same prefix, opposite kind |
| `Show only 90` | `Hide 9` | Refused — overlapping prefixes, opposite kinds |
| `Show only 90` | `Hide 9012` | Refused — overlapping prefixes, opposite kinds |
| `Show only 90` | `hide 9 0` | Refused — normalisation makes this the same as `Hide 90` (FR-033) |
| `Show only 9` | `Show only 90` | **Accepted** — same kind, redundant but not contradictory (FR-028) |
| `Show only 90` | `Hide A1` | **Accepted** — no overlap |

For every refusal, check all four:

- the message appears **inside the Filter panel, directly under the entry row** — not in the
  window's shared banner, which is reserved for failures the user cannot attribute to a control they
  just used;
- it names the prefix you typed **and** the existing rule it clashes with (SC-006);
- the entry keeps the text you typed, and the field's border turns red so the refusal is visible
  even before the sentence is read;
- the rule list is exactly as it was, and the table has not changed by a single row (SC-005), and the
  rest of the window still works (FR-032).

Then check the message clears on its own terms: type one more character into the field, and it
disappears — it described the previous entry. Finally, with a clash showing, delete the existing rule
and add the refused one again. **Expect**: it is accepted, and the message clears.

**The inert-`Hide` consequence (FR-024)** is expected behaviour, not a bug: with `Show only 9` and
`Hide A1` both listed, deleting `Hide A1` changes nothing on screen, because the `Show only` rule was
already excluding everything the `Hide` rule could have caught. Confirm it behaves that way rather
than reporting it.

## 5 — Rules survive a restart, and an old settings file migrates (SC-007, FR-025)

1. Build a list of three rules — say `Hide F8`, `Hide FE`, `Hide B0`. Quit the application.
2. Relaunch.
   - **Expect**: the same three rules, in the same order, with the same kinds.
3. Quit. Restore the settings file written by the earlier build (the one from Prerequisites) and
   relaunch.
   - **Expect**: `90` and `B007` are listed as two `Hide` rules — the mode they were saved with — and
     the source selections, hidden columns, and retention limit from that file are all still in
     effect. If the window comes up with every source selected and `1000` in the retention field, the
     migration did not run and the document fell back to defaults.

## 6 — Nothing the screenshots depict has changed (SC-008, FR-035)

Compare the running window side by side with `screenshots/data.png` and `screenshots/filters.png`:

- `Remember up to`, `events`, and `Clear` — same labels, same order, same row, same size and style.
  `Clear` still sits at the end of the row. **`Pause` now sits immediately before it**, by the
  project owner's decision — a recorded deviation from principle VII, documented in
  `RetentionRow.tsx`. Check that adding it displaced nothing: `Clear` has not moved left, changed
  size, or changed style.
- The three filter columns, their headings, their entries and indentation, `System Exclusive`,
  `Invalid`, `All Channels` / `One Channel` — unchanged, still all ticked and `All Channels` selected
  on a fresh profile.
- `Time`, `Source`, `Message`, `Chan`, `Data` — same five columns in the same order.
- Both disclosure sections still start collapsed.

The visible differences are the `Pause` button in the retention row and the rewritten
`Data starts with` row.

Then check the interactive states the still image cannot show: `Pause`, `Clear`, and `Add` all tint
on hover, darken while held, and take a visible focus ring when tabbed to; `Pause` stays pressed-in
while the monitor is paused; a listed rule's row tints on hover and its delete cross comes up to full
strength.

## 7 — The other platform

Build and launch on the platform you did not use above, and repeat sections 1, 3, and 4 briefly.
Nothing in this feature is platform-specific; this pass exists to confirm that remains true, and that
the `Data` column's fidelity note on Windows is unaffected.
