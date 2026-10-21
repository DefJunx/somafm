import { applyReducedMotionDefault, getSettings, onSettingsChanged, setFadeAudio, setFavouritesOnly, setShowVisualizer, toggleFavourite, type SettingsView } from "./api";

const prefersReducedMotion = () => typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;

/** View of Rust's settings, replaced wholesale on every `settings-changed` event. `init` also reports the OS reduced-motion preference once (Rust applies it only to a fresh or migrated file). */
export function createSettingsStore() {
  let favourites: string[] = $state([]);
  let favouritesOnly = $state(false);
  let fadeAudio = $state(true);
  let showVisualizer = $state(true);
  let started: Promise<void> | null = null;

  const apply = (s: SettingsView | undefined) => {
    if (!s) return;
    favourites = s.favourites;
    favouritesOnly = s.favouritesOnly;
    fadeAudio = s.fadeAudio;
    showVisualizer = s.showVisualizer;
  };

  return {
    get favourites() { return favourites; },
    get favouritesOnly() { return favouritesOnly; },
    get fadeAudio() { return fadeAudio; },
    get showVisualizer() { return showVisualizer; },
    /** Subscribe before the initial get to avoid missing an event. Idempotent: views remount. */
    init() {
      started ??= (async () => {
        const unlisten = await onSettingsChanged(apply);
        try {
          // Rust only uses this when settings.json was just created or migrated.
          await applyReducedMotionDefault(prefersReducedMotion()).catch(() => {});
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
    setFadeAudio: (value: boolean) => setFadeAudio(value),
    setShowVisualizer: (value: boolean) => setShowVisualizer(value),
  };
}

export const settingsStore = createSettingsStore();
