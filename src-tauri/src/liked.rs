//! Liked tracks: the raw title is the source of truth, plus Station id and the
//! time liked. Own file (`liked-tracks.json`) with its own `schemaVersion`, so it
//! recovers from corruption independently of `settings.json`. This module is
//! the only writer of that file (via `tauri-plugin-store`).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::Value;

use crate::settings::{Migration, Unparsable, migrate};

pub const FILE: &str = "liked-tracks.json";
pub const LIKED_CHANGED: &str = "liked-changed";
pub const SCHEMA_VERSION: i64 = 1;

/// Index `i` migrates v(i+1) to v(i+2). Empty while v1 is current.
pub const MIGRATIONS: &[Migration] = &[];

/// Artist/title are not stored: they are split from `raw` for display only.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LikedTrack {
    pub raw: String,
    pub station_id: String,
    /// Unix milliseconds.
    pub liked_at: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LikedFile {
    pub schema_version: i64,
    /// Oldest first (insertion order).
    pub tracks: Vec<LikedTrack>,
}

impl Default for LikedFile {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            tracks: Vec::new(),
        }
    }
}

/// Parses file contents. `Err` means not a JSON object. Invalid entries are
/// skipped and duplicates on (stationId, raw) keep the first.
pub fn parse(contents: &str, migrations: &[Migration]) -> Result<(LikedFile, bool), Unparsable> {
    let Ok(Value::Object(mut map)) = serde_json::from_str::<Value>(contents) else {
        return Err(Unparsable);
    };
    let migrated = migrate(&mut map, migrations);
    let mut tracks: Vec<LikedTrack> = Vec::new();
    for item in map
        .get("tracks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let (Some(raw), Some(station_id), Some(liked_at)) = (
            item.get("raw").and_then(Value::as_str),
            item.get("stationId").and_then(Value::as_str),
            item.get("likedAt").and_then(Value::as_u64),
        ) else {
            continue;
        };
        if raw.trim().is_empty()
            || tracks
                .iter()
                .any(|t| t.station_id == station_id && t.raw == raw)
        {
            continue;
        }
        tracks.push(LikedTrack {
            raw: raw.to_string(),
            station_id: station_id.to_string(),
            liked_at,
        });
    }
    let schema_version = map
        .get("schemaVersion")
        .and_then(Value::as_i64)
        .filter(|v| *v >= 1)
        .unwrap_or(SCHEMA_VERSION);
    Ok((
        LikedFile {
            schema_version,
            tracks,
        },
        migrated,
    ))
}

pub struct Loaded {
    pub file: LikedFile,
    /// The file should be rewritten (migrated, or replaced by defaults).
    pub needs_save: bool,
}

/// Reads `path`. No file: empty. Unparsable file: renamed to `<name>.bak`, empty.
pub fn load_file(path: &Path, migrations: &[Migration]) -> Loaded {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return Loaded {
            file: LikedFile::default(),
            needs_save: false,
        };
    };
    match parse(&contents, migrations) {
        Ok((file, migrated)) => Loaded {
            file,
            needs_save: migrated,
        },
        Err(Unparsable) => {
            let mut bak = path.as_os_str().to_owned();
            bak.push(".bak");
            let bak = PathBuf::from(bak);
            eprintln!(
                "liked: {} is unparsable, moving it to {}",
                path.display(),
                bak.display()
            );
            if let Err(e) = std::fs::rename(path, &bak) {
                eprintln!("liked: could not back up corrupt file: {e}");
            }
            Loaded {
                file: LikedFile::default(),
                needs_save: true,
            }
        }
    }
}

/// Where liked tracks are written. The app uses `tauri-plugin-store`; tests record.
pub trait Persist: Send + Sync + 'static {
    fn save(&self, file: &LikedFile);
}

/// `tauri-plugin-store` backend. Only known keys are written, so unknown keys
/// (e.g. from a newer version) stay in the file.
pub struct StorePersist<R: tauri::Runtime>(pub Arc<tauri_plugin_store::Store<R>>);

