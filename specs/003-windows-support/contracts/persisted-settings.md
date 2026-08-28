# Contract: Persisted Settings

**Feature**: `003-windows-support` | **Location**: `crates/midi-core/src/application/settings.rs`,
written through `SettingsRepository` (`tauri-plugin-store`)

The on-disk contract. It matters here for one reason: FR-050 requires that settings written by the
previous version on macOS load and apply unchanged, with nothing reset or migrated away. This
document exists to show that the change satisfies that by construction rather than by care.

---

## What changes

`PersistedSettings` keeps every field and every type it has today. The **only** difference is that
`selected_sources: Vec<SourceKey>` may now contain entries of a second variant shape.

```text
PersistedSettings
├── selected_sources: Vec<SourceKey>     shape unchanged; entries may hold the new variant
├── filter: FilterSettings               unchanged
├── columns: ColumnVisibility            unchanged
└── retention: RetentionLimit            unchanged
```

## The on-disk shape

`SourceKey` derives serde's default **externally tagged** representation, so each entry is a
single-key object named after its variant.

| Variant | Written as | Produced by |
|---|---|---|
| `Endpoint(i32)` | `{"Endpoint": 1234567}` | macOS — CoreMIDI's signed unique-id property |
| `DeviceInterface(String)` | `{"DeviceInterface": "\\\\?\\USB#VID_…#…"}` | Windows — the device-interface string (research D3) |
| `VirtualDestination` | `"VirtualDestination"` | either platform, when the standalone row was ticked |

Example of a file written by the current macOS build, and still valid after this feature:

```json
{
  "selected_sources": [ { "Endpoint": 1234567 }, "VirtualDestination" ],
  "filter":  { "…": "unchanged" },
  "columns": { "…": "unchanged" },
  "retention": 1000
}
```

## Compatibility guarantees

| Direction | Result |
|---|---|
| **Old macOS file → new macOS build** | Loads unchanged. `Endpoint` is untouched, and adding a sibling variant does not alter how an existing one deserializes. **This is FR-050.** |
| **New macOS file → new macOS build** | Identical to today. macOS never writes `DeviceInterface`. |
| **New Windows file → new Windows build** | Round-trips. Selections return on relaunch (FR-017, SC-006). |
| **Old macOS file → new Windows build** (a copied or synced profile) | Loads. Its `Endpoint` keys match no local port, so they are **retained and match nothing** — which is precisely the spec's stated edge case for cross-platform profiles, not a failure. |
| **Windows file → macOS build** | Symmetric, same reasoning. |
| **Downgrade**: new Windows file → an older build | The old build's `SourceKey` has no `DeviceInterface` variant, so deserialization of `selected_sources` fails and the store falls back to first-run defaults. Acceptable and not guarded: downgrading across a feature is not a supported path, and the failure is a reset of selections, not data loss or a crash. |

**No migration step, no schema version, no rewrite on load.** Adding a variant is the whole change.
That is the substance of research D3's argument for a new variant over widening `Endpoint(i32)`,
which would have invalidated every existing macOS file.

## Retention semantics, unchanged

- A selection for a port that is **not currently present** is retained, never discarded, so the device
  returns ticked when reattached (FR-018). `Monitor.remembered` holds these alongside the live
  catalogue.
- A port present at launch that the file does not mention starts **unticked** (FR-019).
- With **no settings file at all**, every discovered source starts selected — first-launch behaviour,
  unchanged from feature 002.

## What is still not persisted

- **Observed events.** Settings describe configuration; events are observations of a moment.
- **`PlatformCapabilities`.** It describes the machine the application is running on right now.
  Restoring a saved copy onto a different machine would state a fidelity promise that machine does not
  make — exactly the kind of lie this feature exists to prevent.
- **`Availability`.** A runtime fact about this instant, recomputed on every scan.

## Effect on the generated TypeScript bindings: none

`SourceKey` does not cross the IPC boundary. Its own doc comment says so, and this was verified rather
than assumed — `grep -rn SourceKey src-tauri/` returns nothing. The webview keys rows on `SourceId`,
which is unchanged.

The bindings **do** regenerate for this feature, but for an unrelated and additive reason: the
`SourceDto` availability field becomes a tagged union and the catalogue gains a byte-fidelity field.
See [event-source-port.md](./event-source-port.md) and [../data-model.md](../data-model.md) §5.
`src/bindings.ts` is generated from the Rust declarations and is never hand-edited.
