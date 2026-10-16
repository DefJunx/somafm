import { getStations, onStationsUpdated, refreshStations, type Station, type StationsSnapshot } from "./api";

export function createStationsStore() {
  let stations: Station[] = $state([]);
  let loaded = $state(false);
  let retrying = $state(false);
  let ready = $state(false);
  let started: Promise<void> | null = null;

  const apply = (s: StationsSnapshot) => {
    stations = s.stations;
    loaded = s.loaded;
    ready = true;
  };

  return {
    get stations() { return stations; },
    get loaded() { return loaded; },
    get ready() { return ready; },
    get retrying() { return retrying; },
    /** Subscribe before the initial get to avoid missing an update. Idempotent: Grid remounts on view switch. */
    init() {
      started ??= (async () => {
        const unlisten = await onStationsUpdated(apply);
        try {
          apply(await getStations());
        } catch (e) {
          unlisten();
          started = null; // let a later mount retry
          throw e;
        }
      })();
      return started;
    },
    async retry() {
      retrying = true;
      try {
        apply(await refreshStations());
      } catch {
        // keep current state; the empty state stays visible
      } finally {
        retrying = false;
      }
    },
  };
}

export const stationsStore = createStationsStore();
