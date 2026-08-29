# Phase 1 Data Model: Pause the View and Build Several Data Prefix Rules

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Date**: 2026-08-29

Types are grouped by the layer that owns them. Signatures are shown as intent, not as final code —
the doc comments each one must carry are named at the end, because under this project's Verification
Standard the documentation is where the reasoning is checked.

---

## 1. Application layer — `crates/midi-core/src/application/capture.rs` (new)

### `CaptureState`

```rust
pub enum CaptureState {
    /// Arriving events are retained and streamed.
    Running,
    /// Arriving events are discarded. What was already retained is untouched.
    Paused,
}
```

| Property | Value |
|---|---|
| Default | `Running` — a freshly launched monitor is running (FR-012) |
| Persisted | **No.** Deliberately absent from `PersistedSettings` |
| Derives | `Debug, Clone, Copy, PartialEq, Eq` — no `Serialize`/`Deserialize`, because nothing stores it |
| Matched | Exhaustively, no catch-all arm, at `ingest` and at the DTO conversion |

**Why an enum and not a `bool`**: see [research.md](./research.md) D2. In short — the call site reads
as what it means, and it crosses the IPC boundary as a tagged union the webview matches exhaustively.

---

## 2. Domain layer — `crates/midi-core/src/domain/filter.rs` (changed)

### `PrefixMode` — unchanged, reused

Already exists with exactly the two kinds this feature needs: `Include` ("show only matches") and
`Exclude` ("hide matches"). Not renamed. The interface's `Show only` / `Hide` wording is unchanged, so
the user's vocabulary is stable and no depicted or added label is reworded.

### `DataPrefixRule` (new)

```rust
pub struct DataPrefixRule {
    /// The normalised nibble prefix this rule tests.
    pub prefix: HexPrefix,
    /// Whether matching events are the only ones shown, or the only ones hidden.
    pub kind: PrefixMode,
}

impl DataPrefixRule {
    /// Whether this rule's prefix matches an event's raw hex.
    pub fn matches(&self, raw_hex: &str) -> bool;
    /// Whether two rules' prefixes can match the same bytes — i.e. one is a prefix of the other.
    fn overlaps(&self, other: &Self) -> bool;
}
```

| Rule | Statement |
|---|---|
| Identity | The `prefix`. No two rules in a list may share one (enforced by `add`) |
| Normalisation | Inherited from `HexPrefix::parse` — whitespace stripped, uppercased (FR-020) |
| Matching | Nibble prefix test, so `9` matches `90`–`9F` (FR-019), unchanged from today |
| Derives | `Debug, Clone, PartialEq, Eq, Serialize, Deserialize` — persisted as part of the filter |

### `DataPrefixFilter` (rewritten)

```rust
#[serde(from = "StoredDataPrefixFilter")]
pub struct DataPrefixFilter {
    /// The rules in force, in the order they were added.
    pub rules: Vec<DataPrefixRule>,
}

impl DataPrefixFilter {
    /// Adds a rule, or refuses it because it clashes with one already listed.
    pub fn add(&mut self, rule: DataPrefixRule) -> Result<(), CoreError>;
    /// Removes the rule carrying this prefix.
    pub fn remove(&mut self, prefix: &HexPrefix) -> Result<(), CoreError>;
    /// Whether this event survives the rules.
    pub fn admits(&self, event: &MidiEvent) -> bool;
}
```

**Before → after**: `{ mode: PrefixMode, prefixes: Vec<HexPrefix> }` becomes `{ rules: Vec<DataPrefixRule> }`.
The old shape's single mode applied to every prefix; the new one carries a kind per rule.

#### `add` — the clash rule (FR-027 to FR-029)

Validates completely before mutating. On refusal the list is byte-identical to what it was (FR-031).

| Condition on the rule being added, versus each existing rule | Outcome |
|---|---|
| Same normalised prefix, either kind | `Err(DuplicatePrefixRule)` |
| Prefixes overlap (one is a prefix of the other) **and** kinds differ | `Err(ContradictoryPrefixRule)` |
| Prefixes overlap and kinds are the same | **Permitted** — redundant, not contradictory (FR-028) |
| Prefixes do not overlap | Permitted |

Symmetric by construction (FR-029): both the equality test and the overlap test are symmetric, and the
kind comparison does not depend on which rule is being added.

#### `admits` — the combination rule (FR-022 to FR-024)

1. No rules → everything passes (FR-018).
2. Matches any `Exclude` rule → hidden.
3. No `Include` rule exists → shown.
4. Otherwise → shown only if it matches an `Include` rule.

Steps 2 and 4 can never disagree about one event: two prefixes match the same bytes only when one is
a prefix of the other, and `add` refuses that pair across kinds. This is the invariant that lets the
combination rule have no precedence clause, and it must be stated in the doc comment because it is
enforced elsewhere in the file.

### `StoredDataPrefixFilter` (new, private)

```rust
#[serde(untagged)]
enum StoredDataPrefixFilter {
    Rules { rules: Vec<DataPrefixRule> },                   // 004 and later
    Legacy { mode: PrefixMode, prefixes: Vec<HexPrefix> },  // 001 through 003
}
```

`From<StoredDataPrefixFilter> for DataPrefixFilter` maps a legacy document's prefixes to rules of the
saved mode, **deduplicating by prefix** — the old field accepted the same prefix twice and the new
invariant forbids it. See [contracts/persisted-settings.md](./contracts/persisted-settings.md).

