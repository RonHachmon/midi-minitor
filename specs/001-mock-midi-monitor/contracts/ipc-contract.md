# IPC Contract: Rust Core ↔ Webview

**Date**: 2026-08-26 · **Plan**: [plan.md](../plan.md) · **Data model**: [data-model.md](../data-model.md)

This is the application's only external interface: the boundary between the Rust core and the
React webview. Constitution IV governs it — declared once in Rust, TypeScript generated from those
declarations, no untyped data crossing, errors as typed variants.

**Generation, not duplication.** `src/bindings.ts` is produced by `tauri-specta` at debug-build time
from the Rust declarations. It is **generated output and MUST NOT be hand-edited**; this document
describes the contract's shape and rules, and the Rust source is its authority. The TypeScript below
is illustrative of what generation produces, not a file anyone maintains.

```text
Builder::new()
    .commands(collect_commands![...])
    .error_handling(ErrorHandlingMode::Result)
    .function_casing(Casing::CamelCase);

#[cfg(debug_assertions)]
builder.export(Typescript::default(), "../src/bindings.ts")?;
```

## Design rules for this boundary

1. **The IPC layer holds no business rules.** Each handler validates its input, delegates to
   `Monitor`, and maps `CoreError` to `IpcError`. A handler that decides whether an event is visible
   is a Constitution IV violation.
2. **Nothing untyped crosses.** No `serde_json::Value` in a signature, no `any` or unchecked cast in
   the webview.
3. **Wire types are DTOs, not domain types.** `src-tauri/dto.rs` owns them. The domain must not grow
   `specta::Type` derives, because that would make the core's public shape hostage to the webview's
   serialization needs (Constitution I: one reason to change).
4. **The Data column is rendered in Rust.** The DTO carries the finished display string
   (`"C2 127"`). Rendering is a domain rule about MIDI, not a presentation detail, and duplicating
   it in TypeScript would create the second source of truth Constitution IV exists to prevent.

## Wire types

```text
EventDto {
  id: number
  time: string          // "18:31:23.572" — pre-formatted (FR-009)
  source: string        // display name (FR-010)
  message: string       // "Note On", "Channel Pressure" (FR-011)
  channel: number | null   // null for messages carrying no channel (FR-012)
  data: string          // "C2 127", "0", "3 bytes" (FR-013)
  rawHex: string        // "903C7F" — backs the raw-bytes view (FR-015)
}

SourceDto      { id: number, name: string, group: string | null, selected: boolean }
SourceGroupDto { name: string, expanded: boolean, sources: SourceDto[] }
MessageKindDto { id: MessageKindId, label: string, category: CategoryId, enabled: boolean }

FilterSettingsDto {
  kinds: MessageKindId[]                 // enabled kinds
  channelMode: { type: "allChannels" } | { type: "oneChannel", data: number }
  dataFilter: { mode: "include" | "exclude", prefixes: string[] }
}

SnapshotDto {
  events: EventDto[]        // full filtered view, oldest first
  retainedCount: number     // events held in the log, before filters
  monitoring: boolean       // false when no source is selected (FR-027)
}

EventBatchDto { events: EventDto[] }   // streamed; see Channel below
```

`channel: number | null` rather than an optional key: the Chan cell must be *blank*, and a missing
key would let a renderer print `undefined`.

`MessageKindId` and `CategoryId` are generated string-union types (e.g. `"noteOnOff" | "control" |
…`), not free `string`. A typo in a filter id is then a compile error, which — with no test suite —
is the only place it can be caught.

## Commands

All are `async` on the TypeScript side and return `Result<T, IpcError>` under
`ErrorHandlingMode::Result`.

| Command | Input | Returns | Requirements |
|---------|-------|---------|--------------|
| `getCatalogue` | — | `SourceGroupDto[]` | FR-021, FR-022 |
| `getFilterModel` | — | `MessageKindDto[]` | FR-028 — the panel's structure comes from Rust, so the UI cannot invent a category |
| `getSettings` | — | `SettingsDto` | FR-047 — restores persisted state at startup |
| `snapshot` | — | `SnapshotDto` | FR-034 |
| `setSourceSelected` | `sourceId, selected` | `SnapshotDto` | FR-023, FR-026 |
| `setGroupSelected` | `group, selected` | `SnapshotDto` | FR-024 |
| `setFilter` | `FilterSettingsDto` | `SnapshotDto` | FR-030, FR-033, FR-034 |
| `setDataPrefixFilter` | `mode, prefixes[]` | `SnapshotDto` | FR-035–FR-041 |
| `setRetentionLimit` | `limit` | `SnapshotDto` | FR-016, FR-018, FR-019 |
| `clearEvents` | — | `SnapshotDto` | FR-020 |
| `setColumnVisibility` | `Column[]` | `()` | FR-042–FR-046 — no snapshot; display only |
| `subscribeEvents` | `Channel<EventBatchDto>` | `()` | The stream; see below |

