# Contract: IPC Commands

**Feature**: [../spec.md](../spec.md) | **Date**: 2026-09-23

Two new commands. Both are declared in `src-tauri/src/commands.rs` with `#[tauri::command]` and
`#[specta::specta]`, registered in `src-tauri/src/lib.rs`, and reach TypeScript through the
regenerated `src/bindings.ts`. **`src/bindings.ts` is never hand-edited** — it carries the
generator's own warning, and Principle IV forbids a hand-maintained parallel definition.

---

## `get_display_model() -> IpcResult<DisplayViewDto>`

The `Display` tab's structure and current state, read once when the screen mounts.

Mirrors `get_filter_model` deliberately, including its reason for existing: the panel's entries
and labels come from Rust so the interface cannot invent an option or reword a control. The
strings are the normative content of `screenshots/setting.jpg` (see [../research.md](../research.md) D6).

## `set_display_settings(settings: DisplaySettingsDto) -> IpcResult<DisplayChangeDto>`

Replaces all six settings, persists them, and returns the updated model together with a snapshot
rendered under the new settings.

**Why one command rather than six**, and why it returns a snapshot: [../research.md](../research.md)
D7 and D1. In short — the tab is one form, no field has a validation error to report, and
returning the snapshot in the same payload leaves no interval in which the settings have changed
but the visible rows have not.

**Why it persists**: FR-028. It follows `set_retention_limit`, calling `state.persist(&monitor,
&sender)?` so the `send` half of the document is not erased — the mistake
`PersistedSettings::with_send` exists to make greppable.

**What it must not do**: discard the pending event batch. `set_capture_state` and `clear_events`
call `state.pump.discard_pending()` because they freeze or empty the list; a display change does
neither. Events queued in the pump when settings change are still arriving normally and must be
delivered. The webview's existing `high_water_mark` guard keeps them from duplicating rows the
returned snapshot already contains (FR-026).

---

## DTOs

All in `src-tauri/src/dto.rs`, `#[derive(Serialize, Type)]` with `#[serde(rename_all = "camelCase")]`,
matching the file's existing convention.

### `DisplaySettingsDto`

The six choices. Each field is a generated string union, never a bare `String` — the webview
cannot compose a value the core does not have (Principle IV).

| Field | TypeScript type |
|-------|-----------------|
| `time` | `"clockTime" \| "hostInteger" \| "hostSeconds" \| "hostNanoseconds"` |
| `note` | `"nameMiddleCthree" \| "nameMiddleCfour" \| "decimal" \| "hexadecimal"` |
| `controller` | `"standardName" \| "decimal" \| "hexadecimal"` |
| `data` | `"decimal" \| "hexadecimal"` |
| `program` | `"fromOne" \| "fromZero"` |
| `expert` | `boolean` |

`expert` crosses as a boolean because it is a checkbox on the wire; it becomes `ExpertMode::On` /
`Off` at the translation boundary, which is the IPC surface's job.

This DTO is both an input and an output type, so it derives `Deserialize` as well.

### `DisplayGroupDto`

One labelled radio group, so the webview renders structure it is handed rather than structure it
knows.

| Field | Type | Meaning |
|-------|------|---------|
| `id` | `string` | Stable identifier — which setting this group sets. |
| `label` | `string` | The verbatim group label: `Time format`, `Note format`, … |
| `sublabel` | `string \| null` | `(Decimal)` for `Program number`; `null` for the rest. The reference image gives that one group a two-line label. |
| `options` | `DisplayOptionDto[]` | In the image's order. |

### `DisplayOptionDto`

| Field | Type | Meaning |
|-------|------|---------|
| `id` | `string` | The wire value this option selects. |
| `label` | `string` | Verbatim, e.g. `Note (Middle C = C3)`, `1 – 128 (Standard)`. |
| `selected` | `boolean` | Exactly one `true` per group (FR-009). |

### `DisplayViewDto`

| Field | Type | Meaning |
|-------|------|---------|
| `groups` | `DisplayGroupDto[]` | The five radio groups, in the image's order. |
| `expertLabel` | `string` | `Expert mode`. |
| `expertEnabled` | `boolean` | Whether the checkbox is ticked. |
| `expertNotes` | `string[]` | The three explanatory lines, verbatim (FR-010). |
| `settings` | `DisplaySettingsDto` | The same state as values, for the webview to echo back on a change. |
| `tabs` | `PreferencesTabDto[]` | `Display`, `Sources`, `Other` — labels and availability (FR-004, FR-005). |

### `PreferencesTabDto`

| Field | Type | Meaning |
|-------|------|---------|
| `id` | `string` | `display` \| `sources` \| `other`. |
| `label` | `string` | Verbatim: `Display`, `Sources`, `Other`. |
| `available` | `boolean` | `true` only for `display` in this feature. |
| `unavailableNote` | `string \| null` | What an unavailable tab states. `null` when available. |

Carrying the two unimplemented tabs in the same model as the implemented one is what makes FR-005
enforceable: the webview cannot render an operable control for a tab the core marks unavailable,
and filling `Sources` later means flipping a flag and adding groups, not restructuring the screen
(Principle VII — extend rather than edit).

### `DisplayChangeDto`

| Field | Type | Meaning |
|-------|------|---------|
| `view` | `DisplayViewDto` | The model after the change — so the tab reflects what the core holds, including any value it defaulted (FR-030). |
| `snapshot` | `SnapshotDto` | Retained events re-rendered under the new settings (FR-025). |

This pairing follows `CatalogueChangeDto`, which already returns `{ snapshot, catalogue }` for the
same reason: one change, one atomic payload.

---

## Changed existing surface

### `EventDto::from_event`

Signature becomes `from_event(event, catalogue, settings, tick_rate)`. The `time` and `data`
fields are still pre-formatted strings — the webview's contract does not change shape, only what
determines the contents.

`SnapshotDto::from_monitor` already has the `Monitor`, which after this feature holds both the
settings and the tick rate, so no call site outside `dto.rs` gains a parameter.

### `IpcError`

No new variant. `DisplaySettingsDto` cannot carry an invalid state — every field is a generated
union — so there is no rejection to report. The only fallible paths are ones that already have
errors: `settingsUnavailable` when persistence fails (the change still applies, it is simply not
remembered) and the existing lock-poisoning path.

---

## Regenerating the bindings

`src/bindings.ts` is produced by `tauri-specta` when the app builds. After adding the commands
and DTOs, build once and commit the regenerated file. Definition of Done gate 9 requires this;
a hand-edited `bindings.ts` is a Principle IV violation.
