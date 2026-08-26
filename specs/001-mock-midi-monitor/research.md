# Phase 0 Research: Mock MIDI Monitor

**Date**: 2026-08-26 · **Plan**: [plan.md](./plan.md)

All findings below were checked against current documentation via Context7 rather than recalled,
because Tauri 2, Tailwind 4, and `tauri-specta` 2 have all changed shape recently. Sources are named
per finding. Entries marked **[verified on this machine]** were checked by running a command here.

## Toolchain status

**[verified on this machine]** `node v24.18.0` and `npm 11.16.0` are present.
**`cargo`, `rustc`, and `rustup` are NOT installed** — checked on both the Git Bash and PowerShell
paths, and `%USERPROFILE%\.cargo\bin` does not exist.

- **Decision**: Installing the Rust toolchain is task zero of implementation, and a prerequisite in
  [quickstart.md](./quickstart.md). Nothing else can proceed: `cargo build` is the project's
  definition of verification.
- **Rationale**: Better to surface this now than to have `/speckit-implement` fail on its first
  build command.
- **Alternatives considered**: None — Tauri's backend is Rust; there is no path around it.
- **Platform note**: On Windows, Tauri 2 also needs Microsoft Edge WebView2 (present by default on
  Windows 11) and the MSVC C++ build tools that `rustup` prompts for.

## D-01: Rust ↔ webview transport for a 500 event/second stream

**Decision**: Stream events through a **`tauri::ipc::Channel<EventBatch>`**, opened once by the
webview and fed by a Rust pump that **coalesces events into batches at ~60 Hz**. Use ordinary
`#[tauri::command]` calls for control operations (settings changes, snapshot, clear).

**Rationale**: The Tauri v2 documentation is explicit about the limits of the event system:

> "The event system was designed for situations where small amounts of data need to be streamed…
> **The event system is not designed for low latency or high throughput situations.** See the
> channels section for the implementation optimized for streaming data. …events have **no strong
> type support**, event payloads are always JSON strings."
> — `tauri-docs`, `develop/calling-frontend.mdx`

Both halves of that quote disqualify `emit`/`listen` here. Throughput: SC-005 requires 500 events/s
sustained. Typing: Constitution IV forbids untyped data crossing IPC, and JSON-string payloads with
no type support are exactly that. Channels are the documented streaming mechanism and, per D-02,
`tauri-specta` renders them as a typed `Channel<T>` in the generated bindings — so this choice
satisfies the throughput requirement and the typing principle at once.

Batching is a separate and equally important decision: 500 discrete IPC messages per second means
500 serialization round-trips and 500 React state updates per second. One batch per animation frame
reduces that to ~60 messages/s carrying ~8 events each, which is both less IPC work and a natural
fit for the browser's paint cadence.

**Alternatives considered**:
- *`app.emit` + `listen`* — rejected on both documented counts above.
- *Frontend polling with a `get_events_since(cursor)` command* — workable and fully typed, but it
  trades push for a timer, adds cursor bookkeeping to the webview, and would put "what is new"
  logic on the wrong side of the boundary (Principle IV).
- *One channel message per event* — rejected; correct but wasteful, and it makes the React list the
  bottleneck rather than the data source.

## D-02: Keeping the IPC contract typed and generated

**Decision**: Use **`tauri-specta` 2.0.0-rc** with `specta` to generate `src/bindings.ts` from the
Rust command and type declarations at debug-build time. Wire types are annotated
`#[derive(Serialize, specta::Type)]`.

**Rationale**: Constitution IV requires the contract be "declared in Rust and their TypeScript types
generated from those declarations", with hand-maintained parallel definitions forbidden. The
documented builder does exactly this, including error typing and `Channel<T>`:

```text
let mut builder = Builder::new()
    .commands(collect_commands![...])
    .error_handling(ErrorHandlingMode::Result)
    .function_casing(Casing::CamelCase);

#[cfg(debug_assertions)]
builder.export(Typescript::default(), "../src/bindings.ts")?;
```
— `specta-rs/tauri-specta`, `_autodocs/configuration.md`

Two details confirmed from the crate source that matter to this design:
- `Channel<T>` is recognised and rendered as a typed `Channel<T>` imported from
  `@tauri-apps/api/core` — so the stream is covered by generation, not exempt from it.
