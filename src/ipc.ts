import { Channel } from "@tauri-apps/api/core";
import { commands } from "./bindings";
import type {
  CatalogueDto,
  EventBatchDto,
  EventDto,
  IpcError,
  PrefixModeDto,
  TargetsDto,
} from "./bindings";
import { useMonitorStore } from "./store";
import { useSendStore } from "./sendStore";

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
 * Names a rule kind the way the control that creates it does.
 *
 * The words are the interface's own — `Show only` and `Hide` — so an error about
 * a rule reads in the same vocabulary as the rule itself. Exhaustive with no
 * default arm, for the same reason as `describeError` below.
 */
function describePrefixMode(mode: PrefixModeDto): string {
  switch (mode) {
    case "include":
      return "Show only";
    case "exclude":
      return "Hide";
  }
}

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
    case "duplicatePrefixRule":
      return `A rule for "${error.data.prefix}" is already in the list (${describePrefixMode(error.data.existingKind)}). Delete it first, or enter a different prefix.`;
    case "contradictoryPrefixRule":
      return `"${describePrefixMode(error.data.kind)} ${error.data.prefix}" clashes with "${describePrefixMode(error.data.existingKind)} ${error.data.existingPrefix}" — the prefixes overlap and the two rules have opposite effects. Narrow one of them, or delete the existing rule.`;
    case "unknownPrefixRule":
      return `There is no rule for "${error.data.prefix}" — the list has changed since it was shown.`;
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
    case "unsendableMessage":
      return `That message cannot be transmitted: ${error.data.reason}.`;
    case "malformedSendBytes":
      return `Those bytes are not a single valid MIDI message — ${error.data.detail}.`;
    case "valueOutOfRange":
      return `${error.data.field} must be between ${error.data.min} and ${error.data.max}.`;
    case "noSendTarget":
      return "Choose where to send before sending.";
    case "unknownTarget":
      return error.data.name === ""
        ? "That target is no longer available."
        : `${error.data.name} is no longer available to send to.`;
    case "transmitFailed":
      return `Could not send to ${error.data.target}: ${error.data.detail}`;
    case "publicationUnsupported":
      return error.data.detail;
    case "publicationFailed":
      return `The source could not be published: ${error.data.detail}`;
    case "malformedPublicationName":
      return `A name for the published source must be 1 to ${error.data.max} characters.`;
    case "unknownRequest":
      return `There is no request called "${error.data.name}" — the list has changed since it was shown.`;
    case "duplicateRequestName":
      return `A request called "${error.data.name}" already exists. Choose a different name.`;
    case "builtInRequestImmutable":
      return `"${error.data.name}" is a built-in request and cannot be renamed or deleted.`;
    case "malformedRequestName":
      return `A request name must be 1 to ${error.data.max} characters.`;
    case "unknownField":
      return "That value is not part of the message being composed.";
    case "unknownSendableKind":
      return "That message type is not one this version can compose.";
    case "unknownSendRecord":
      return "That send is no longer in the record.";
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
 * Runs a command and hands its failure back instead of putting it in the banner.
 *
 * # Why some failures take this path
 *
 * The banner sits between the controls and the event table, which is the right
 * place for something the user cannot attribute — settings that would not save, a
 * source that has gone. It is the wrong place for a rule the user just typed:
 * they are looking at the field, not at a strip further down the window, and the
 * message has to be beside the thing that needs correcting.
 *
 * Describing the error still happens here, through the same exhaustive
 * `describeError`, so a caller receives a finished sentence and never inspects a
 * variant to decide what to say.
 */
