import { Channel } from "@tauri-apps/api/core";
import { commands } from "./bindings";
import type { CatalogueDto, EventBatchDto, EventDto, IpcError } from "./bindings";
import { useMonitorStore } from "./store";

/**
 * The webview's side of the IPC contract.
 *
 * # Why every call goes through this module
 *
 * The generated bindings return a tagged result rather than throwing, which is
 * what keeps errors typed. Unwrapping that in each component would spread the
 * same six lines everywhere and invite one of them to quietly ignore a failure.
 * Here it happens once, and a failure always lands somewhere the user can see.
 */

/**
 * Turns a typed error into a sentence for the user.
 *
 * Exhaustive over the union with no default arm: a new error variant in Rust
 * becomes a TypeScript compile error here, so it cannot reach a user as a blank
 * message. That is the whole reason errors cross as variants and not strings.
 */
export function describeError(error: IpcError): string {
  switch (error.type) {
    case "channelOutOfRange":
      return `Channel ${error.data.value} is outside 1–16.`;
    case "retentionOutOfRange":
      return `${error.data.value} is outside the supported range 1–100000.`;
    case "malformedHexPrefix":
      return `"${error.data.entry}" is not hexadecimal. The previous filter is still in effect.`;
    case "emptyHexPrefix":
      return "A hexadecimal prefix cannot be empty.";
    case "lastColumnVisible":
      return "At least one column must stay visible.";
    case "unknownSource":
      return "That source is no longer available.";
    case "unknownGroup":
      return "That source group is no longer available.";
    case "settingsUnavailable":
      return `Settings could not be saved: ${error.data.detail}`;
    case "monitorUnavailable":
      return "The monitor stopped responding. Restart the application.";
  }
}

/**
 * Whether a rejected value is one of our own typed errors.
 *
 * A failure raised by Tauri itself — an unresolvable command, unregistered
 * state — is not an `IpcError` and has no `type` discriminant. Without this
 * check `describeError` falls off its switch and returns nothing, which reaches
 * the user as an empty alert: a visible banner saying nothing at all. Checking
 * the shape lets a framework failure be reported as what it is.
 */
function isIpcError(value: unknown): value is IpcError {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as { type?: unknown }).type === "string"
  );
}

/**
 * Runs a command and routes its outcome into the store.
 *
 * Returns the value on success and `null` on failure, having already recorded
 * the message — so callers may ignore the return without losing the error.
 */
async function run<T>(
  call: Promise<{ status: "ok"; data: T } | { status: "error"; error: IpcError }>,
): Promise<T | null> {
  const { setError } = useMonitorStore.getState();
  let result;
  try {
    result = await call;
  } catch (thrown) {
    // The generated wrapper rethrows genuine exceptions rather than tagging
    // them, so this is the only place a transport failure can surface.
    setError(`The application could not be reached: ${String(thrown)}`);
    return null;
  }

  if (result.status === "error") {
    setError(
      isIpcError(result.error)
        ? describeError(result.error)
        : `Unexpected failure: ${String(result.error)}`,
    );
    return null;
  }
  setError(null);
  return result.data;
}

/**
 * Opens the event stream and loads the initial state.
 *
 * # Why incoming batches are buffered rather than applied directly
 *
 * The core sends roughly sixty batches a second. Calling into the store on each
 * one would schedule sixty React renders a second regardless of whether the
 * browser could paint them. Buffering into a plain array and flushing on
 * `requestAnimationFrame` collapses that to one render per frame — and to none
 * at all when the window is hidden, since the callback stops firing.
 *
 * Returns a teardown function.
 */