- `ErrorHandlingMode::Result` emits Rust error enums as TypeScript discriminated unions
  (`{ type: "IoError" } | { type: "AnotherError"; data: string }`), which is precisely the
  "errors cross as typed variants, never as strings" rule in Constitution IV.

**Risk**: `tauri-specta` 2.x is at release-candidate status (`2.0.0-rc.21` at time of writing). This
is accepted deliberately: the alternative is worse against the constitution, and an RC used only at
build time cannot break a shipped binary.

**Alternatives considered**:
- *Hand-written `types.ts` mirroring the Rust structs* — flatly forbidden by Constitution IV, and
  the failure mode (silent drift with no test to catch it) is the exact risk the principle exists to
  prevent.
- *`ts-rs`* — stable and generates types, but not command signatures or channel types, leaving the
  invoke wrappers hand-maintained. Recorded as the fallback if `tauri-specta` proves unworkable;
  choosing it would mean hand-writing thin wrappers and documenting the deviation.

## D-03: Where filtering happens

**Decision**: **All** filtering runs in `midi-core`. The webview receives events that already passed
every filter and renders them without deciding anything. On any settings change the webview calls
one command and replaces its list with the returned snapshot.

**Rationale**: Constitution IV states the webview "MUST NOT contain business rules". Whether a
`0x90` byte with `One Channel = 5` selected should appear is a business rule. Keeping it in Rust
also makes FR-034 (already-listed events re-evaluate when filters change) trivially consistent —
there is one filter implementation, not two that must agree. It is additionally the cheaper design:
suppressed high-rate traffic like Clock never crosses the IPC boundary at all.

**Alternatives considered**:
- *Send everything, filter in React* — rejected on Principle IV, and it would push the full
  unfiltered rate across IPC precisely when the user is trying to reduce it.
- *Filter in both places for responsiveness* — rejected: two implementations of one rule with no
  tests to keep them honest.

## D-04: Retention semantics

**Decision**: The bounded `EventLog` in the core retains events from **selected sources**, capped by
the retention limit and evicting oldest-first. Message-type, channel, and hex-prefix filters are
applied as a **view over** that log, not as an admission gate into it.

**Rationale**: This is the only reading that satisfies FR-026 (deselecting a source removes its
already-listed events) and FR-034 (filter changes re-evaluate listed events) simultaneously. Source
selection is *what is being monitored*; the filters are *what is being looked at*. Consequently,
toggling `Clock` back on immediately re-reveals retained Clock events rather than waiting for new
ones — the behaviour a user expects from a monitor.

**Range**: retention accepts `1..=100_000`, default `1000` (the screenshot value). The upper bound
satisfies FR-019's "stated supported range"; at ~64 bytes per event, 100 000 events is a few MB and
comfortably virtualized.

**Alternatives considered**:
- *Cap the filtered view instead of the log* — rejected: the visible count would jump around as
  filters change, and "Remember up to N events" describes memory, not the viewport.

## D-05: MIDI message model — the one deliberate hand-roll

**Decision**: Define the message model as a domain enum in `midi-core` rather than depending on a
MIDI crate such as `midi-msg`, `wmidi`, or `midir`.

**Rationale**: Constitution II requires that hand-rolling be justified and the justification recorded
in module docs. It is justified here: those crates model *the MIDI protocol* for real I/O — `midir`
is device access this app explicitly does not perform, and `midi-msg`/`wmidi` model parsing and
encoding fidelity. What this application needs is the **Filter panel's vocabulary**: the exact 14
categories shown in `screenshots/filters.png`, each mapped to a display name for the Message column
and a rendering rule for the Data column, plus an `Invalid` case that no correct MIDI library would
willingly produce. Adapting a protocol crate to that shape would mean writing the same mapping table
anyway, on top of a dependency. The domain enum *is* the domain.

**Alternatives considered**: `midi-msg` (closest fit; rejected as above), `wmidi` (no-alloc parsing,
solving a problem this app does not have), `midir` (real device I/O — out of scope by FR-004).

**Recorded obligation**: this justification is repeated verbatim in the `domain/message.rs` module
documentation, as Principle II requires.

## D-06: Frontend stack — small by construction

**Decision**: React 19 + Vite + **Tailwind CSS v4** + `@tanstack/react-virtual` + `zustand`.

**Tailwind v4 setup** (v4 dropped the PostCSS/`tailwind.config.js` ceremony):

