import { getStationHistory, type HistoryEntry } from "./api";

/** Station history of the Active station: in-memory, refetched by the Player screen, never polled. */
export function createHistoryStore() {
  let entries: HistoryEntry[] = $state([]);
  let failed = $state(false);
  let stationId: string | null = null;
  let seq = 0;

  const load = async (id: string) => {
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
  };

  return {
    get entries() { return entries; },
    get failed() { return failed; },
    /** Each mounted Player gets fresh refresh eligibility; requests still order through this store. */
    createSession() {
      let requestedStation: string | null = null;
      let requestedRaw: string | null = null;

      return {
        update(visible: boolean, id: string | null, raw: string | null) {
          if (!visible || id === null) return;
          if (id === requestedStation && raw === null) return;
          if (id === requestedStation && raw === requestedRaw) return;

          requestedStation = id;
          requestedRaw = raw;
          void load(id);
        },
      };
    },
  };
}

export const historyStore = createHistoryStore();
