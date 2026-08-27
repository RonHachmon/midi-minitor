# IPC Contract Delta: Real MIDI Input

**Feature**: [spec.md](../spec.md) | **Data model**: [data-model.md](../data-model.md) | **Date**: 2026-08-27

This is a **delta** against the contract established by feature 001. Anything not listed here is
unchanged, which FR-037 requires and which is the clearest evidence the port seam held: the event path,
the filter panel, the columns, and retention cross the boundary exactly as before.

**The contract is declared in Rust and generated into `src/bindings.ts`** by `tauri-specta` at debug
startup (Constitution IV). The TypeScript below is illustrative of what generation produces — it is
never hand-written, and `src/bindings.ts` stays gitignored.

---

## 1. New: the MIDI system's own status

Real hardware introduces a state a simulator could not have: the application may be unable to reach the
MIDI system at all. FR-008 and SC-016 require this to be distinguishable from "no devices found", so it
crosses as its own type rather than as an empty list.

```rust
#[derive(Serialize, Type)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum MidiSystemStatusDto {
    Available,
    Unavailable { detail: String },
}
```

```typescript
type MidiSystemStatusDto =
  | { type: "available" }
  | { type: "unavailable"; data: { detail: string } };
```

A tagged union, not a nullable string: `detail: string | null` would let the webview render a reason
while the system is fine, and would make "available" and "unavailable with no detail" indistinguishable.

---

## 2. Changed: `SourceDto` carries availability

```rust
pub struct SourceDto {
    pub id: u32,
    pub name: String,
    pub selected: bool,
    pub unavailable: Option<String>,   // NEW
}
```

| Value | Meaning | Requirement |
|---|---|---|
| `null` | Present and open — renders exactly as today | — |
| `"Remembered from a previous session"` | Selected once, not currently attached | FR-033 |
| a reason, e.g. `"In use by another application"` | Present but could not be opened | FR-013 |

`Option<String>` rather than a boolean plus a message, because an unavailable row without a reason is a
state the interface must never have to render.

**Screenshot fidelity (FR-036)**: an available row is byte-identical to today's. The reason is
additional information on rows the reference screenshots never depicted, since the reference had no way
to show an absent device.

---

## 3. Changed: `SourceGroupDto` can declare itself unavailable

```rust
pub struct SourceGroupDto {
    pub id: Option<String>,
    pub label: Option<String>,
    pub state: CheckStateDto,
    pub sources: Vec<SourceDto>,
    pub unavailable_reason: Option<String>,   // NEW
}
```

This carries the deferred spy group (FR-028, FR-029). The `spyOnOutput` group is always emitted, always
with an empty `sources`, and always with:

```text
unavailableReason: "Observing output to destinations is not available in this version."
```

The group keeps its verbatim label and its position between the standalone row and the end of the list,
per Constitution VI. It renders as a heading with an explanation where its children would be — no
placeholder rows, which FR-029 forbids outright.

---

## 4. New: `CatalogueDto` — the catalogue plus how it was obtained

`get_catalogue` currently returns `Vec<SourceGroupDto>`. Hot-plug and system status both need to travel
with it, and a bare vector has nowhere to put them.

```rust
pub struct CatalogueDto {
    pub groups: Vec<SourceGroupDto>,
    pub midi_system: MidiSystemStatusDto,
}
```

**Breaking change to `get_catalogue`'s return type.** Acceptable and, in fact, the mechanism the project
relies on: the webview compiles against generated bindings, so this surfaces as a TypeScript error at
the one call site rather than as a runtime shape mismatch. That is the trade Constitution IV was
written to buy.

---

## 5. New command: `subscribe_catalogue`

```rust
#[tauri::command]
#[specta::specta]
pub fn subscribe_catalogue(
    state: State<'_, AppState>,
    channel: Channel<CatalogueDto>,
) -> IpcResult<CatalogueDto>;
```

Registers the webview's channel for hot-plug pushes and returns the current catalogue in the same round
trip — the same pattern `subscribe_events` already uses, so the webview never has a window where it is
subscribed but has not yet rendered.