```text
npm install tailwindcss @tailwindcss/vite
// vite.config.ts
import tailwindcss from "@tailwindcss/vite";
export default defineConfig({ plugins: [tailwindcss()] });
/* index.css */
@import "tailwindcss";
```
— `tailwindlabs/tailwindcss.com`, upgrade guide and v4 blog

There is no `tailwind.config.js`; design tokens are declared in CSS via `@theme`. That suits
Constitution VI well — the handful of values taken from the screenshots (row height, column widths,
header tint, list border) become named CSS tokens in one file rather than being scattered as
arbitrary utility values.

**Virtualization**: `useVirtualizer` with a fixed `estimateSize` and a scroll-container ref renders
only visible rows, keeping a 100 000-event log at 60 fps.

**State**: `zustand` — a few hundred bytes, no provider tree, and it satisfies Principle II's
"reuse rather than hand-roll" for a store. Rejected: Redux/RTK (ceremony at this size, Principle V),
and a hand-written context+reducer (a solved problem, Principle II).

**Rationale for "simple and small"**: no router (one window), no component library (Constitution VI
requires reproducing macOS-style controls exactly — a component library's opinions would fight the
screenshots), no data-fetching library, no animation library.

**Vite configuration for Tauri** (from `tauri-docs`, `start/frontend/vite.mdx`): `clearScreen: false`
so Rust errors stay visible, `strictPort: true` on 5173 matching `devUrl`, and
`watch.ignored: ['**/src-tauri/**']` so Vite does not thrash on Rust rebuilds.

## D-07: Settings persistence

**Decision**: `tauri-plugin-store`, written from Rust via `app.store("settings.json")`, wrapped
behind a `SettingsRepository` port implemented in `src-tauri/settings.rs`.

**Rationale**: Principle II — the plugin already solves atomic write, path resolution to the OS
config directory, and debounced auto-save (it defaults to a 100 ms auto-save window). The Repository
wrapper is named as a pattern under Principle V because it keeps the plugin — and `serde_json::Value`
— out of `midi-core`, which must stay Tauri-free.

**Alternatives considered**: hand-rolled `serde_json` file I/O (re-solves path resolution and atomic
writes — Principle II), `confy` (fine, but adds a dependency where the Tauri plugin is already in
the dependency graph and integrates with the app's path resolver).

## D-08: Design fidelity mechanics

**Decision**: Freeze the screenshot-derived facts as named constants in one place per side —
`crates/midi-core/src/constants.rs` for behavioural defaults, CSS `@theme` tokens for visual ones —
and gate every UI change on a side-by-side comparison recorded in
[quickstart.md](./quickstart.md).

**Facts extracted from the reference images** (normative under Constitution VI):
- Column order and headers: `Time`, `Source`, `Message`, `Chan`, `Data`.
- Retention row reads `Remember up to` · field · `events`, with `Clear` at the opposite edge.
- Both disclosure sections start collapsed with a `▶` triangle; expanded shows `▼`.
- Filter defaults: every checkbox checked, `All Channels` selected, channel field inactive showing `1`.
- Sources tree: `MIDI sources` (children `IAC Driver Bus 1`, `MidiKeys`), `Act as a destination for
  other programs` (no children), `Spy on output to destinations` (child `IAC Driver Bus 1`).
- Timestamps ascend downward — newest at the bottom.
- Channel Pressure rows show a single Data value; note rows show note name then velocity (`C2 127`).

**Rationale**: Constitution VI makes these normative rather than advisory, and a value that lives in
a named constant can be checked against the image once instead of being re-guessed at each use.

## D-09: Column visibility — an additive surface

**Decision**: Column visibility is controlled by a small menu affordance on the table header, a
**new** surface that the screenshots do not depict.

**Rationale**: Constitution VII requires additive features to earn new surface and forbids altering
any control the screenshots show. Column visibility is not in the reference images, so it must not
be bolted onto the Filter panel or change the header's appearance at rest. A header context menu
adds capability without disturbing the frozen surface. At least one column always remains visible
(FR-045), enforced in the core so the rule cannot be bypassed from the UI.

## Resolved unknowns

Every `NEEDS CLARIFICATION` from Technical Context is resolved above: transport (D-01), type
generation (D-02), filter placement (D-03), retention semantics and range (D-04), MIDI modelling
(D-05), frontend stack and versions (D-06), persistence (D-07). No unknowns remain open for
Phase 1.

One item is outstanding but is an **environment prerequisite, not a design unknown**: the Rust
toolchain must be installed before implementation begins.
