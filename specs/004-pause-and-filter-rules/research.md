# Phase 0 Research: Pause the View and Build Several Data Prefix Rules

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Date**: 2026-08-29

This feature adds no dependency and touches no platform API, so the research below is about the
existing codebase rather than about the outside world: where each rule belongs, what the current code
already gives us for free, and which of the obvious-looking abstractions are ceremony. Each decision
records what was rejected, because the rejections are the part that will otherwise be re-litigated.

The one external question — how to read two on-disk shapes with one type — is answered against
serde's own documentation (D7).

---

## D1 — Pause stops retention, not display

**Decision**: `Monitor` holds a `CaptureState`, and `Monitor::ingest` returns `None` without pushing
to the log while it is `Paused`.

**Rationale**: The spec's requirement is not "the rows stop moving" but "the rows stop moving *and
what I already received survives*" (FR-003, FR-010). Those come apart under load. `EventLog::push`
evicts oldest-first at the retention cap, so a monitor that keeps retaining while the display is
frozen will, on a stream fast enough to be worth pausing, evict exactly the rows the user paused to
read — the failure would be invisible in testing with a slow source and certain with a fast one.
Gating ingest makes the guarantee structural rather than probabilistic.

`ingest` is also the only place this can go. It is the single point where an arriving event meets the
user's settings, and it already answers two questions of the same kind ("is this source selected",
"does this pass the filter"). A gate anywhere else would mean a second place that decides whether an
event is recorded.

**Alternatives considered**:

- *Freeze the display in the webview only.* Rejected on the eviction argument above. It also puts a
  rule ("do not show what has arrived") in the layer forbidden to hold rules.
- *Stop the pump instead of ingest.* Rejected: it stops delivery but not retention, so the cap keeps
  evicting; and on resume the webview and the log would disagree about what was seen.
- *Stop the adapter / close ports while paused.* Rejected: FR-008 forbids disturbing the connection,
  and reopening on resume is slow, visible, and can fail when another program has taken the device in
  the meantime.
- *Buffer arrivals while paused and reveal them on resume.* Rejected — this was clarification Q1 and
  the answer was to discard. Buffering reintroduces the eviction problem it was meant to solve, since
  the buffer must either be bounded (losing events anyway) or unbounded (a memory leak the user
  cannot see).

---

## D2 — `CaptureState` is a two-variant enum in the application layer

**Decision**: `enum CaptureState { Running, Paused }` in a new `crates/midi-core/src/application/capture.rs`,
held by `Monitor`, never persisted.

**Rationale**: Pausing is a use case's state, not a MIDI concept, so the domain layer is the wrong
home — the domain holds messages, events, filters, and sources. It sits beside `MidiSystemStatus`
conceptually but not physically: that one lives in `ports.rs` because a port reports it, and nothing
reports this one.

An enum rather than a `bool` follows the house's own precedent. `PrefixMode`, `CheckState`,
`ChannelMode`, `Availability`, and `ByteFidelity` are all small enums where a flag would have
compiled, and the reasons are the same each time: the call site reads as what it means
(`set_capture_state(CaptureState::Paused)`), and it crosses the IPC boundary as a tagged union the
webview matches exhaustively rather than as a bare `true`.

Not persisted, because FR-012 requires a freshly launched monitor to be running. Restoring a paused
monitor would open a window that looks broken and gives no clue why.

**Alternatives considered**:

- *`is_paused: bool` on `Monitor`.* Rejected on readability and on wire typing, above.
- *Put the state in `AppState` (the Tauri shell).* Rejected: it would put "should this event be
  recorded" in the layer that is required to hold no rules, and the shell's event callback would
  have to branch before calling `ingest`.
- *Model it as a third `MidiSystemStatus`.* Rejected outright: the MIDI system's reachability is a
  fact about the machine, pausing is a choice by the user, and conflating them would make "the user
  paused" indistinguishable from "MIDI is unavailable" in the interface.

---

## D3 — A rule's normalised prefix is its identity

**Decision**: `remove_data_prefix_rule(prefix: String)`. No rule id, no index.

