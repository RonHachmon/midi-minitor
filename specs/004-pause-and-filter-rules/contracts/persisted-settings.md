# Contract: The On-Disk Settings Shape

**Feature**: [../spec.md](../spec.md) | **Plan**: [../plan.md](../plan.md) | **Date**: 2026-08-29

The settings document lives in the platform's config directory, written by `tauri-plugin-store` under
the file `settings.json` and the single key `settings`. One field changes shape in this feature.
Everything else is untouched.

---

## What changes

`PersistedSettings.filter.data_filter` only.

**Written by 001 through 003** — one mode applying to every prefix:

```json
"data_filter": {
  "mode": "Include",
  "prefixes": ["90", "B007"]
}
```

**Written by 004 and later** — a kind per rule:

```json
"data_filter": {
  "rules": [
    { "prefix": "90",   "kind": "Include" },
    { "prefix": "B007", "kind": "Exclude" }
  ]
}
```

`selected_sources`, `columns`, and `retention` are unchanged, as are `filter.kinds` and
`filter.channel_mode`.

**The capture state is deliberately not persisted.** A freshly launched monitor is running (FR-012).
A paused monitor restored from disk would open a window that looks broken and says nothing about why.

---

## Reading an older document

`DataPrefixFilter` carries `#[serde(from = "StoredDataPrefixFilter")]`, where the intermediate is an
untagged enum listing the current shape first and the pre-004 shape second. Serde tries each variant
in order; the two share no field names, so neither can satisfy the other's required fields and the
ordering cannot produce a mis-parse.

The conversion:

| Legacy input | Result |
|---|---|
| `{ mode: M, prefixes: [p1, p2] }` | `[{ prefix: p1, kind: M }, { prefix: p2, kind: M }]` — every prefix keeps the mode it was saved with (FR-025) |
| A prefix repeated in the legacy list | Kept once. The old field accepted `90 90`; the new list may not hold one prefix twice |
| `{ mode: M, prefixes: [] }` | `{ rules: [] }` — an empty filter, admitting everything, exactly as before |

A migrated list can never violate the clash rule: every rule takes the same kind, and same-kind
overlaps are permitted (FR-028). The only invariant at risk is uniqueness, which the deduplication
above preserves.

---

## Why a migration at all, rather than the existing fallback

`StoreSettingsRepository::load` already treats a document this build cannot parse as absent:

> A settings document this build cannot read is treated as absent rather than fatal: an older or newer
> file on disk must never stop the application from starting, and defaults are always a valid state.

That rule is correct for a document that is genuinely unreadable, and wrong as a substitute for a
migration. `data_filter` is one field inside a document that also carries which devices the user had
selected, which columns they had hidden, and how many events they wanted remembered. Letting the
fallback fire would silently discard all of it to avoid converting a two-field object — and the user
would have no way to tell that had happened, because a first-run state looks completely normal.

---

## Writing

Serialization is unaffected by `#[serde(from = ...)]`, so the new shape is what gets written. The
first save after upgrading rewrites `data_filter` in the new form; from then on the legacy arm is
only exercised by a document that predates the upgrade.

## Forward compatibility

A 003-or-earlier build reading a 004 document sees `data_filter` with no `mode` or `prefixes`, fails
to parse `PersistedSettings`, and falls back to defaults for the whole document — the existing rule,
behaving exactly as documented. Downgrading is not supported and was not before; nothing here makes
it worse, and nothing here makes it better.

## Version field

None is introduced. One field changed shape once, and the change is expressible structurally. A
version number becomes worth adding when a change cannot be told apart by shape — at which point it
should be added deliberately, with a migration path, rather than retrofitted here on speculation.
