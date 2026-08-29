# Implementation Plan: Pause the View and Build Several Data Prefix Rules

**Branch**: `004-pause-and-filter-rules` | **Date**: 2026-08-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/004-pause-and-filter-rules/spec.md`

## Summary

Two capabilities, both landing as **rules in `midi-core`** with the Tauri shell and the webview doing
nothing but carrying and rendering them.

**Pause** becomes a `CaptureState` held by `Monitor`. `Monitor::ingest` already decides whether an
arriving event is retained and whether it is visible; pausing adds one gate at the top of that method
so a paused monitor retains nothing. That is what makes the spec's central promise cheap: with
nothing entering the log, the retention cap cannot evict a frozen row (FR-010), and every filter
keeps working while paused because filtering was always a *view* over the retained log, never an
admission gate (FR-006).

**Data prefix rules** replace `DataPrefixFilter { mode, prefixes }` with
`DataPrefixFilter { rules: Vec<DataPrefixRule> }`, where a rule is one `HexPrefix` plus one
`PrefixMode`. `PrefixMode` already exists and already means `Show only` / `Hide`; it is reused
rather than replaced. Clash detection lives on `DataPrefixFilter::add`, which returns
`Result<(), CoreError>` and mutates nothing when it refuses — the same shape
`HexPrefix::parse` already uses, and the reason a rejected rule cannot blank the list.

Three facts shape the rest of the design:

1. **A rule's prefix is its identity.** Because the clash rule refuses a repeated prefix under
   *either* kind (FR-027), no two rules can share a prefix — so deletion keys on the normalised
   prefix string and needs no index, no id, and no fragile positional contract.
2. **The clash rule is what removes the need for precedence.** Two prefixes can match the same bytes
   only when one is a prefix of the other; refusing that pair across kinds means no event can ever
   match both a `Show only` and a `Hide` rule, so `admits` never has to arbitrate (FR-023).
3. **The settings file changes shape, so it needs a real migration.** `StoreSettingsRepository::load`
   treats an unreadable document as absent, which would silently discard source selections, columns,
   and retention along with the filter. `#[serde(from = ...)]` over an untagged enum reads both the
   old and the new shape, so FR-025 is satisfied without a fallback that loses unrelated settings.

No new dependency. No new pattern.

**One deviation from the original design, decided by the project owner after implementation**:
`Pause` sits inside the retention row beside `Clear` rather than in a row of its own, and refused
rules are explained inside the Filter panel rather than in the window's shared banner. Both are
recorded where they apply — in `RetentionRow.tsx`'s module doc, and in the Constitution Check below.

## Technical Context

**Language/Version**: Rust (workspace edition 2021); TypeScript 5 strict in the webview

**Primary Dependencies**: existing only — `tauri` 2, `tauri-specta`/`specta`, `tauri-plugin-store`,
`serde`, `thiserror`, `zustand`, `@tanstack/react-virtual`. **Nothing new** (see [research.md](./research.md) D6)

**Storage**: `tauri-plugin-store` JSON through the existing `SettingsRepository` port. Schema effect:
`PersistedSettings.filter.data_filter` changes shape; read-compatible with documents written by
003 and earlier — see [contracts/persisted-settings.md](./contracts/persisted-settings.md)

**Testing**: None. The constitution's Verification Standard prohibits a test suite, and the Spec Kit
templates' test phases are skipped under Governance. Verification is `cargo clippy` clean,
`tsc --noEmit` clean, and the manual pass in [quickstart.md](./quickstart.md)

**Target Platform**: macOS and Windows 10 1809+ / Windows 11 — both, identically. This feature has no
platform-specific behaviour and adds no platform conditional

**Project Type**: Tauri desktop application — Rust workspace (domain/application core, per-platform
adapters, Tauri shell) plus a React webview