### `FilterSettings` — unchanged in shape

Still `{ kinds, channel_mode, data_filter }`. Its `admits` is untouched: three independent tests, all
of which must pass.

---

## 3. Application layer — `crates/midi-core/src/application/monitor.rs` (changed)

New field:

```rust
pub struct Monitor {
    // ... existing fields ...
    /// Whether arriving events are being retained. Session-only, never persisted.
    capture: CaptureState,
}
```

| Method | Change |
|---|---|
| `ingest` | **Gated**: returns `None` immediately when paused, before the selection check and before `log.push` |
| `capture_state()` | New. Read-only accessor for the DTO layer |
| `set_capture_state(CaptureState)` | New. Sets the state and nothing else — it does not clear the log, touch the catalogue, or close ports (FR-008) |
| `add_prefix_rule(DataPrefixRule) -> Result<(), CoreError>` | New. Delegates to `filter.data_filter.add` |
| `remove_prefix_rule(&HexPrefix) -> Result<(), CoreError>` | New. Delegates to `filter.data_filter.remove` |
| `set_data_prefix_filter` | **Removed.** Replaced by the two methods above |
| `set_filter` | Unchanged — still preserves the data filter when message kinds or channel change |
| `persisted_settings` | Unchanged — `capture` is deliberately not included |

---

## 4. Errors — `crates/midi-core/src/application/error.rs` (changed)

Three new `CoreError` variants. Each documents the condition that produces it and what the caller can
do, as every existing variant does.

```rust
DuplicatePrefixRule {
    /// The normalised prefix that is already listed, for quoting back.
    prefix: String,
    /// The kind the existing rule carries, so the message can name it.
    existing_kind: PrefixMode,
},
ContradictoryPrefixRule {
    /// The normalised prefix being added.
    prefix: String,
    /// The kind being added.
    kind: PrefixMode,
    /// The listed prefix it overlaps.
    existing_prefix: String,
    /// That rule's kind.
    existing_kind: PrefixMode,
},
UnknownPrefixRule {
    /// The prefix that is not in the list — a webview holding a stale rule list.
    prefix: String,
},
```

**Why two clash variants rather than one with a reason**: the messages and the remedies differ — delete
the duplicate, versus narrow one of the overlapping pair. The same reasoning already separates
`MalformedHexPrefix` from `EmptyHexPrefix`. It also keeps the decision in Rust: the webview renders a
sentence per variant instead of inspecting fields to work out which case it has.

`From<CoreError> for IpcError` stays exhaustive with no catch-all arm, so these three must be given
deliberate wire representations.

---

## 5. Wire types — `src-tauri/src/dto.rs` (changed)

| Type | Change |
|---|---|
| `CaptureStateDto` | **New.** `Running` \| `Paused`, tagged union, mirroring `CaptureState` one variant for one |
| `DataPrefixRuleDto` | **New.** `{ prefix: String, kind: PrefixModeDto }` |
| `PrefixModeDto` | Unchanged, and still converts both ways with `PrefixMode` |
| `FilterViewDto` | `prefix_mode` and `prefixes` **removed**; `rules: Vec<DataPrefixRuleDto>` added |
| `SnapshotDto` | `capture_state: CaptureStateDto` added, so every mutation reports it and the control can never disagree with the core |
| `EventDto`, `EventBatchDto`, `CatalogueDto`, `MutationResultDto`, `ColumnDto` | Unchanged |

`src/bindings.ts` is regenerated from these declarations; it is never hand-edited.

---

## 6. Webview state — `src/store.ts` (changed)

```ts
/** Whether the core is retaining arriving events, or holding what it has. */
captureState: CaptureStateDto;
```

Set by `applySnapshot` from every mutation and from the initial subscribe. `appendBatch` is
**unchanged** — no pause check is added there, because while paused nothing is streamed and a
webview-side guard would put a rule on the wrong side of the IPC boundary
([research.md](./research.md) D8).

---

## 7. Doc comments that carry the reasoning

Principle III makes these part of the deliverable, not a follow-up. Each states a *why* that the
signature does not:

1. `CaptureState` / `Monitor::ingest` — why pausing stops **retention** rather than display: the cap
   would otherwise evict exactly the rows the user paused to read.
2. `DataPrefixFilter::add` — why a clash is refused rather than resolved, and why the two clash cases
   are separate errors.
3. `DataPrefixFilter::admits` — the invariant that makes precedence unnecessary, and the consequence
   that `Hide` rules are inert beside any `Show only` rule (FR-024) — stated as intended, so it is not
   "fixed" later.
4. `DataPrefixRule` — why the prefix is the identity, and why there is no rule id.
5. `StoredDataPrefixFilter` — why the pre-004 shape is still read, and why the legacy conversion
   deduplicates.
6. `RetentionRow.tsx` — why `Pause` sits inside a row the screenshots depict (a recorded deviation,
   with its reason and its bounds), and why the cost of pausing is on the button's tooltip rather
   than in a line of text beside it.
7. `DataPrefixRules.tsx` — why the entry is committed on `Add` rather than applied per keystroke
   (inherited from the row it replaces), and why a refusal leaves the field's text alone so the user
   can correct it.
