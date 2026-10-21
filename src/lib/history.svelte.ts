import { getStationHistory, type HistoryEntry } from "./api";

/** Station history of the Active station: in-memory, refetched by the Player screen, never polled. */
export function createHistoryStore() {
  let entries: HistoryEntry[] = $state([]);
  let failed = $state(false);
  let stationId: string | null = null;
  let seq = 0;

  return {
    get entries() { return entries; },
    get failed() { return failed; },
    /** A Station switch drops the old list at once; only the latest request may apply its result. */
    async load(id: string) {
      if (id !== stationId) {
        stationId = id;
        entries = [];
        failed = false;
      }
      const mine = ++seq;
      try {
        const result = await getStationHistory(id);
        if (mine !== seq) return;
        entries = result;
        failed = false;
      } catch {
        if (mine !== seq) return;
        entries = [];
        failed = true;
      }
    },
  };
}

export const historyStore = createHistoryStore();