**Why a channel and not `emit`**: the reasoning in [`stream.rs`](../../../src-tauri/src/stream.rs)
applies unchanged — Tauri's event system is untyped, and the contract must be typed end to end.

**Why a second channel rather than folding into the event stream**: they have unrelated rates and
unrelated payloads. Events arrive hundreds per second and are batched on a 16 ms timer; catalogue
changes arrive when someone physically touches a cable. Sharing one channel would force a sum type onto
the hot path and make the batching timer the floor for hot-plug latency.

**Delivery**: a catalogue push is *coalesced and debounced*, not sent per notification. macOS emits a
burst of `Notification`s for one physical connection (object added, then setup changed); pushing each
would rebuild the Sources panel several times for one plug event. A short debounce collapses the burst
into one push, comfortably inside SC-005's two-second bound.

---

## 6. Changed: `set_source_selected` and `set_group_selected` return the catalogue too

Both currently return `SnapshotDto`. Selecting a source can now change *availability* — a port that
fails to open reports it at selection time (FR-013) — which `SnapshotDto` has no field for.

```rust
pub struct MutationResultDto {
    pub snapshot: SnapshotDto,
    pub catalogue: CatalogueDto,
}
```

Applies to `set_source_selected` and `set_group_selected` only. The other mutating commands — filter,
prefix, retention, clear, columns — cannot affect the catalogue and keep returning `SnapshotDto`
unchanged.

**Why not always return both**: a filter change returning a catalogue would invite the webview to
re-render the Sources panel on every checkbox tick, and would imply a coupling that does not exist.

---

## 7. Unchanged, and deliberately so

| Item | Note |
|---|---|
| `EventDto` | Real events render through the identical wire type. The strongest single sign the seam held |
| `EventBatchDto`, `subscribe_events` | Batching is unaffected by where events come from |
| `SnapshotDto` | `monitoring` still means "at least one source selected" — distinct from system status |
| `FilterViewDto`, `MessageKindDto`, `MessageCategoryDto` | The Filter panel is generated from Rust and untouched (FR-021) |
| `ColumnDto`, `set_column_visibility` | Untouched |
| `set_filter`, `set_data_prefix_filter`, `set_retention_limit`, `clear_events` | Untouched |
| `ChannelModeDto`, `PrefixModeDto`, `CheckStateDto` | Untouched |
| `wire_group_id` | `midiSources` / `spyOnOutput` keep their identifiers |

---

## 8. Error surface

`IpcError` gains no variant for MIDI failures, which is a deliberate choice.

A device that will not open, or a MIDI system that cannot be reached, is **not a failed command** — it
is a state the user needs to see and keep working around. Modelling it as an error would push it into a
rejected promise at the call site and give the webview nothing to render in the Sources panel, which is
where FR-013 and FR-008 require the information to appear. So these travel as *data* on `SourceDto` and
`MidiSystemStatusDto`.

`IpcError` variants stay as they are: `MonitorUnavailable`, `SettingsUnavailable`, `UnknownGroup`, and
the domain errors mapped from `CoreError`.

---

## 9. Frontend call sites affected

Generated bindings turn each of these into a compile error until updated — the intended mechanism.

| File | Change |
|---|---|
| [`src/ipc.ts`](../../../src/ipc.ts) | `getCatalogue` return shape; add `subscribeCatalogue`; `setSourceSelected` / `setGroupSelected` now return `MutationResultDto` |
| [`src/store.ts`](../../../src/store.ts) | Hold `midiSystem`; apply pushed catalogues; keep selection state keyed on `SourceDto.id` |
| [`src/components/SourcesPanel.tsx`](../../../src/components/SourcesPanel.tsx) | Render `unavailable` on rows, `unavailableReason` on the spy group, and the empty-list and system-unavailable states |
| [`src/App.tsx`](../../../src/App.tsx) | Subscribe to the catalogue channel alongside the event channel |

**Constitution VI applies to every one of these.** The new text is confined to states the reference
screenshots could not depict. No existing control is relabelled, reordered, restyled, or repurposed —
the disclosure triangles, the checkboxes, the bordered list, the group indentation, and the
`Remember up to [N] events` row are all untouched.