export function startStream(): () => void {
  let buffer: EventDto[] = [];
  let frame: number | null = null;
  let stopped = false;

  const flush = () => {
    frame = null;
    if (stopped || buffer.length === 0) {
      return;
    }
    const batch = buffer;
    buffer = [];
    useMonitorStore.getState().appendBatch(batch);
  };

  const channel = new Channel<EventBatchDto>();
  channel.onmessage = (message) => {
    if (stopped) {
      return;
    }
    buffer = buffer.concat(message.events);
    if (frame === null) {
      frame = requestAnimationFrame(flush);
    }
  };

  void (async () => {
    const snapshot = await run(commands.subscribeEvents(channel));
    if (snapshot !== null && !stopped) {
      // Any batch buffered while the subscription was in flight predates the
      // snapshot, so it is dropped rather than appended twice.
      buffer = [];
      useMonitorStore.getState().applySnapshot(snapshot);
    }
    await refreshPanels();
  })();

  return () => {
    stopped = true;
    if (frame !== null) {
      cancelAnimationFrame(frame);
    }
  };
}

/** Reloads the Sources, Filter, and column structures from Rust. */
export async function refreshPanels(): Promise<void> {
  const store = useMonitorStore.getState();
  const [catalogue, filter, columns] = await Promise.all([
    run(commands.getCatalogue()),
    run(commands.getFilterModel()),
    run(commands.getColumns()),
  ]);
  if (catalogue) store.applyCatalogue(catalogue);
  if (filter) store.setFilter(filter);
  if (columns) store.setColumns(columns);
}

/**
 * Subscribes to catalogue changes so the Sources list follows the hardware.
 *
 * # Why this is a separate channel from the event stream
 *
 * The two carry unrelated payloads at unrelated rates: events arrive hundreds
 * per second and are batched on a frame timer, while a catalogue change happens
 * when someone physically touches a cable. Sharing one channel would make the
 * batching interval the floor for how quickly the Sources list could react.
 */
export async function subscribeCatalogue(): Promise<void> {
  const channel = new Channel<CatalogueDto>();
  channel.onmessage = (catalogue) => {
    useMonitorStore.getState().applyCatalogue(catalogue);
  };
  const initial = await run(commands.subscribeCatalogue(channel));
  if (initial) {
    useMonitorStore.getState().applyCatalogue(initial);
  }
}

/** Applies a snapshot-returning command and refreshes the panels it may change. */
async function mutate(
  call: ReturnType<typeof commands.snapshot>,
  refresh: boolean,
): Promise<void> {
  const snapshot = await run(call);
  if (snapshot) {
    useMonitorStore.getState().applySnapshot(snapshot);
  }
  if (refresh) {
    await refreshPanels();
  }
}

/**
 * Applies a command that returns both a snapshot and a catalogue.
 *
 * Selecting a source can reveal that its port will not open, which the snapshot
 * has no field for — so these two commands carry the catalogue back with them
 * rather than requiring a second round trip to discover it.
 */
async function mutateWithCatalogue(
  call: ReturnType<typeof commands.setSourceSelected>,
): Promise<void> {
  const result = await run(call);
  if (result) {
    const store = useMonitorStore.getState();
    store.applySnapshot(result.snapshot);
    store.applyCatalogue(result.catalogue);
  }
}

/** Selects or deselects one source. */
export const setSourceSelected = (id: number, selected: boolean) =>
  mutateWithCatalogue(commands.setSourceSelected(id, selected));

/** Applies one selection state to every source in a group. */
export const setGroupSelected = (groupId: string, selected: boolean) =>
  mutateWithCatalogue(commands.setGroupSelected(groupId, selected));

/** Replaces the message-kind and channel filter. */
export const setFilter = (
  kinds: string[],
  channelMode: Parameters<typeof commands.setFilter>[1],
) => mutate(commands.setFilter(kinds, channelMode), true);

/** Replaces the hexadecimal prefix filter. */
export const setDataPrefixFilter = (
  mode: Parameters<typeof commands.setDataPrefixFilter>[0],
  prefixes: string[],
) => mutate(commands.setDataPrefixFilter(mode, prefixes), true);

/** Replaces the retention cap. */
export const setRetentionLimit = (limit: number) =>
  mutate(commands.setRetentionLimit(limit), false);

/** Discards every retained event. */
export const clearEvents = () => mutate(commands.clearEvents(), false);

/** Replaces which columns are shown. */
export async function setColumnVisibility(visible: string[]): Promise<void> {
  const columns = await run(commands.setColumnVisibility(visible));
  if (columns) {
    useMonitorStore.getState().setColumns(columns);
  }
}
