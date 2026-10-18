//! Settings: v1 schema, ordered migrations and lenient loading (spec §8).
//! This module is the only writer of `settings.json` (via `tauri-plugin-store`);
//! window geometry lives in its own file, outside this schema.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use serde_json::{Map, Value};
use tokio::runtime::Handle;

pub const FILE: &str = "settings.json";
pub const SETTINGS_CHANGED: &str = "settings-changed";
pub const SCHEMA_VERSION: i64 = 1;
pub const DEFAULT_VOLUME: f32 = 0.8;
pub const VOLUME_DEBOUNCE: Duration = Duration::from_millis(500);

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub schema_version: i64,
    pub favourites: Vec<String>,
    pub volume: f32,
    pub last_station_id: Option<String>,
    pub favourites_only: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            favourites: Vec::new(),
            volume: DEFAULT_VOLUME,
            last_station_id: None,
            favourites_only: false,
        }
    }
}

/// What `get_settings` and `settings-changed` carry.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub favourites: Vec<String>,
    pub favourites_only: bool,
}

/// One `vN -> vN+1` step, operating on the raw key map.
pub type Migration = fn(&mut Map<String, Value>);

/// Index `i` migrates v(i+1) to v(i+2). Empty while v1 is current.
pub const MIGRATIONS: &[Migration] = &[];

/// Runs the migrations from the map's version up to `1 + migrations.len()`.
/// A newer version is left alone. Returns whether anything was migrated.
pub fn migrate(map: &mut Map<String, Value>, migrations: &[Migration]) -> bool {
    let current = 1 + migrations.len() as i64;
    let version = map
        .get("schemaVersion")
        .and_then(Value::as_i64)
        .filter(|v| *v >= 1)
        .unwrap_or(1);
    if version >= current {
        return false;
    }
    for step in &migrations[(version - 1) as usize..] {
        step(map);
    }
    map.insert("schemaVersion".into(), current.into());
    true
}

/// The file is not a JSON object.
#[derive(Debug)]
pub struct Unparsable;

/// Parses file contents into settings. `Err` means the file is unparsable
/// (not a JSON object). Each invalid key falls back to its own default.
pub fn parse(contents: &str, migrations: &[Migration]) -> Result<(Settings, bool), Unparsable> {
    let Ok(Value::Object(mut map)) = serde_json::from_str::<Value>(contents) else {
        return Err(Unparsable);
    };
    let migrated = migrate(&mut map, migrations);
    let d = Settings::default();
    let mut favourites: Vec<String> = Vec::new();
    if let Some(Value::Array(items)) = map.get("favourites")
        && items.iter().all(Value::is_string)
    {
        for id in items.iter().filter_map(Value::as_str) {
            if !favourites.iter().any(|f| f == id) {
                favourites.push(id.to_string());
            }
        }
    }
    let settings = Settings {
        schema_version: map
            .get("schemaVersion")
            .and_then(Value::as_i64)
            .filter(|v| *v >= 1)
            .unwrap_or(d.schema_version),
        favourites,
        volume: map
            .get("volume")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
            .map_or(d.volume, |v| v.clamp(0.0, 1.0) as f32),
        last_station_id: match map.get("lastStationId") {
            Some(Value::String(s)) => Some(s.clone()),
            _ => d.last_station_id,
        },
        favourites_only: map
            .get("favouritesOnly")
            .and_then(Value::as_bool)
            .unwrap_or(d.favourites_only),
    };
    Ok((settings, migrated))
}

pub struct Loaded {
    pub settings: Settings,
    /// The file should be rewritten (migrated, or replaced by defaults).
    pub needs_save: bool,
}

/// Reads `path`. No file: defaults. Unparsable file: renamed to `<name>.bak`, defaults.
pub fn load_file(path: &Path, migrations: &[Migration]) -> Loaded {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return Loaded {
            settings: Settings::default(),
            needs_save: false,
        };
    };
    match parse(&contents, migrations) {
        Ok((settings, migrated)) => Loaded {
            settings,
            needs_save: migrated,
        },
        Err(Unparsable) => {
            let mut bak = path.as_os_str().to_owned();
            bak.push(".bak");
            let bak = PathBuf::from(bak);
            eprintln!(
                "settings: {} is unparsable, moving it to {}",
                path.display(),
                bak.display()
            );
            if let Err(e) = std::fs::rename(path, &bak) {
                eprintln!("settings: could not back up corrupt file: {e}");
            }
            Loaded {
                settings: Settings::default(),
                needs_save: true,
            }
        }
    }
}

