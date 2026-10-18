import { getSettings, onSettingsChanged, setFavouritesOnly, toggleFavourite, type SettingsView } from "./api";

/** Pure view of Rust's settings: replaced wholesale on every `settings-changed` event. */
export function createSettingsStore() {
  let favourites: string[] = $state([]);
  let favouritesOnly = $state(false);
  let started: Promise<void> | null = null;

  const apply = (s: SettingsView | undefined) => {
    if (!s) return;
    favourites = s.favourites;
    favouritesOnly = s.favouritesOnly;
  };

  return {
    get favourites() { return favourites; },
    get favouritesOnly() { return favouritesOnly; },
    /** Subscribe before the initial get to avoid missing an event. Idempotent: views remount. */
    init() {
      started ??= (async () => {
        const unlisten = await onSettingsChanged(apply);
        try {
          apply(await getSettings());
        } catch (e) {
          unlisten();
          started = null;
          throw e;
        }
      })();
      return started;
    },
    toggleFavourite: (stationId: string) => toggleFavourite(stationId),
    setFavouritesOnly: (value: boolean) => setFavouritesOnly(value),
  };
}

export const settingsStore = createSettingsStore();
