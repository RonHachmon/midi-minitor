import { create } from "zustand";
import type { SendViewDto } from "./bindings";

/**
 * Presentation state for the send screen.
 *
 * # Why this holds no rules — and computes no bytes
 *
 * Everything a message *is* — which values it carries, what they may be, and
 * what they encode to — is decided in Rust and arrives in `view`. This store
 * keeps two things only: that model as it was last received, and what the user
 * has typed into a control but not yet committed.
 *
 * The byte preview in particular is never computed here. The promise that what
 * the screen shows is what goes out is only keepable if the two are the *same
 * value*, so the preview is a field of `view` like any other. If a MIDI fact
 * ever appears in this file, it belongs on the other side of the IPC boundary.
 *
 * # Why the screen choice lives here rather than in a route
 *
 * There are two screens and no addresses. A router would bring a dependency, a
 * history stack, and a URL scheme to answer a question a single value answers —
 * which is the ceremony the project's principles reject.
 */

/** Which of the two screens is showing. */
export type Screen = "monitor" | "send";

/** What the send screen renders and remembers. */
export interface SendState {
  /** Which screen the window is showing. */
  screen: Screen;
  /**
   * The core's model of the send screen, or `null` before it has been read.
   *
   * `null` is the honest state on first mount: the screen has asked and not yet
   * been answered, which is different from "there is nothing to send to".
   */
  view: SendViewDto | null;
  /**
   * What the user has typed into a field but not yet committed.
   *
   * Keyed by field identifier. Presentation state in the strictest sense: it
   * exists so a half-typed number does not fight the value coming back from the
   * core on every keystroke, and it is discarded the moment the core answers.
   */
  drafts: Record<string, string>;
  /** The hand-typed byte entry, before it is submitted for validation. */
  rawEntry: string;
  /** The name being typed into the save control. */
  saveName: string;
  /** The name being typed for the published source. */
  publishName: string;
  /**
   * A failure that belongs beside a send control rather than in the window's
   * banner.
   *
   * The same split the Filter panel already uses: a failure the user can tie to
   * the control they just used is shown at that control.
   */
  message: string | null;

  /** Shows one of the two screens. */
  setScreen: (screen: Screen) => void;
  /** Replaces the model with what the core just returned. */
  applyView: (view: SendViewDto) => void;
  /** Replaces the target list alone, as pushed when devices come and go. */
  applyTargets: (targets: SendViewDto["targets"]) => void;
  /** Records a keystroke in one field, without sending anything yet. */
  setDraft: (field: string, value: string) => void;
  /** Forgets a field's uncommitted text. */
  clearDraft: (field: string) => void;
  /** Records the hand-typed byte entry. */
  setRawEntry: (entry: string) => void;
  /** Records the name being typed into the save control. */
  setSaveName: (name: string) => void;
  /** Records the name being typed for the published source. */
  setPublishName: (name: string) => void;
  /** Shows or clears the message beside the controls. */
  setMessage: (message: string | null) => void;
}

export const useSendStore = create<SendState>((set) => ({
  screen: "monitor",
  view: null,
  drafts: {},
  rawEntry: "",
  saveName: "",
  publishName: "",
  message: null,

  setScreen: (screen) => set({ screen }),

  applyView: (view) =>
    set((state) => ({
      view,
      // Drafts are dropped wholesale: the core has just answered, so every field
      // now shows a committed value and any half-typed text is stale by
      // definition.
      drafts: {},
      // The published name follows the core unless the user is mid-edit on this
      // very field, which `applyView` cannot be reached from.
      publishName: view.publication.name,
      message: state.message,
    })),

  applyTargets: (targets) =>
    set((state) => (state.view === null ? {} : { view: { ...state.view, targets } })),

  setDraft: (field, value) =>
    set((state) => ({ drafts: { ...state.drafts, [field]: value } })),

  clearDraft: (field) =>
    set((state) => {
      const drafts = { ...state.drafts };
      delete drafts[field];
      return { drafts };
    }),

  setRawEntry: (rawEntry) => set({ rawEntry }),
  setSaveName: (saveName) => set({ saveName }),
  setPublishName: (publishName) => set({ publishName }),
  setMessage: (message) => set({ message }),
}));