**Performance Goals**: pausing takes effect within one batch interval (16 ms) and, once paused, the
ingest path does strictly less work than before; adding or deleting a rule re-renders the list on the
same interaction (SC-004). Clash detection is O(rules) over a handful of short strings

**Constraints**: a refused rule mutates nothing (FR-031); a paused monitor retains nothing; no
depicted control is relabelled, reordered, or restyled (FR-035); zero `unwrap`/`expect`/`panic`
outside `main`; zero clippy warnings; no `any` in the webview

**Scale/Scope**: 5 files changed in `midi-core` (1 new), 4 in `src-tauri`, 6 in the webview (2 new, 1
deleted), plus README. Three IPC commands added, one removed. Three `CoreError` variants added

## Constitution Check

*GATE: evaluated before Phase 0, re-evaluated after Phase 1 design. Result: **PASS**, with two
entries in Complexity Tracking.*

| Principle | Assessment | Verdict |
|---|---|---|
| **I. Code quality** | `CaptureState` and `DataPrefixRule` are purpose-built types, not booleans and tuples. Three new typed `CoreError` variants, each carrying what the message must quote. The clash rule is one named predicate (`overlaps`), not an inline condition repeated at two call sites. `HexPrefixFilter.tsx` is **deleted** rather than left beside its replacement — no dead code. No new literal escapes into a use site. | PASS |
| **II. Reuse libraries** | Nothing here is a solved problem worth a crate: prefix overlap is `str::starts_with` in both directions over a handful of entries, and a trie or radix-set dependency would be more code than it removes (research D6). Serde's `from` container attribute does the schema migration rather than a hand-written `Deserialize` impl — reuse in the direction the principle asks for. | PASS |
| **III. Documentation explains why** | Every new `pub` item and both new modules carry a *why*. The five that earn their keep are pre-identified in [data-model.md](./data-model.md): why pause stops retention rather than display; why a rule's prefix is its identity; why clashes are prevented rather than arbitrated; why `Hide` is inert beside `Show only`; why the legacy settings shape is still readable. | PASS |
| **IV. Layered architecture + typed IPC** | Every rule lands in `midi-core`. The three new command handlers validate, delegate, persist, and map errors — no branching on rule kinds, no decision about what is visible. New DTOs are declared in Rust and regenerate `src/bindings.ts`; no hand-written parallel types, no `any`, no `serde_json::Value` in a signature. Errors cross as new variants, never strings. | PASS |
| **V. Named patterns, never ceremony** | No pattern introduced. `Command` for the three new handlers is the one already in use; `Newtype` (`HexPrefix`) is reused as-is. A `RuleValidator` service, a rule-id type, and a builder for `DataPrefixFilter` were all considered and rejected as abstractions answering no present pressure (research D3, D4). | PASS |
| **VI. Screenshots are the design authority** | Nothing the screenshots depict changes: `Sources`, the `Filter` columns and their labels, `All Channels` / `One Channel`, `Remember up to … events`, `Clear`, and the five table columns keep their labels, control types, order, and defaults. The one surface being *rewritten* — the `Data starts with` row — appears in no screenshot; it was added by feature 001 (research D5). | PASS |
| **VII. Extend rather than edit** | Pause is added state, gated in one place. `PrefixMode` is reused, so both rule kinds already exist as variants and both are matched exhaustively with no catch-all. **One departure**: `Pause` was placed inside the retention row, which the screenshots depict, rather than earning a row of its own — an explicit decision by the project's owner, recorded in `RetentionRow.tsx` with its reason. It is bounded: `Clear` keeps its label, size, style, and position at the end of the row, and nothing else depicted is touched. See Complexity Tracking. | PASS, with a recorded departure |
| **Verification Standard** | No tests added; no test files, dependencies, or CI test steps. The templates' test phases are skipped, recorded here per Governance. Manual verification is specified in [quickstart.md](./quickstart.md). | PASS (skip recorded) |