/// Where settings are written. The app uses `tauri-plugin-store`; tests record.
pub trait Persist: Send + Sync + 'static {
    fn save(&self, settings: &Settings);
}

/// `tauri-plugin-store` backend. Only known keys are written, so unknown keys
/// (e.g. from a newer version) stay in the file.
pub struct StorePersist<R: tauri::Runtime>(pub Arc<tauri_plugin_store::Store<R>>);

impl<R: tauri::Runtime> Persist for StorePersist<R> {
    fn save(&self, s: &Settings) {
        self.0.set("schemaVersion", s.schema_version);
        self.0.set("favourites", s.favourites.clone());
        self.0.set("volume", s.volume as f64);
        self.0.set(
            "lastStationId",
            s.last_station_id.clone().map_or(Value::Null, Value::String),
        );
        self.0.set("favouritesOnly", s.favourites_only);
        if let Err(e) = self.0.save() {
            eprintln!("settings: save failed: {e}");
        }
    }
}

struct Inner {
    state: Mutex<Settings>,
    persist: Box<dyn Persist>,
    on_change: Box<dyn Fn(SettingsView) + Send + Sync>,
    rt: Handle,
    debounce: Duration,
    /// Bumped by each volume change; a pending write only runs if still current.
    volume_gen: AtomicU64,
    volume_dirty: Mutex<bool>,
}

/// Shared handle to the in-memory settings; every change is persisted.
#[derive(Clone)]
pub struct SettingsService {
    inner: Arc<Inner>,
}

impl SettingsService {
    pub fn new(
        initial: Settings,
        persist: impl Persist,
        rt: Handle,
        debounce: Duration,
        on_change: impl Fn(SettingsView) + Send + Sync + 'static,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                state: Mutex::new(initial),
                persist: Box::new(persist),
                on_change: Box::new(on_change),
                rt,
                debounce,
                volume_gen: AtomicU64::new(0),
                volume_dirty: Mutex::new(false),
            }),
        }
    }

    pub fn snapshot(&self) -> Settings {
        self.inner.state.lock().unwrap().clone()
    }

    pub fn view(&self) -> SettingsView {
        view_of(&self.inner.state.lock().unwrap())
    }

    /// Adds or removes `id`; emits `settings-changed`.
    pub fn toggle_favourite(&self, id: &str) {
        let (view, snap) = {
            let mut s = self.inner.state.lock().unwrap();
            match s.favourites.iter().position(|f| f == id) {
                Some(i) => {
                    s.favourites.remove(i);
                }
                None => s.favourites.push(id.to_string()),
            }
            (view_of(&s), s.clone())
        };
        self.inner.persist.save(&snap);
        (self.inner.on_change)(view);
    }

    pub fn set_favourites_only(&self, value: bool) {
        let (view, snap) = {
            let mut s = self.inner.state.lock().unwrap();
            if s.favourites_only == value {
                return;
            }
            s.favourites_only = value;
            (view_of(&s), s.clone())
        };
        self.inner.persist.save(&snap);
        (self.inner.on_change)(view);
    }

    /// Written immediately when a `play` for a known Station is accepted.
    pub fn set_last_station(&self, id: &str) {
        let snap = {
            let mut s = self.inner.state.lock().unwrap();
            if s.last_station_id.as_deref() == Some(id) {
                return;
            }
            s.last_station_id = Some(id.to_string());
            s.clone()
        };
        self.inner.persist.save(&snap);
    }

    /// Updates memory now; the file write is debounced.
    pub fn set_volume(&self, volume: f32) {
        let volume = if volume.is_finite() {
            volume.clamp(0.0, 1.0)
        } else {
            return;
        };
        self.inner.state.lock().unwrap().volume = volume;
        *self.inner.volume_dirty.lock().unwrap() = true;
        let generation = self.inner.volume_gen.fetch_add(1, Ordering::SeqCst) + 1;
        let this = self.clone();
        self.inner.rt.spawn(async move {
            tokio::time::sleep(this.inner.debounce).await;
            if this.inner.volume_gen.load(Ordering::SeqCst) == generation {
                this.flush();
            }
        });
    }

    /// Writes a pending debounced volume now (also called on exit).
    pub fn flush(&self) {
        let dirty = std::mem::take(&mut *self.inner.volume_dirty.lock().unwrap());
        if dirty {
            let snap = self.snapshot();
            self.inner.persist.save(&snap);
        }
    }
}

