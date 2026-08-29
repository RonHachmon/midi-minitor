# Contract: The IPC Command Surface

**Feature**: [../spec.md](../spec.md) | **Plan**: [../plan.md](../plan.md) | **Date**: 2026-08-29

Thirteen commands added, one subscription added, none removed, none changed. Every existing command
keeps its name, signature, and behaviour — the monitoring screen's contract is untouched.

All types below are declared in Rust and regenerated into `src/bindings.ts` by `tauri-specta`. The
webview never declares a parallel type, and `src/bindings.ts` is never edited by hand.

---

## Rules this surface follows

These are the project's existing rules, restated because every command below is bound by them.

1. **Handlers validate, delegate, persist, and map errors.** No handler decides what a message means,
   what may be sent, or what is shown. Those decisions are in `midi-core`.
2. **Errors cross as typed variants, never strings.** Each `CoreError` maps to exactly one `IpcError`.
3. **No untyped data crosses.** No `serde_json::Value` in a signature, no `any` at the invoke
   boundary.
4. **Every mutating command returns the full `SendViewDto`.** The webview replaces its model rather
   than patching it — the convention `set_filter` and `set_retention_limit` already follow with
   `SnapshotDto`. It removes any possibility of the two sides disagreeing about what is composed.
5. **A refused command mutates nothing.** There is no partially applied send, no half-saved request,
   and no composition left in a state the user did not ask for.

---

## Reading

### `get_send_view() -> SendViewDto`

The whole send screen model: targets, chosen target, publication, composition with its fields and byte
preview, the request library, and the session's send records. Called once when the screen first
mounts.

### `subscribe_send_targets(channel: Channel<TargetsDto>) -> TargetsDto`

Registers the webview's channel for target-list changes and returns the current list, so there is no
moment where the screen is subscribed but empty — the shape `subscribe_catalogue` already uses.

Pushes when a device is attached or removed, and when publishing is switched on or off (the published
source enters and leaves the list). Not batched: this is a human-scale event with nothing to coalesce.

---

## Choosing where traffic goes

### `set_send_target(target_id: u32) -> SendViewDto`

Chooses the target. Persists it by `TargetKey`, so it is recognised after a replug and on the next
launch (FR-023).

| Error | Condition |
|---|---|
| `UnknownTarget` | the id is not in the current list |

### `set_publication_name(name: String) -> SendViewDto`

Sets the name the published source carries. If the source is currently published, it is republished
under the new name — the receiving program sees the old device disappear and a new one appear, which
is what the user is warned about **before** this command is called. The warning is the webview's; the
core applies what it is told.

| Error | Condition |
|---|---|
| `MalformedPublicationName` | empty after trimming, or longer than 64 characters |
| `PublicationUnsupported` | the platform cannot publish (reachable only from a stale webview) |
| `PublicationFailed` | republishing under the new name did not succeed |

### `set_publication_enabled(published: bool) -> SendViewDto`

Starts or stops publishing. Stopping removes the source from other programs' lists and from the target
list; if it was the chosen target, the choice is cleared and the screen says so rather than silently
sending somewhere else.

Never affects reception, never affects sending to destinations (FR-026).

| Error | Condition |
|---|---|
| `PublicationUnsupported` | the platform cannot publish — the control is not operable, so only a stale webview reaches this |
| `PublicationFailed` | the platform can publish, but this attempt did not succeed |

---

## Composing

### `set_composition_kind(kind_id: String) -> SendViewDto`

Replaces the composition with the default message for that kind. The returned `fields` are exactly the
values that message carries — no more, no fewer (FR-013).

| Error | Condition |
|---|---|
| `UnknownSendableKind` | the id is not one of the eighteen |

### `set_composition_field(field_id: String, value: u16) -> SendViewDto`

Sets one value. The returned `preview` is the re-encoded byte string, which is what makes FR-015 and
SC-004 the same fact rather than two.

| Error | Condition |
|---|---|
| `ValueOutOfRange { field, min, max }` | outside the range the message permits (FR-016) |
| `UnknownField` | the field is not on the current composition — a stale webview, or a raw composition |

### `compose_raw(entry: String) -> SendViewDto`

Replaces the composition with hand-typed bytes, validated by the existing decoder. Accepted only when
the entry is exactly one valid MIDI message with nothing left pending.