**Rationale**: The clash rule refuses any addition whose prefix already appears under either kind
(FR-027), so the list cannot contain two rules with the same prefix. Uniqueness is therefore an
invariant, not a hope, and the prefix can serve as the key. The webview sends back the exact
normalised string the core gave it, which also means no normalisation logic on the webview side.

A stale delete — the webview asking to remove a rule that is already gone — returns
`CoreError::UnknownPrefixRule`, matching how `UnknownSource` and `UnknownGroup` already handle a
webview built against a stale view.

**Alternatives considered**:

- *A `RuleId` newtype.* Rejected as ceremony: ids exist to distinguish otherwise identical rows, and
  identical rows are unrepresentable here. Principle V is explicit that a pattern answering no
  present pressure must be removed.
- *Positional index.* Rejected: it makes the contract order-dependent, so any future reordering or
  concurrent mutation silently deletes the wrong rule.
- *Delete by prefix **and** kind.* Rejected as redundant — the prefix already determines the rule —
  and worse, it invites a caller to believe two rules can share a prefix.

---

## D4 — Contradictions are prevented at `add`, so `admits` needs no precedence rule

**Decision**: `DataPrefixFilter::add` refuses a clashing rule; `admits` applies
"no `Hide` matched, and if any `Show only` exists, one matched" with no tie-break.

**Rationale**: Two prefixes match the same byte string only when one is a prefix of the other. The
clash rule (FR-027 case 2) refuses exactly that pair when the kinds differ. Therefore no event can
match both a `Show only` and a `Hide` rule, and the two halves of `admits` cannot disagree — there is
nothing to arbitrate. This is why the user's answer to clarification Q2 ("don't allow a clash to
happen") is implementable as stated rather than as a precedence table.

The consequence recorded at FR-024 — while any `Show only` rule exists, `Hide` rules are inert —
follows from the same fact and is documented at `admits` and in the `Assumptions`, so it is
recognisable as intended rather than as a bug to be patched with a precedence rule later.

**Alternatives considered**:

- *`Hide` wins over `Show only` (deny-list beats allow-list).* Rejected: it is unreachable under the
  clash rule, so implementing it would be dead logic that reads as though the clash rule were weaker
  than it is.
- *Allow the carve-out pattern (`Show only 90` plus `Hide 9012`) and give `Hide` precedence.*
  Rejected because clarification Q3's answer refuses that pair. It is the more expressive design and
  it was put to the user explicitly; the decision recorded in the spec stands.
- *Evaluate rules in list order, first match wins.* Rejected: it makes behaviour depend on insertion
  order, which the interface does not show and the user cannot change.

---

## D5 — The `Data starts with` row may be rewritten; the screenshots' controls may not

**Decision**: replace `HexPrefixFilter.tsx` with `DataPrefixRules.tsx`; add a new row for the capture
control between the retention row and the table; touch no depicted control.

**Rationale**: `screenshots/filters.png` shows the three checkbox columns, `System Exclusive`,
`Invalid`, and the `All Channels` / `One Channel` pair — and **no prefix filter**. That row was
introduced by feature 001 as additive surface. Constitution VI freezes what the images depict;
constitution VII states that the space around the reference surface is open. Rewriting an additive
row is therefore permitted, and rewording `Show only` / `Hide` is avoided anyway to keep the
vocabulary stable for users.

`screenshots/data.png` *does* depict the retention row including `Clear`, so the capture control gets
its own row rather than joining that one. The chosen position — below the retention row, above the
table — is already used by the error banner when it is present, so it introduces no new relationship
between elements; the table shrinks by one row height exactly as it does when a disclosure section
expands.

**Alternatives considered**:

- *A `Pause` button beside `Clear`.* Rejected: it adds a control into a surface the screenshots
  depict, which VII names specifically.
- *A keyboard shortcut with no visible control.* Rejected: FR-002 requires the state to be visible,
  and an invisible toggle makes a frozen list indistinguishable from a stalled one.
- *A new disclosure section for pause.* Rejected: a one-control section that must be expanded before
  it can be used is worse than a row, and the disclosure pattern belongs to the two sections the
  screenshots show.

