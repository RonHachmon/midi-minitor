# Contract: Persisted Settings

**Feature**: [../spec.md](../spec.md) | **Plan**: [../plan.md](../plan.md) | **Date**: 2026-08-29

What this feature adds to the on-disk settings, and why it needs no migration.

Storage is unchanged: `tauri-plugin-store` JSON, reached through the existing `SettingsRepository`
port. This feature adds no file, no second store, and no new failure mode.

---

## The change

One field is added to `PersistedSettings`. Nothing is renamed, removed, or reshaped.

```rust
pub struct PersistedSettings {
    pub selected_sources: Vec<SourceKey>,
    pub filter: FilterSettings,
    pub columns: ColumnVisibility,
    pub retention: RetentionLimit,

    /// The send screen's state. Absent in documents written before this feature.
    #[serde(default)]
    pub send: SendSettings,
}

#[derive(Default)]
pub struct SendSettings {
    /// The chosen target, by the identity that survives a replug. `None` until one is chosen.
    pub target: Option<TargetKey>,
    /// The disguise: the name, and whether it is published.
    pub publication: Publication,
    /// The user's own requests. Built-ins are code, not data.
    pub saved_requests: Vec<SavedRequest>,
}

pub struct SavedRequest {
    pub name: String,
    pub description: String,
    pub composition: PersistedComposition,
}
```

---

## Why no migration is written

Feature 004 needed one, and the difference is worth stating so the machinery it left behind is not
reached for out of habit.

004 **changed the shape** of an existing field: `data_filter` went from one mode plus a list of
prefixes to a list of rules. Because `StoreSettingsRepository::load` treats an unreadable document as
absent, a naive change there would have silently discarded source selections, columns, and retention
along with the filter — so it needed `#[serde(from = ...)]` over an untagged enum to read both shapes.

This feature **adds a field and changes none**. A document written by features 001 through 004 has no
`send` key; `#[serde(default)]` supplies an empty `SendSettings`, and every other field deserializes
exactly as before. There is no second shape to read, so writing a migration would be dead code.

**Forward compatibility is equally free**: a document written by this feature, opened by an older
build, carries an unknown `send` key that serde ignores. The user loses their send settings on that
launch and keeps everything else — the correct outcome for a downgrade, and it happens without either
version knowing about the other.

---

## Field obligations

### `target: Option<TargetKey>`

Stored as a key, never as a session `TargetId`, for the reason `selected_sources` is stored as
`SourceKey`: ids are minted per session and mean nothing on the next launch.

On load, the key is matched against the current target list. A target that is not present is **not an
error and not cleared** — it is simply not chosen this session, and the screen says nothing is chosen.
The key is retained so that reattaching the device restores the choice, which is what FR-023 asks for.

`TargetKey::PublishedSource` is stored like any other and resolves whenever publishing is on.

### `publication: Publication`

```rust
Publication { name: PublishedName, published: bool }
```

Both fields persist (FR-025, FR-026). Defaults on first run: name `MIDI Monitor`, `published: false`.

**Publishing is restored at launch**, so a user who left the disguise on finds their device present in
the receiving program without touching this screen. If republishing fails — or if the settings were
written on macOS and opened on Windows — the state is reported as unpublished with the reason, and
nothing else is disturbed. A saved `published: true` is a request, not a guarantee, and the platform
has the final say.

A name saved on a platform that can publish and loaded on one that cannot is kept, not discarded: the
same profile may be carried back, and quietly erasing a user's chosen name would be a worse outcome
than an inert value.

### `saved_requests: Vec<SavedRequest>`

Order is the order the user created them, preserved on write and on read.

Names are unique across saved requests and built-ins, enforced when a request is saved or renamed. A
document that somehow contains a duplicate — hand-edited, or written by a future version — is loaded
with the first occurrence kept and the rest dropped, rather than refused: losing one saved request is
better than losing every setting in the file.

`PersistedComposition` records enough to rebuild the composition exactly: for a guided request the
kind and its field values; for a raw request the bytes as typed. A saved request must transmit
identical bytes after a restart (FR-040, SC-010), which is why the bytes of a raw request are stored
rather than re-derived from a decoded message.

---

## What is deliberately not persisted

| Not stored | Why |
|---|---|
| Send records | Scoped to the session by FR-030. A record naming a device that may be gone has no value on a later launch, and the *time* column would be misleading across days. |
| The current composition | Session state. FR-019 requires it to survive moving between screens, not a restart — and restoring a half-built message into a screen the user reopens days later is surprising rather than helpful. |
| The target list | A snapshot of the machine, re-read at launch and on every device change. Caching it would let the screen show a device that is not there. |
| `PublicationSupport` | Describes the machine this build is running on, read from the adapter at startup. Persisting it would let a profile copied between platforms claim a capability the current one does not have. |
| Built-in requests | They are code. Storing them would let a stale copy on disk contradict FR-011. |

---

## Compatibility summary

| Document written by | Read by this feature | Result |
|---|---|---|
| 001–004 | ✓ | `send` defaults; every other setting preserved |
| This feature | ✓ | Full round trip |
| This feature | 001–004 build | `send` ignored; every other setting preserved |
| Hand-edited with a bad `send` block | ✓ | The malformed field defaults; the rest of the document is still read |

The last row is the one that matters in practice, and it is why `send` is a single `#[serde(default)]`
field rather than several fields at the top level: a defect in the new block cannot take the user's
source selections, columns, retention, or filter rules down with it.