| Error | Condition |
|---|---|
| `MalformedSendBytes { detail }` | not a valid message; `detail` is the decoder's own reason (FR-018) |

---

## Sending

### `send() -> SendViewDto`

Transmits the current composition to the chosen target and records the outcome. Returns the updated
view in **both** cases — a failed send is a recorded send, not a lost one.

Sequence: `sender.outgoing()` produces what to send and where; `AppState` performs the transmission
through the `Transmitter`; `sender.record(...)` stores the result. The shell coordinates and decides
nothing, which is the shape `AppState::sync_ports` already uses.

| Error | Condition |
|---|---|
| `NoSendTarget` | nothing is chosen (US1 scenario 4) |
| `UnknownTarget` | the chosen target has gone since it was chosen |
| `TransmitFailed { target, detail }` | the platform refused; also recorded as a failed send |

**`TransmitFailed` is both returned and recorded.** The return puts the message where the user is
looking; the record puts it where they will look later. Neither alone satisfies FR-028 together with
FR-030.

### `resend(record_id: u32) -> SendViewDto`

Transmits the identical bytes of a recorded send to the currently chosen target, without touching the
composition (FR-031). The bytes come from the record, not from re-encoding, so a resend of hand-typed
bytes sends what was typed.

| Error | Condition |
|---|---|
| `UnknownSendRecord` | the id has been evicted past the 200-record cap |
| `NoSendTarget`, `UnknownTarget`, `TransmitFailed` | as for `send` |

---

## The request library

### `select_request(name: String) -> SendViewDto`

Loads a request — built-in or saved — into the composition. Its values remain adjustable (FR-009).
This plus `send` is the two-interaction path SC-002 requires.

### `save_request(name: String) -> SendViewDto`

Saves the current composition under a name, unique across saved requests **and** built-ins.

| Error | Condition |
|---|---|
| `MalformedRequestName` | empty after trimming, or longer than 64 characters |
| `DuplicateRequestName { name }` | the name is already in the library |

### `rename_request(from: String, to: String) -> SendViewDto`

| Error | Condition |
|---|---|
| `UnknownRequest { name }` | `from` is not in the library |
| `BuiltInRequestImmutable { name }` | `from` is a built-in (FR-011) |
| `DuplicateRequestName { name }` | `to` is already taken |
| `MalformedRequestName` | `to` fails validation |

### `delete_request(name: String) -> SendViewDto`

| Error | Condition |
|---|---|
| `UnknownRequest { name }` | not in the library |
| `BuiltInRequestImmutable { name }` | it is a built-in (FR-011) |

---

## Error mapping

Sixteen new `IpcError` variants, one per `CoreError` added, each with a `From` arm. The webview's
`describeError` gains one arm each, so every failure has user-facing prose written once.

| `CoreError` | `IpcError` | Where the user sees it |
|---|---|---|
| `MalformedSendBytes` | same | at the raw entry field |
| `ValueOutOfRange` | same | at the field being edited |
| `NoSendTarget` | same | at the target picker |
| `UnknownTarget` | same | at the target picker |
| `TransmitFailed` | same | at the send control, and in the record |
| `PublicationUnsupported` | same | at the publish control (normally pre-empted) |
| `PublicationFailed` | same | at the publish control |
| `MalformedPublicationName` | same | at the name field |
| `UnknownRequest` | same | at the request list |
| `DuplicateRequestName` | same | at the save control |
| `BuiltInRequestImmutable` | same | at the request list |
| `MalformedRequestName` | same | at the save control |
| `UnknownField`, `UnknownSendableKind`, `UnknownSendRecord` | same | the shared banner — a stale view, not a user mistake |

**Where errors are shown follows 004's precedent**: a failure the user can tie to the control they
just used is shown at that control; a failure they cannot is shown in the window's shared banner. The
banner stays rare enough to be worth reading.

---

## What is not added

- **No `send_request(name)` one-shot.** `select_request` then `send` is two commands and two clicks,
  which is what SC-002 asks for. A third command that fused them would exist only to save an
  already-cheap round trip, and would need its own copy of the target and outcome handling.
- **No `preview_bytes(composition)`.** The preview is a field of the model that every mutating command
  already returns. A separate query would let the preview and the composition disagree.
- **No polling command for targets.** Changes are pushed.
- **No command to configure the receiving program.** Explicitly out of scope: publishing makes the
  monitor appear as a device, and mapping it is done in that program.