---

## D6 — No new dependency

**Decision**: implement prefix overlap and clash detection with `str::starts_with`.

**Rationale**: Principle II requires searching for a library before writing code, and it also
requires the dependency to be proportionate. The whole of the matching logic is
`a.starts_with(b) || b.starts_with(a)` over normalised uppercase nibble strings, evaluated against a
list the spec assumes holds a handful of entries. Prefix-set crates (radix tries, `trie-rs`,
`fst`-style structures) solve lookup at a scale that does not exist here and would add a dependency,
a build cost, and a second representation of the rule list to keep in step. `HexPrefix` already
guarantees normalisation, which is the only part that is genuinely error-prone, and that guarantee is
already reused rather than rewritten.

**Alternatives considered**: a trie/radix-set crate (rejected, above); a regex crate to express
prefixes (rejected — it widens the matching language beyond what the spec's Out of Scope allows, and
invites users to type patterns the clash rule cannot reason about).

---

## D7 — Reading both settings shapes with `#[serde(from = "...")]` over an untagged enum

**Decision**:

```rust
#[derive(Deserialize)]
#[serde(untagged)]
enum StoredDataPrefixFilter {
    Rules { rules: Vec<DataPrefixRule> },        // written by 004 and later
    Legacy { mode: PrefixMode, prefixes: Vec<HexPrefix> },  // written by 001..003
}

#[derive(Serialize, Deserialize)]
#[serde(from = "StoredDataPrefixFilter")]
pub struct DataPrefixFilter { pub rules: Vec<DataPrefixRule> }
```

**Rationale**: Serde's documentation states that `#[serde(from = "FromType")]` "deserializes a type
by first deserializing into `FromType` and then converting", requiring `From<FromType>` on the target
and `Deserialize` on the intermediate — and that an untagged enum "attempts to deserialize the data
against each variant in order". That is exactly the two-shape read this needs, with serialization
left alone so the new shape is what gets written.

The two shapes share no field names, so neither can satisfy the other's required fields and the
ordering of the variants cannot cause a mis-parse. The `From` conversion deduplicates by prefix while
converting a legacy list, because the old single-mode field accepted the same prefix twice and the
new invariant forbids it.

Without this, `StoreSettingsRepository::load`'s existing rule — an unreadable document is treated as
absent — would fire on every upgrade, discarding source selections, column visibility, and the
retention limit along with the filter. That rule is correct for a genuinely unreadable document and
wrong as a substitute for a migration.

**Alternatives considered**:

- *Hand-written `Deserialize` impl.* Rejected: more code, and it re-solves what the attribute already
  does (Principle II).
- *A version field on `PersistedSettings`.* Rejected for now: one field changed shape once, and a
  version number would have to be threaded through a document that has never needed one. It becomes
  worth adding when a change cannot be expressed structurally.
- *Let the fallback fire and accept the loss.* Rejected: FR-025 requires the saved filter to come
  back, and silently forgetting which devices a user had selected is the failure mode
  `PersistedSettings` was carefully designed to avoid.

---

## D8 — What the existing design already gives this feature for free

Recorded because it is the reason the plan is small, and because a future reader should not
re-derive it:

- **Filtering while paused** needs no work. `EventLog::view` reads the log with the current filter at
  render time, so every filter command already re-derives the visible rows from what is retained
  (FR-006, FR-022, FR-026).
- **`Clear` while paused** needs no work: `Monitor::clear` touches the log and nothing else (FR-007).
- **The clash error reaching the user** needs no new plumbing. `ipc.ts`'s `run()` already routes any
  `IpcError` through `describeError` into the store's single error banner, which is dismissible and
  blocks nothing (FR-030, FR-032, FR-034).
- **A refused rule leaving the list untouched** is the same guarantee `set_data_prefix_filter`
  already provides by parsing before mutating; `add` keeps it by validating before inserting
  (FR-031).
- **Stale stream batches after a pause** are already handled by the store's high-water mark plus the
  pump's `discard_pending`, both of which exist for the equivalent problem on a filter change.