impl<R: tauri::Runtime> Persist for StorePersist<R> {
    fn save(&self, f: &LikedFile) {
        self.0.set("schemaVersion", f.schema_version);
        self.0.set(
            "tracks",
            serde_json::to_value(&f.tracks).unwrap_or(Value::Array(Vec::new())),
        );
        if let Err(e) = self.0.save() {
            eprintln!("liked: save failed: {e}");
        }
    }
}

/// Shared handle to the in-memory liked tracks; every change is persisted.
#[derive(Clone)]
pub struct LikedService {
    state: Arc<Mutex<LikedFile>>,
    persist: Arc<dyn Persist>,
    on_change: Arc<dyn Fn(Vec<LikedTrack>) + Send + Sync>,
}

impl LikedService {
    pub fn new(
        initial: LikedFile,
        persist: impl Persist,
        on_change: impl Fn(Vec<LikedTrack>) + Send + Sync + 'static,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(initial)),
            persist: Arc::new(persist),
            on_change: Arc::new(on_change),
        }
    }

    /// Newest first.
    pub fn list(&self) -> Vec<LikedTrack> {
        newest_first(&self.state.lock().unwrap())
    }

    /// Adds (at `now`) or removes the (station, raw) pair; returns whether it is
    /// liked afterwards. Adding needs `station_known(station_id)`; removing never
    /// does (a Station may have left the feed). Persists and emits `liked-changed`.
    pub fn toggle(
        &self,
        station_id: &str,
        raw: &str,
        now: u64,
        station_known: impl Fn(&str) -> bool,
    ) -> Result<bool, String> {
        if raw.trim().is_empty() {
            return Ok(false);
        }
        // Persist and emit under the lock so concurrent changes stay ordered.
        let mut f = self.state.lock().unwrap();
        let liked = match position(&f, station_id, raw) {
            Some(i) => {
                f.tracks.remove(i);
                false
            }
            None if station_known(station_id) => {
                f.tracks.push(LikedTrack {
                    raw: raw.to_string(),
                    station_id: station_id.to_string(),
                    liked_at: now,
                });
                true
            }
            None => return Err(format!("unknown station `{station_id}`")),
        };
        self.changed(&f);
        Ok(liked)
    }

    /// Removes the pair if present; persists and emits only on a change.
    pub fn remove(&self, station_id: &str, raw: &str) {
        let mut f = self.state.lock().unwrap();
        let Some(i) = position(&f, station_id, raw) else {
            return;
        };
        f.tracks.remove(i);
        self.changed(&f);
    }

    fn changed(&self, snap: &LikedFile) {
        self.persist.save(snap);
        (self.on_change)(newest_first(snap));
    }
}

fn position(f: &LikedFile, station_id: &str, raw: &str) -> Option<usize> {
    f.tracks
        .iter()
        .position(|t| t.station_id == station_id && t.raw == raw)
}