async function runReporting<T>(
  call: Promise<{ status: "ok"; data: T } | { status: "error"; error: IpcError }>,
): Promise<{ data: T; message: null } | { data: null; message: string }> {
  let result;
  try {
    result = await call;
  } catch (thrown) {
    return {
      data: null,
      message: `The application could not be reached: ${String(thrown)}`,
    };
  }

  if (result.status === "error") {
    return {
      data: null,
      message: isIpcError(result.error)
        ? describeError(result.error)
        : `Unexpected failure: ${String(result.error)}`,
    };
  }
  return { data: result.data, message: null };
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

/**
 * Applies a rule change, returning its failure rather than banner-ing it.
 *
 * Refreshes the panels on success, because the rule list itself is part of the
 * Filter model: the snapshot says which events survive, and only
 * `getFilterModel` says which rules are in force. On failure nothing is applied,
 * which matches what the core did — it refused before mutating.
 *
 * Returns `null` when the change went through, or the sentence to show beside
 * the control when it did not.
 */
async function mutateRules(
  call: ReturnType<typeof commands.snapshot>,
): Promise<string | null> {
  const result = await runReporting(call);
  if (result.message !== null) {
    return result.message;
  }
  useMonitorStore.getState().applySnapshot(result.data);
  await refreshPanels();
  return null;
}

/** Adds one data prefix rule. Resolves to the failure message, or `null`. */
export const addDataPrefixRule = (
  prefix: string,
  kind: Parameters<typeof commands.addDataPrefixRule>[1],
) => mutateRules(commands.addDataPrefixRule(prefix, kind));

/**
 * Removes the data prefix rule carrying this prefix.
 *
 * Resolves to the failure message, or `null`. Deleting can only fail when the
 * list on screen is behind the core's, which is exactly the case where the
 * message belongs next to the list rather than in the window's banner.
 */
export const removeDataPrefixRule = (prefix: string) =>
  mutateRules(commands.removeDataPrefixRule(prefix));

/**
 * Starts or stops taking in what arrives.
 *
 * Refreshes no panel: pausing changes no filter, no column, and no source, so
 * rebuilding those structures would imply a coupling that does not exist.
 */
export const setCaptureState = (
  capture: Parameters<typeof commands.setCaptureState>[0],
) => mutate(commands.setCaptureState(capture), false);

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

/**
 * Applies a send command, routing its failure beside the controls.
 *
 * # Why these do not use the window's banner
 *
 * The same split the rule list already makes: a failure the user can tie to the
 * control they just used belongs at that control, and the banner is for failures
 * they cannot attribute. Everything on the send screen is attributable — they
 * pressed send, or typed a name, or chose a target — so all of it lands here.
 *
 * Returns the failure message, or `null` when the command went through. The view
 * is applied in **both** cases where one came back: a failed send is a recorded
 * send, and the screen has to show the record that the message is about.
 */
async function mutateSend(
  call: ReturnType<typeof commands.getSendView>,
): Promise<string | null> {
  const result = await runReporting(call);
  const store = useSendStore.getState();
  if (result.message !== null) {
    store.setMessage(result.message);
    // A failed send still changed the record, so the model is re-read rather
    // than left showing the state from before the attempt.
    const refreshed = await runReporting(commands.getSendView());
    if (refreshed.data !== null) {
      store.applyView(refreshed.data);
    }
    return result.message;
  }
  store.applyView(result.data);
  store.setMessage(null);
  return null;
}

/** Loads the send screen's model. */
export async function loadSendView(): Promise<void> {
  const result = await runReporting(commands.getSendView());
  const store = useSendStore.getState();
  if (result.data !== null) {
    store.applyView(result.data);
  } else {
    store.setMessage(result.message);
  }
}

/**
 * Subscribes to target-list changes so the picker follows the hardware.
 *
 * A separate channel from the catalogue's for the reason that one is separate
 * from the event stream: unrelated payloads, and no reason for one to wait on
 * the other.
 */
export async function subscribeSendTargets(): Promise<void> {
  const channel = new Channel<TargetsDto>();
  channel.onmessage = (targets) => {
    useSendStore.getState().applyTargets(targets);
  };
  const initial = await run(commands.subscribeSendTargets(channel));
  if (initial) {
    useSendStore.getState().applyTargets(initial);
  }
}

/** Chooses where traffic goes. */
export const setSendTarget = (targetId: number) =>
  mutateSend(commands.setSendTarget(targetId));

/** Loads a request into the composer. */
export const selectRequest = (name: string) =>
  mutateSend(commands.selectRequest(name));

/** Replaces the message being composed with a different type. */
export const setCompositionKind = (kindId: string) =>
  mutateSend(commands.setCompositionKind(kindId));

/** Sets one value on the message being composed. */
export const setCompositionField = (fieldId: string, value: string) =>
  mutateSend(commands.setCompositionField(fieldId, value));

/** Replaces the composition with hand-typed bytes. */
export const composeRaw = (entry: string) =>
  mutateSend(commands.composeRaw(entry));

/** Transmits the current composition to the chosen target. */
export const send = () => mutateSend(commands.send());

/** Re-sends the exact bytes of an earlier send. */
export const resend = (recordId: number) =>
  mutateSend(commands.resend(recordId));

/** Sets the name the published source carries. */
export const setPublicationName = (name: string) =>
  mutateSend(commands.setPublicationName(name));

/** Starts or stops publishing the source. */
export const setPublicationEnabled = (published: boolean) =>
  mutateSend(commands.setPublicationEnabled(published));

/** Saves the current composition under a name. */
export const saveRequest = (name: string) =>
  mutateSend(commands.saveRequest(name));

/** Renames one of the user's own requests. */
export const renameRequest = (from: string, to: string) =>
  mutateSend(commands.renameRequest(from, to));

/** Deletes one of the user's own requests. */
export const deleteRequest = (name: string) =>
  mutateSend(commands.deleteRequest(name));