**Every mutating command returns a fresh `SnapshotDto`.** This is the mechanism behind FR-034: the
webview never patches its own list after a settings change, it replaces it. One round trip, one
source of truth, and no possibility of the visible list disagreeing with the controls.

`setColumnVisibility` is the deliberate exception — returning a snapshot there would imply
visibility affects which events are listed, which FR-046 forbids.

## The event stream

`subscribeEvents` takes a `tauri::ipc::Channel<EventBatchDto>`, opened once at mount:

```text
import { Channel } from "@tauri-apps/api/core";
const channel = new Channel<EventBatchDto>();
channel.onmessage = (batch) => { /* buffer, flush on rAF */ };
await commands.subscribeEvents(channel);
```

**Why a channel and not `emit`/`listen`**: Tauri documents the event system as *"not designed for
low latency or high throughput situations"* with *"no strong type support"* — failing both SC-005
(500 events/s) and Constitution IV. Channels are the documented streaming mechanism, and
`tauri-specta` renders `Channel<T>` as a typed `Channel<T>` imported from `@tauri-apps/api/core`, so
the stream stays inside the generated contract rather than being an untyped hole in it.

**Batching** (`src-tauri/stream.rs`): the pump coalesces admitted events and sends at most one batch
per ~16 ms. At 500 events/s that is ~60 messages/s of ~8 events each instead of 500 individual
round-trips. The interval is a named constant, not a literal.

**Only filter-passing events are streamed.** Suppressed traffic never crosses the boundary — which
is both the Constitution IV requirement and the reason unchecking `Clock` actually reduces load.

**Ordering and dedup**: `EventDto.id` is monotonic. The webview appends batches in arrival order and
trims to the retention limit locally for rendering; the authoritative log lives in Rust. After any
snapshot the local buffer is discarded and replaced, so a batch in flight during a settings change
cannot resurrect a filtered-out event — the webview drops batches whose ids precede the snapshot's
high-water mark.

## Errors

`IpcError` crosses as a discriminated union — never a string (Constitution IV). Under
`ErrorHandlingMode::Result`, `tauri-specta` generates:

```text
type IpcError =
  | { type: "channelOutOfRange"; data: { value: number } }
  | { type: "retentionOutOfRange"; data: { value: number } }
  | { type: "malformedHexPrefix"; data: { entry: string } }
  | { type: "emptyHexPrefix" }
  | { type: "lastColumnVisible" }
  | { type: "unknownSource"; data: { id: number } }
  | { type: "settingsUnavailable"; data: { detail: string } }
```

Each variant carries what a caller needs to act: the offending value, so the UI can explain the
rejection at the control that caused it rather than showing a generic failure. `malformedHexPrefix`
is the one users will see most (FR-041), and its contract is that the **previous valid filter stays
in effect** — the command fails without mutating state.

## Persistence

`SettingsDto` — sources selection, filter settings, prefix filter and mode, column visibility,
retention limit — is written through `SettingsRepository` to `settings.json` via
`tauri-plugin-store` (FR-047). Retained events are never persisted. Settings are saved after each
successful mutating command; the plugin's own debounced auto-save absorbs rapid changes such as
dragging through checkboxes.

## Contract change rules

- Changing a command signature, DTO field, or error variant means regenerating `bindings.ts` in the
  same change (Definition of Done gate 9). A stale `bindings.ts` is a broken build, not a runtime
  surprise — which is the point.
- New capability is added as a **new** command or a **new** DTO field, never by widening a field to
  a looser type. Loosening a type to avoid a change is how a typed contract decays into an untyped
  one.
- Adding a `MessageKind` or `Column` variant is a compile error at every exhaustive `match`
  (Constitution VII). That is intended: the compiler enumerates the work.
