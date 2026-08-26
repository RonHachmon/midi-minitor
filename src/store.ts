import { create } from "zustand";
import type {
  ColumnDto,
  EventDto,
  FilterViewDto,
  SnapshotDto,
  SourceGroupDto,
} from "./bindings";

/**
 * Presentation state for the monitor window.
 *
 * # Why this holds no rules
 *
 * Whether an event is visible, which sources are selected, and what a filter
 * means are all decided in Rust. This store keeps only what the components need
 * to render and what the user has typed but not yet committed. If a predicate
 * over MIDI data ever appears in this file, it belongs on the other side of the
 * IPC boundary.
 */
export interface MonitorState {
  /** Events currently displayed, oldest first. */
  events: EventDto[];
  /** How many events are retained before filtering. */
  retainedCount: number;
  /** The retention cap the core is enforcing. */
  retentionLimit: number;
  /** False when no source is selected — an explained empty list, not a stalled one. */
  monitoring: boolean;
  /**
   * Newest event id included in the last snapshot.
   *
   * Batches at or below this mark were admitted under the previous settings and
   * are discarded, so a message in flight during a filter change cannot
   * resurrect an event the user just filtered out.
   */
  highWaterMark: number | null;
  /** The Sources panel's structure. */
  groups: SourceGroupDto[];
  /** The Filter panel's structure and state. */
  filter: FilterViewDto | null;
  /** Every column and whether it is shown. */
  columns: ColumnDto[];
  /** The most recent failure to explain, or null. */
  error: string | null;

  /** Replaces the visible list wholesale from a snapshot. */
  applySnapshot: (snapshot: SnapshotDto) => void;
  /** Appends a streamed batch, dropping stale events and trimming to the cap. */
  appendBatch: (events: EventDto[]) => void;
  /** Stores the Sources panel structure. */
  setGroups: (groups: SourceGroupDto[]) => void;
  /** Stores the Filter panel structure. */
  setFilter: (filter: FilterViewDto) => void;
  /** Stores the column list. */
  setColumns: (columns: ColumnDto[]) => void;
  /** Records or clears the message shown to the user. */
  setError: (error: string | null) => void;
}

export const useMonitorStore = create<MonitorState>((set) => ({
  events: [],
  retainedCount: 0,
  retentionLimit: 1000,
  monitoring: true,
  highWaterMark: null,
  groups: [],
  filter: null,
  columns: [],
  error: null,

  applySnapshot: (snapshot) =>
    set({
      events: snapshot.events,
      retainedCount: snapshot.retainedCount,
      retentionLimit: snapshot.retentionLimit,
      monitoring: snapshot.monitoring,
      highWaterMark: snapshot.highWaterMark,
    }),

  appendBatch: (incoming) =>
    set((state) => {
      const mark = state.highWaterMark;
      const fresh =
        mark === null ? incoming : incoming.filter((event) => event.id > mark);
      if (fresh.length === 0) {
        return state;
      }

      const combined = state.events.concat(fresh);
      // The core is the authority on retention; trimming here only keeps the
      // rendered list from outrunning it between snapshots.
      const overflow = combined.length - state.retentionLimit;
      const events = overflow > 0 ? combined.slice(overflow) : combined;
      const newest = fresh[fresh.length - 1];

      return {
        ...state,
        events,
        highWaterMark: newest === undefined ? mark : newest.id,
        retainedCount: Math.min(
          state.retainedCount + fresh.length,
          state.retentionLimit,
        ),
      };
    }),

  setGroups: (groups) => set({ groups }),
  setFilter: (filter) => set({ filter }),
  setColumns: (columns) => set({ columns }),
  setError: (error) => set({ error }),
}));