fn view_of(s: &Settings) -> SettingsView {
    SettingsView {
        favourites: s.favourites.clone(),
        favourites_only: s.favourites_only,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn p(contents: &str) -> Settings {
        parse(contents, MIGRATIONS).unwrap().0
    }

    #[test]
    fn missing_file_gives_defaults_without_saving() {
        let dir = tempfile::tempdir().unwrap();
        let l = load_file(&dir.path().join(FILE), MIGRATIONS);
        assert_eq!(l.settings, Settings::default());
        assert!(!l.needs_save);
        assert_eq!(l.settings.volume, 0.8);
        assert!(l.settings.favourites.is_empty());
        assert_eq!(l.settings.last_station_id, None);
        assert!(!l.settings.favourites_only);
    }

    #[test]
    fn valid_v1_file_is_read() {
        let s = p(
            r#"{"schemaVersion":1,"favourites":["a","b"],"volume":0.25,"lastStationId":"b","favouritesOnly":true}"#,
        );
        assert_eq!(s.favourites, ["a", "b"]);
        assert_eq!(s.volume, 0.25);
        assert_eq!(s.last_station_id.as_deref(), Some("b"));
        assert!(s.favourites_only);
    }

    #[test]
    fn invalid_key_falls_back_alone() {
        let s = p(
            r#"{"schemaVersion":1,"favourites":"nope","volume":"loud","lastStationId":7,"favouritesOnly":"yes"}"#,
        );
        assert_eq!(s, Settings::default());
        let s = p(r#"{"schemaVersion":1,"favourites":["a"],"volume":"loud"}"#);
        assert_eq!(s.favourites, ["a"]);
        assert_eq!(s.volume, 0.8);
    }

    #[test]
    fn volume_is_clamped_and_favourites_deduplicated() {
        assert_eq!(p(r#"{"volume":7}"#).volume, 1.0);
        assert_eq!(p(r#"{"volume":-1}"#).volume, 0.0);
        assert_eq!(p(r#"{"favourites":["a","b","a"]}"#).favourites, ["a", "b"]);
    }

    #[test]
    fn corrupt_file_is_moved_to_bak_and_defaults_used() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE);
        std::fs::write(&path, "{not json").unwrap();
        let l = load_file(&path, MIGRATIONS);
        assert_eq!(l.settings, Settings::default());
        assert!(l.needs_save);
        assert!(!path.exists());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("settings.json.bak")).unwrap(),
            "{not json"
        );
        // A JSON value that is not an object is just as unusable.
        std::fs::write(&path, "[1,2]").unwrap();
        assert!(parse("[1,2]", MIGRATIONS).is_err());
        load_file(&path, MIGRATIONS);
        assert!(!path.exists());
    }

    #[test]
    fn newer_schema_version_is_tolerated_and_kept() {
        let s = p(r#"{"schemaVersion":99,"favourites":["a"],"volume":0.5,"future":true}"#);
        assert_eq!(s.schema_version, 99);
        assert_eq!(s.favourites, ["a"]);
        assert_eq!(s.volume, 0.5);
    }

    fn v1_to_v2(m: &mut Map<String, Value>) {
        let old = m.remove("vol").unwrap_or(json!(0.1));
        m.insert("step1".into(), old);
    }
    fn v2_to_v3(m: &mut Map<String, Value>) {
        // Only correct if v1->v2 ran first.
        let seen = m.contains_key("step1");
        m.insert("volume".into(), json!(if seen { 0.33 } else { 0.99 }));
    }

    #[test]
    fn migrations_run_in_order_and_bump_the_version() {
        let (s, migrated) = parse(r#"{"schemaVersion":1}"#, &[v1_to_v2, v2_to_v3]).unwrap();
        assert!(migrated);
        assert_eq!(s.schema_version, 3);
        assert!((s.volume - 0.33).abs() < 1e-6);
    }

    #[test]
    fn migration_starts_from_the_file_version() {
        let (s, migrated) = parse(r#"{"schemaVersion":2}"#, &[v1_to_v2, v2_to_v3]).unwrap();
        assert!(migrated);
        assert_eq!(s.schema_version, 3);
        assert!((s.volume - 0.99).abs() < 1e-6);
        let (_, migrated) = parse(r#"{"schemaVersion":3}"#, &[v1_to_v2, v2_to_v3]).unwrap();
        assert!(!migrated);
    }

    #[test]
    fn migrated_file_needs_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE);
        std::fs::write(&path, r#"{"schemaVersion":1}"#).unwrap();
        let l = load_file(&path, &[v1_to_v2]);
        assert!(l.needs_save);
        assert_eq!(l.settings.schema_version, 2);
    }

    #[derive(Clone, Default)]
    struct Rec(Arc<Mutex<Vec<Settings>>>);
    impl Persist for Rec {
        fn save(&self, s: &Settings) {
            self.0.lock().unwrap().push(s.clone());
        }
    }

    fn service(initial: Settings) -> (SettingsService, Rec, Arc<Mutex<Vec<SettingsView>>>) {
        let rec = Rec::default();
        let events = Arc::new(Mutex::new(Vec::new()));
        let e = events.clone();
        let svc = SettingsService::new(
            initial,
            rec.clone(),
            Handle::current(),
            Duration::from_millis(40),
            move |v| e.lock().unwrap().push(v),
        );
        (svc, rec, events)
    }

    #[tokio::test]
    async fn toggle_favourite_adds_removes_saves_and_emits() {
        let (svc, rec, events) = service(Settings::default());
        svc.toggle_favourite("a");
        svc.toggle_favourite("b");
        svc.toggle_favourite("a");
        assert_eq!(svc.view().favourites, ["b"]);
        assert_eq!(rec.0.lock().unwrap().len(), 3);
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].favourites, ["a"]);
        assert_eq!(events[2].favourites, ["b"]);
    }

    #[tokio::test]
    async fn set_favourites_only_persists_and_emits() {
        let (svc, rec, events) = service(Settings::default());
        svc.set_favourites_only(true);
        assert!(svc.view().favourites_only);
        assert!(rec.0.lock().unwrap().last().unwrap().favourites_only);
        assert!(events.lock().unwrap()[0].favourites_only);
    }

    #[tokio::test]
    async fn last_station_is_saved_immediately() {
        let (svc, rec, events) = service(Settings::default());
        svc.set_last_station("x");
        assert_eq!(
            rec.0
                .lock()
                .unwrap()
                .last()
                .unwrap()
                .last_station_id
                .as_deref(),
            Some("x")
        );
        assert!(events.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn volume_writes_are_debounced_to_the_last_value() {
        let (svc, rec, _) = service(Settings::default());
        for v in [0.1, 0.2, 0.3] {
            svc.set_volume(v);
        }
        assert_eq!(svc.snapshot().volume, 0.3);
        assert!(rec.0.lock().unwrap().is_empty());
        tokio::time::sleep(Duration::from_millis(150)).await;
        let saved = rec.0.lock().unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].volume, 0.3);
    }

    #[tokio::test]
    async fn flush_writes_a_pending_volume_once() {
        let (svc, rec, _) = service(Settings::default());
        svc.set_volume(0.4);
        svc.flush();
        svc.flush();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(rec.0.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn volume_is_clamped() {
        let (svc, _, _) = service(Settings::default());
        svc.set_volume(5.0);
        assert_eq!(svc.snapshot().volume, 1.0);
    }

    #[test]
    fn view_serialises_camel_case() {
        let v = serde_json::to_value(SettingsView {
            favourites: vec!["a".into()],
            favourites_only: true,
        })
        .unwrap();
        assert_eq!(v, json!({"favourites":["a"],"favouritesOnly":true}));
    }
}