**Re-evaluation after Phase 1**: unchanged. The design added no type, port, or indirection beyond the
two tracked items, and both are forced by a requirement in the spec rather than by anticipation.

## Project Structure

### Documentation (this feature)

```text
specs/004-pause-and-filter-rules/
├── plan.md              # This file
├── research.md          # Phase 0 — D1..D8, each with the alternatives rejected
├── data-model.md        # Phase 1 — the new and changed types, and their reach
├── quickstart.md        # Phase 1 — build, run, and verify by hand
├── contracts/
│   ├── ipc-commands.md         # The command surface: three added, one removed
│   └── persisted-settings.md   # The on-disk shape change and its migration
├── checklists/
│   └── requirements.md
└── tasks.md             # Phase 2 — created by /speckit-tasks, not here
```

### Source Code (repository root)

```text
crates/midi-core/src/
├── application/
│   ├── capture.rs       # NEW — CaptureState { Running, Paused }
│   ├── mod.rs           # + pub mod capture;
│   ├── monitor.rs       # + capture state, ingest gate, rule add/remove
│   └── error.rs         # + DuplicatePrefixRule, ContradictoryPrefixRule, UnknownPrefixRule
└── domain/
    └── filter.rs        # DataPrefixRule; DataPrefixFilter rewritten; clash rule; migration

src-tauri/src/
├── dto.rs               # + CaptureStateDto, DataPrefixRuleDto; FilterViewDto and SnapshotDto changed
├── commands.rs          # + set_capture_state, add_data_prefix_rule, remove_data_prefix_rule
│                        # - set_data_prefix_filter
├── error.rs             # + three IpcError variants and their From arms
└── lib.rs               # command registration only

src/
├── App.tsx              # error banner restyled; rule refusals no longer routed here
├── bindings.ts          # REGENERATED — never edited by hand
├── ipc.ts               # + three call wrappers, three describeError arms
├── store.ts             # + captureState from the snapshot
└── components/
    ├── RetentionRow.tsx    # + the Pause/Resume button, before Clear
    ├── DataPrefixRules.tsx # NEW — entry field, kind choice, Add, and the rule list
    ├── HexPrefixFilter.tsx # DELETED — replaced by DataPrefixRules
    ├── FilterPanel.tsx     # renders DataPrefixRules in place of HexPrefixFilter
    └── EventTable.tsx      # empty-list explanation gains the paused case

README.md                # what the app does — pause and the rule list
```

**Structure Decision**: no structural change. Both capabilities fit the existing four-layer shape
(domain → application → IPC surface → webview) and neither needs a new crate, module tree, or port.
The only new core module is `application/capture.rs`, holding one enum.

## Key Design Decisions

### 1. Pause lives in `Monitor::ingest`, and stops retention rather than display

```rust
pub fn ingest(&mut self, event: MidiEvent) -> Option<MidiEvent> {
    match self.capture {
        CaptureState::Paused => return None,   // NEW: nothing retained, nothing streamed
        CaptureState::Running => {}
    }
    if !self.catalogue.is_selected(event.source) { return None; }
    let streamed = self.filter.admits(&event).then(|| event.clone());
    self.log.push(event);
    streamed
}
```

Three consequences fall out of that one gate, and each is a requirement satisfied for free:

- **FR-010** — the cap cannot evict a frozen row, because `EventLog::push` is never reached.
- **FR-004** — no event is streamed, because the only producer of stream traffic is this method's
  return value.
- **FR-006** — filters keep working, because `visible_events()` reads the log and does not care
  whether ingest is running.

The pause command also calls `state.pump.discard_pending()`, the existing mechanism for
"events admitted under previous settings must not arrive after the change". Without it, a batch
already queued in the pump could land up to 16 ms after the user pauses.

**No gate is needed in the webview.** A queued event that was retained *before* the pause is present
in the snapshot the pause command returns, so the store's existing high-water mark discards it if it
also arrives on the stream. An event arriving *after* the pause was never ingested and never queued.
Adding a webview-side check would put a rule on the wrong side of the boundary to solve a problem
that does not exist.