fn newest_first(f: &LikedFile) -> Vec<LikedTrack> {
    f.tracks.iter().rev().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn t(station: &str, raw: &str, at: u64) -> LikedTrack {
        LikedTrack {
            raw: raw.into(),
            station_id: station.into(),
            liked_at: at,
        }
    }

    fn p(contents: &str) -> LikedFile {
        parse(contents, MIGRATIONS).unwrap().0
    }

    #[test]
    fn missing_file_gives_empty_without_saving() {
        let dir = tempfile::tempdir().unwrap();
        let l = load_file(&dir.path().join(FILE), MIGRATIONS);
        assert_eq!(l.file, LikedFile::default());
        assert!(!l.needs_save);
    }

    #[test]
    fn valid_v1_file_is_read() {
        let f = p(
            r#"{"schemaVersion":1,"tracks":[{"raw":"A - B","stationId":"x","likedAt":5},{"raw":"C","stationId":"y","likedAt":9}]}"#,
        );
        assert_eq!(f.tracks, vec![t("x", "A - B", 5), t("y", "C", 9)]);
    }

    #[test]
    fn invalid_entries_are_skipped_and_duplicates_collapse() {
        let f = p(r#"{"tracks":[
            {"raw":"A","stationId":"x","likedAt":1},
            {"raw":"A","stationId":"x","likedAt":2},
            {"raw":"A","stationId":"y","likedAt":3},
            {"raw":"","stationId":"x","likedAt":4},
            {"raw":"B","stationId":"x"},
            {"raw":7,"stationId":"x","likedAt":1},
            "junk"]}"#);
        assert_eq!(f.tracks, vec![t("x", "A", 1), t("y", "A", 3)]);
        assert!(p(r#"{"tracks":"nope"}"#).tracks.is_empty());
    }

    #[test]
    fn corrupt_file_is_moved_to_bak_and_empty_used() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE);
        std::fs::write(&path, "{not json").unwrap();
        let l = load_file(&path, MIGRATIONS);
        assert_eq!(l.file, LikedFile::default());
        assert!(l.needs_save);
        assert!(!path.exists());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("liked-tracks.json.bak")).unwrap(),
            "{not json"
        );
        assert!(parse("[1,2]", MIGRATIONS).is_err());
    }

    #[test]
    fn newer_schema_version_is_tolerated_and_kept() {
        let f = p(
            r#"{"schemaVersion":99,"tracks":[{"raw":"A","stationId":"x","likedAt":1}],"future":true}"#,
        );
        assert_eq!(f.schema_version, 99);
        assert_eq!(f.tracks.len(), 1);
    }

    #[test]
    fn migrations_run_and_flag_a_save() {
        fn v1_to_v2(m: &mut serde_json::Map<String, Value>) {
            m.insert(
                "tracks".into(),
                json!([{"raw":"M","stationId":"x","likedAt":1}]),
            );
        }
        let (f, migrated) = parse(r#"{"schemaVersion":1}"#, &[v1_to_v2]).unwrap();
        assert!(migrated);
        assert_eq!(f.schema_version, 2);
        assert_eq!(f.tracks.len(), 1);
    }

    #[test]
    fn corrupt_liked_file_does_not_touch_settings_and_vice_versa() {
        use crate::settings;
        let dir = tempfile::tempdir().unwrap();
        let sp = dir.path().join(settings::FILE);
        let lp = dir.path().join(FILE);
        std::fs::write(&sp, r#"{"schemaVersion":1,"favourites":["a"]}"#).unwrap();
        std::fs::write(&lp, "{not json").unwrap();
        let l = load_file(&lp, MIGRATIONS);
        let s = settings::load_file(&sp, settings::MIGRATIONS);
        assert!(l.needs_save && !s.needs_save);
        assert_eq!(s.settings.favourites, ["a"]);
        assert!(sp.exists());

        std::fs::write(&sp, "{not json").unwrap();
        std::fs::write(
            &lp,
            r#"{"tracks":[{"raw":"A","stationId":"x","likedAt":1}]}"#,
        )
        .unwrap();
        let s = settings::load_file(&sp, settings::MIGRATIONS);
        let l = load_file(&lp, MIGRATIONS);
        assert!(s.needs_save && !l.needs_save);
        assert_eq!(l.file.tracks.len(), 1);
        assert!(lp.exists());
    }

    #[derive(Clone, Default)]
    struct Rec(Arc<Mutex<Vec<LikedFile>>>);
    impl Persist for Rec {
        fn save(&self, f: &LikedFile) {
            self.0.lock().unwrap().push(f.clone());
        }
    }

    fn service(initial: LikedFile) -> (LikedService, Rec, Arc<Mutex<Vec<Vec<LikedTrack>>>>) {
        let rec = Rec::default();
        let events = Arc::new(Mutex::new(Vec::new()));
        let e = events.clone();
        let svc = LikedService::new(initial, rec.clone(), move |v| e.lock().unwrap().push(v));
        (svc, rec, events)
    }

    fn tog(svc: &LikedService, station: &str, raw: &str, now: u64) -> bool {
        svc.toggle(station, raw, now, |_| true).unwrap()
    }

    #[test]
    fn unknown_station_cannot_be_liked_but_an_existing_like_can_be_removed() {
        let (svc, rec, _) = service(LikedFile::default());
        assert!(svc.toggle("gone", "A", 1, |_| false).is_err());
        assert!(svc.list().is_empty());
        assert!(rec.0.lock().unwrap().is_empty());
        tog(&svc, "gone", "A", 1);
        assert_eq!(svc.toggle("gone", "A", 2, |_| false), Ok(false));
        assert!(svc.list().is_empty());
    }

    #[test]
    fn known_check_is_asked_about_the_station_id() {
        let (svc, _, _) = service(LikedFile::default());
        assert_eq!(svc.toggle("x", "A", 1, |id| id == "x"), Ok(true));
        assert!(svc.toggle("y", "A", 1, |id| id == "x").is_err());
    }

    #[test]
    fn persist_and_emit_happen_in_change_order_under_concurrency() {
        let (svc, rec, events) = service(LikedFile::default());
        let hs: Vec<_> = (0..8)
            .map(|n| {
                let svc = svc.clone();
                std::thread::spawn(move || {
                    for i in 0..50 {
                        tog(&svc, "x", &format!("t{n}-{i}"), 1);
                    }
                })
            })
            .collect();
        hs.into_iter().for_each(|h| h.join().unwrap());
        let saved = rec.0.lock().unwrap();
        let lens: Vec<_> = saved.iter().map(|f| f.tracks.len()).collect();
        assert_eq!(lens, (1..=400).collect::<Vec<_>>());
        let ev: Vec<_> = events.lock().unwrap().iter().map(Vec::len).collect();
        assert_eq!(ev, lens);
    }

    #[test]
    fn toggle_adds_then_removes_and_dedupes_on_station_and_raw() {
        let (svc, rec, events) = service(LikedFile::default());
        assert!(tog(&svc, "x", "A - B", 10));
        assert!(tog(&svc, "y", "A - B", 20), "other station is distinct");
        assert_eq!(svc.list().len(), 2);
        assert!(!tog(&svc, "x", "A - B", 30), "second toggle unlikes");
        assert_eq!(svc.list(), vec![t("y", "A - B", 20)]);
        assert_eq!(rec.0.lock().unwrap().len(), 3);
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[2], vec![t("y", "A - B", 20)]);
    }

    #[test]
    fn list_is_newest_first() {
        let (svc, _, _) = service(LikedFile::default());
        tog(&svc, "x", "one", 1);
        tog(&svc, "x", "two", 2);
        tog(&svc, "x", "three", 2);
        let raws: Vec<_> = svc.list().into_iter().map(|l| l.raw).collect();
        assert_eq!(raws, ["three", "two", "one"]);
    }

    #[test]
    fn remove_deletes_only_a_present_pair() {
        let (svc, rec, events) = service(LikedFile::default());
        tog(&svc, "x", "A", 1);
        svc.remove("x", "missing");
        svc.remove("y", "A");
        assert_eq!(rec.0.lock().unwrap().len(), 1);
        svc.remove("x", "A");
        assert!(svc.list().is_empty());
        assert_eq!(rec.0.lock().unwrap().len(), 2);
        assert_eq!(events.lock().unwrap().len(), 2);
    }

    #[test]
    fn blank_raw_is_not_liked() {
        let (svc, rec, _) = service(LikedFile::default());
        assert!(!tog(&svc, "x", "   ", 1));
        assert!(svc.list().is_empty());
        assert!(rec.0.lock().unwrap().is_empty());
    }

    #[test]
    fn serialises_camel_case() {
        let v = serde_json::to_value(t("x", "A - B", 5)).unwrap();
        assert_eq!(v, json!({"raw":"A - B","stationId":"x","likedAt":5}));
    }

    #[test]
    fn saved_snapshot_keeps_the_schema_version() {
        let (svc, rec, _) = service(LikedFile {
            schema_version: 99,
            tracks: vec![],
        });
        tog(&svc, "x", "A", 1);
        assert_eq!(rec.0.lock().unwrap()[0].schema_version, 99);
    }
}
