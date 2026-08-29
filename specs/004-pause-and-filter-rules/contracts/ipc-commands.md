# Contract: The IPC Command Surface

**Feature**: [../spec.md](../spec.md) | **Plan**: [../plan.md](../plan.md) | **Date**: 2026-08-29

The contract is declared in Rust and `src/bindings.ts` is generated from it. This document records
what changes and why; the generated file is the authority on shape, and it is never hand-edited.

Three commands are added and one is removed. Every mutating command continues to return a
`SnapshotDto`, so the webview replaces its list rather than patching it.

---

## Added — `set_capture_state`

```rust
#[tauri::command]
pub fn set_capture_state(
    state: State<'_, AppState>,
    capture: CaptureStateDto,
) -> IpcResult<SnapshotDto>
```

| Aspect | Contract |
|---|---|
| Effect | Sets the monitor's capture state. Nothing else — no log change, no catalogue change, no port opened or closed (FR-008) |
| Pump | Calls `pump.discard_pending()`, so a batch queued up to 16 ms earlier cannot land after the pause takes effect (FR-004) |
| Persistence | **None.** The capture state is session-only (FR-012), so this handler does not call `persist` |
| Returns | A snapshot, carrying the frozen event list and the new `capture_state` |
| Idempotent | Yes. Pausing an already-paused monitor is a no-op that returns the current snapshot |
| Errors | `monitorUnavailable` only |

**Why one command with a state rather than `pause` and `resume`**: it matches the `set_*` naming every
other mutation uses, it is idempotent by construction, and a webview that has drifted cannot toggle
into the state opposite the one the user asked for.

---

## Added — `add_data_prefix_rule`

```rust
#[tauri::command]
pub fn add_data_prefix_rule(
    state: State<'_, AppState>,
    prefix: String,
    kind: PrefixModeDto,
) -> IpcResult<SnapshotDto>
```

| Aspect | Contract |
|---|---|
| Validation order | `HexPrefix::parse` first, then the clash rule. Both fail **before** any mutation (FR-031) |
| Effect on success | Appends the rule, discards pending stream events, persists the settings |
| Returns | A snapshot. The webview then calls `refreshPanels()` for the updated `FilterViewDto`, the same two-step the prefix field already used |
| Errors | `malformedHexPrefix`, `emptyHexPrefix`, `duplicatePrefixRule`, `contradictoryPrefixRule`, `settingsUnavailable`, `monitorUnavailable` |

---

## Added — `remove_data_prefix_rule`

```rust
#[tauri::command]
pub fn remove_data_prefix_rule(
    state: State<'_, AppState>,
    prefix: String,
) -> IpcResult<SnapshotDto>
```

| Aspect | Contract |
|---|---|
| Key | The normalised prefix — unique within the list, because the clash rule refuses a repeat under either kind |
| Effect | Removes that one rule, leaving every other rule in force (FR-016). Discards pending stream events, persists |
| Returns | A snapshot; the rule list follows from `refreshPanels()` |
| Errors | `malformedHexPrefix`, `emptyHexPrefix`, `unknownPrefixRule`, `settingsUnavailable`, `monitorUnavailable` |

`unknownPrefixRule` means the webview is holding a rule list that no longer matches the core's — the
same situation `unknownSource` and `unknownGroup` already describe. The remedy is to refresh, not to
retry.

---

## Removed — `set_data_prefix_filter`

```rust
// GONE: pub fn set_data_prefix_filter(state, mode: PrefixModeDto, prefixes: Vec<String>)
```

It could only express one mode for every prefix, which is the limitation this feature exists to
remove. Removing rather than deprecating is deliberate: leaving it in place would be dead code, and
its removal makes the stale webview call site a `tsc` error instead of a silent second path that can
overwrite the rule list.

---

## Changed — `FilterViewDto`

```diff
  pub struct FilterViewDto {
      pub categories: Vec<MessageCategoryDto>,
      pub standalone: Vec<MessageKindDto>,
      pub channel_mode: ChannelModeDto,
-     pub prefix_mode: PrefixModeDto,
-     pub prefixes: Vec<String>,
+     pub rules: Vec<DataPrefixRuleDto>,
  }
```

Returned by `get_filter_model`, unchanged in every other respect. The panel's labels and ordering
still come from Rust so the interface cannot invent or reword an entry.

## Changed — `SnapshotDto`

```diff
  pub struct SnapshotDto {
      pub events: Vec<EventDto>,
      pub retained_count: u32,
      pub retention_limit: u32,
      pub monitoring: bool,
      pub high_water_mark: Option<u32>,
+     pub capture_state: CaptureStateDto,
  }
```

`monitoring` and `capture_state` are independent and both are needed: "no source is selected" and
"the user paused" are different situations calling for different actions, and an interface that
conflated them would explain an empty list wrongly in one of the two cases.

## Added — `CaptureStateDto`, `DataPrefixRuleDto`

```rust
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum CaptureStateDto { Running, Paused }

#[serde(rename_all = "camelCase")]
pub struct DataPrefixRuleDto { pub prefix: String, pub kind: PrefixModeDto }
```

`CaptureStateDto` follows the house's tagged-union shape so the webview matches it exhaustively and a
third capture state would become a type error rather than a silent default.

---

## Added error variants — `IpcError`

```rust
DuplicatePrefixRule    { prefix: String, existing_kind: PrefixModeDto },
ContradictoryPrefixRule{ prefix: String, kind: PrefixModeDto,
                         existing_prefix: String, existing_kind: PrefixModeDto },
UnknownPrefixRule      { prefix: String },
```

`describeError` in `src/ipc.ts` is an exhaustive switch with no default arm, so these three must be
given sentences before the webview compiles. Each sentence must name the entered prefix *and* the
existing rule it clashes with (FR-030, SC-006); the phrasing belongs with the implementation, but it
must carry both facts and must not require the user to go and read the list to work out what
happened.

`From<CoreError> for IpcError` stays exhaustive — a new core variant must be given a deliberate wire
representation rather than folded into a generic bucket.

---

## Unchanged

`get_catalogue`, `subscribe_catalogue`, `get_columns`, `snapshot`, `subscribe_events`,
`set_source_selected`, `set_group_selected`, `set_filter`, `set_retention_limit`, `clear_events`,
`set_column_visibility` — all unchanged in signature and behaviour.

`clear_events` and the filter commands are worth naming explicitly: they behave identically whether
the monitor is running or paused (FR-007, FR-006), and they need no change to do so, because
filtering has always been a view over the retained log rather than an admission gate.