**Rejected:** freezing only the display (webview stops appending). It fails the spec's central
promise — retention keeps running, so on a dense stream the cap evicts precisely the rows the user
paused to read. **Rejected:** stopping the adapter or closing ports. That would drop the connection,
lose device state, and make resume slow and visible; FR-008 forbids it.

### 2. A rule's prefix is its identity, so deletion needs no id

The clash rule refuses any addition whose normalised prefix already appears in the list, under either
kind. That makes the prefix a unique key, which decides three things at once: `remove_data_prefix_rule`
takes a `String`, the webview's delete control sends back the string it was rendered, and a stale
webview asking to delete something already gone gets `CoreError::UnknownPrefixRule` rather than
silently deleting the wrong row.

**Rejected:** a `RuleId` newtype, and positional indices. An id solves duplicate-identity problems
this list cannot have, and an index makes the contract order-dependent for no gain — both are the
abstraction-without-pressure that Principle V rejects.

### 3. `admits` never arbitrates, because the clash rule guarantees it never has to

```rust
pub fn admits(&self, event: &MidiEvent) -> bool {
    if self.rules.is_empty() { return true; }         // no rules means no opinion
    let hex = event.raw_hex();
    // Two prefixes can match the same bytes only if one is a prefix of the other,
    // and `add` refuses that pair across kinds — so these two tests can never disagree.
    if self.rules.iter().any(|rule| rule.kind == PrefixMode::Exclude && rule.matches(&hex)) {
        return false;
    }
    let mut includes = self.rules.iter().filter(|rule| rule.kind == PrefixMode::Include).peekable();
    includes.peek().is_none() || includes.any(|rule| rule.matches(&hex))
}
```

The invariant is stated in the doc comment because it is load-bearing and not visible in the code: it
is enforced in `add`, several lines away. FR-024's consequence — `Hide` rules are inert while any
`Show only` rule is in the list — is documented as intended behaviour at the same place, so a future
reader does not "fix" it by inventing a precedence rule the spec deliberately does not have.

### 4. Clash detection returns two distinct errors, not one with a reason field

```rust
pub fn add(&mut self, rule: DataPrefixRule) -> Result<(), CoreError>
```

- `CoreError::DuplicatePrefixRule { prefix, existing_kind }` — the same prefix is already listed.
  The remedy is to delete it or enter a different prefix.
- `CoreError::ContradictoryPrefixRule { prefix, kind, existing_prefix, existing_kind }` — an
  overlapping prefix of the opposite kind. The remedy is to narrow one of the two.

Two variants rather than one because the messages differ and the remedies differ — the same reasoning
that already keeps `MalformedHexPrefix` and `EmptyHexPrefix` apart. Splitting them here also keeps
the *deciding* in Rust: the webview renders each variant's sentence and never inspects the data to
work out which explanation applies.

`add` validates fully before it mutates, so a refusal leaves the list byte-identical (FR-031).

### 5. The settings file reads both shapes

`DataPrefixFilter` carries `#[serde(from = "StoredDataPrefixFilter")]`, where the stored type is an
untagged enum with the current shape first and the pre-004 shape second. A legacy document's
prefixes each become a rule of the mode they were saved with (FR-025), deduplicated by prefix so the
migration cannot produce a list the new invariant forbids — the old field accepted `90 90`, the new
list cannot hold it twice. Serialization is unaffected: the new shape is what gets written.

**Rejected:** relying on `load`'s existing "unreadable means absent" fallback. It is the right rule
for a document this build genuinely cannot understand, but here it would throw away source
selections, column visibility, and the retention limit to avoid migrating one field.

### 6. Where the two new surfaces go

The `Data starts with` row appears in **no screenshot** — it was added by feature 001 as new surface —
so rewriting it into a rule list changes added surface, not the design authority (research D5). The
row keeps its `Data starts with` label, its entry field, and its `Show only` / `Hide` wording; it
gains an `Add` action and a list of rules beneath, each with a delete control.

> **Revised after implementation, by the project owner's decision.** `Pause` now sits **inside the
> retention row, immediately before `Clear`**, and carries no sentence beside it — the reasoning it
> used to state is on its tooltip. The deviation from principle VII is recorded in
> `RetentionRow.tsx`'s module doc, and it is bounded: `Clear` keeps its label, size, style, and
> position at the end of the row. Rule refusals were also moved out of the window's shared banner and
> into the Filter panel, beside the field that produced them. The original reasoning is kept below
> because the trade-off it describes is still the one that was made — it was simply decided the other
> way.

The capture control gets **a new row of its own**, between the retention row and the event table —
the position the error banner already occupies when it is present. While paused it states that
arriving events are not being recorded, so the gap is something the user can anticipate rather than
discover afterwards.

**Rejected:** putting a `Pause` button into the retention row beside `Clear`. That row is depicted in
`screenshots/data.png`, and constitution VII is explicit that new capability earns new surface rather
than being added into a depicted one.

`EventTable`'s empty-list explanation gains a fourth case: paused with nothing to show must not read
`Waiting for events…`, which would be untrue — nothing is coming until the user resumes.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| A new `application/capture.rs` module holding a single two-variant enum | `CaptureState` is a use-case state, not a MIDI concept, so the domain is the wrong home; `monitor.rs` is already the crate's largest module and this type is referenced by the DTO layer independently | A `bool paused` field reads as `set_paused(true)` at every call site and crosses the wire as an untyped flag. The house already prefers a named two-variant enum for exactly this (`PrefixMode`), and the enum matches exhaustively in the webview |
| `domain/filter.rs` returns `application::error::CoreError` from `add` | Follows the existing arrangement — `domain/ids.rs` and `domain/column.rs` already return `CoreError` from validating constructors | A second, domain-local error type would double the error vocabulary and force a translation layer, for one method. Consolidating the two error types is a real cleanup but belongs to its own change, not to this feature |
| `Pause` placed inside the retention row, which `screenshots/data.png` depicts (principle VII) | An explicit decision by the project's owner: the control belongs beside `Clear`, the other control that acts on the retained list, and a row of its own put a permanent divider between the controls and the table | A row of its own was what the plan originally specified and what was first built. It satisfies principle VII exactly, and it was judged to read worse. The departure is bounded to adding one sibling: `Clear` is not relabelled, resized, restyled, or moved from the end of the row |

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| A user pauses on a dense stream and later believes the gap was a dropped-event bug | The button reads `Resume` and stays visibly pressed while paused, and its tooltip states that arriving events are not recorded (FR-002); `EventTable`'s empty case says it in words. **Weaker than the original design**, which stated it in a permanent line of text — accepted deliberately to keep the row as quiet as the reference image |
| The untagged migration silently matches the wrong arm on a hand-edited settings file | The two shapes share no field names, so neither can satisfy the other's required fields; a document matching neither still falls back to defaults exactly as it does today |
| Regenerated bindings leave a stale call site in the webview | That is the intended mechanism: `tsc --noEmit` under `strict` fails on the removed `setDataPrefixFilter` and on the changed `FilterViewDto`, and `describeError`'s exhaustive switch fails until the three new error variants are handled |
| The rule list grows long enough to push the table off screen | Out of scope by assumption (a handful of rules), but the list scrolls within its own row rather than growing the panel unbounded |

## Phase 2 Note

`/speckit-tasks` generates `tasks.md` from this plan. Per the constitution's Governance clause, the
task template's test categories are inert here and MUST be skipped; the verification tasks are the
`quickstart.md` steps, performed by hand against the running application on both platforms.
