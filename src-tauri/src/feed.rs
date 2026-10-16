//! Station feed: parse `channels.json`, cache the last good copy, refresh on a schedule.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tokio::sync::watch;

pub const FEED_URL: &str = "https://somafm.com/channels.json";
const CACHE_FILE: &str = "channels.json";

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Playlist {
    pub url: String,
    pub format: String,
    pub quality: String,
}

/// Station as sent to the frontend (spec §3.3). `playlists` stays Rust-side
/// for `.pls` resolution in the player and is not serialized.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Station {
    pub id: String,
    pub title: String,
    pub description: String,
    pub dj: String,
    pub genres: Vec<String>,
    pub listeners: u32,
    pub last_playing: String,
    pub image: String,
    pub large_image: String,
    pub xl_image: String,
    #[serde(skip)]
    pub playlists: Vec<Playlist>,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct StationsSnapshot {
    pub stations: Vec<Station>,
    /// false: no cache and no successful fetch yet.
    pub loaded: bool,
}

fn text(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn parse_listeners(v: Option<&Value>) -> u32 {
    match v {
        Some(Value::String(s)) => s.trim().parse().unwrap_or(0),
        Some(Value::Number(n)) => n.as_u64().and_then(|n| u32::try_from(n).ok()).unwrap_or(0),
        _ => 0,
    }
}

fn parse_playlists(v: &Value) -> Vec<Playlist> {
    v.get("playlists")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|p| text(p, "format") != "aacp")
        .map(|p| Playlist {
            url: text(p, "url"),
            format: text(p, "format"),
            quality: text(p, "quality"),
        })
        .collect()
}

/// Parse a `channels.json` body. Channels without an `id` are skipped;
/// missing or extra fields are tolerated.
pub fn parse_channels(body: &str) -> Result<Vec<Station>, String> {
    let root: Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    if !root.get("channels").is_some_and(Value::is_array) {
        return Err("no `channels` array".into());
    }
    let stations = root
        .get("channels")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|c| {
            let id = c
                .get("id")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())?;
            Some(Station {
                id: id.to_string(),
                title: text(c, "title"),
                description: text(c, "description"),
                dj: text(c, "dj"),
                genres: text(c, "genre")
                    .split('|')
                    .map(str::trim)
                    .filter(|g| !g.is_empty())
                    .map(String::from)
                    .collect(),
                listeners: parse_listeners(c.get("listeners")),
                last_playing: text(c, "lastPlaying"),
                image: text(c, "image"),
                large_image: text(c, "largeimage"),
                xl_image: text(c, "xlimage"),
                playlists: parse_playlists(c),
            })
        })
        .collect();
    let stations: Vec<Station> = stations;
    if stations.is_empty() {
        return Err("no stations in feed".into());
    }
    Ok(stations)
}

/// Delay before retry number `failures` (1-based): `base`, 2x, 4x ... capped at `cap`.
pub fn backoff_delay(base: Duration, cap: Duration, failures: u32) -> Duration {
    let exp = failures.saturating_sub(1).min(16);
    base.saturating_mul(1u32 << exp).min(cap)
}

#[derive(Debug)]
pub enum FetchError {
    Http(reqwest::Error),
    Parse(String),
}

pub struct Feed {
    url: String,
    cache_path: PathBuf,
    client: reqwest::Client,
    snapshot: Mutex<StationsSnapshot>,
}

impl Feed {
    /// Creates the feed and loads the cached copy from `cache_dir`, if any.
    pub fn new(url: impl Into<String>, cache_dir: &Path) -> Self {
        let cache_path = cache_dir.join(CACHE_FILE);
        let snapshot = std::fs::read_to_string(&cache_path)
            .ok()
            .and_then(|body| parse_channels(&body).ok())
            .map(|stations| StationsSnapshot {
                stations,
                loaded: true,
            })
            .unwrap_or_default();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("http client");
        Self {
            url: url.into(),
            cache_path,
            client,
            snapshot: Mutex::new(snapshot),
        }
    }

    /// Atomic write: temp file in the same dir, then rename over the cache file.
    fn write_cache(&self, body: &str) -> std::io::Result<()> {
        if let Some(dir) = self.cache_path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.cache_path.with_extension("json.tmp");
        std::fs::write(&tmp, body)?;
        std::fs::rename(&tmp, &self.cache_path)
    }

    pub fn snapshot(&self) -> StationsSnapshot {
        self.snapshot.lock().unwrap().clone()
    }

    /// Fetches the feed once. On success caches the body and returns whether the
    /// snapshot changed. On failure the current snapshot is left untouched.
    pub async fn refresh(&self) -> Result<bool, FetchError> {
        let body = self
            .client
            .get(&self.url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(FetchError::Http)?
            .text()
            .await
            .map_err(FetchError::Http)?;
        let stations = parse_channels(&body).map_err(FetchError::Parse)?;
        if let Err(e) = self.write_cache(&body) {
            eprintln!("somafm: could not write station cache: {e}");
        }
        let next = StationsSnapshot {
            stations,
            loaded: true,
        };
        let mut cur = self.snapshot.lock().unwrap();
        let changed = *cur != next;
        *cur = next;
        Ok(changed)
    }
}

#[derive(Clone, Copy)]
pub struct Schedule {
    pub interval: Duration,
    pub backoff_base: Duration,
}

impl Schedule {
    pub const DEFAULT: Schedule = Schedule {
        interval: Duration::from_secs(300),
        backoff_base: Duration::from_secs(30),
    };
}

async fn wait_visible(rx: &mut watch::Receiver<bool>) -> bool {
    rx.wait_for(|v| *v).await.is_ok()
}

/// Refreshes while `visible` is true: immediately, then every `interval`; after a
/// failure retries on backoff capped at `interval`. Hidden: paused; on becoming
/// visible again, refreshes straight away. Calls `on_update` with the new snapshot
/// whenever data changed (including the first load). Returns when `visible`'s sender drops.
pub async fn run_refresh_loop(
    feed: Arc<Feed>,
    mut visible: watch::Receiver<bool>,
    schedule: Schedule,
    on_update: impl Fn(StationsSnapshot),
) {
    let mut failures = 0u32;
    loop {
        if !wait_visible(&mut visible).await {
            return;
        }
        let delay = match feed.refresh().await {
            Ok(changed) => {
                failures = 0;
                if changed {
                    on_update(feed.snapshot());
                }
                schedule.interval
            }
            Err(_) => {
                failures += 1;
                backoff_delay(schedule.backoff_base, schedule.interval, failures)
            }
        };
        tokio::select! {
            _ = tokio::time::sleep(delay) => {}
            hidden = visible.wait_for(|v| !*v) => {
                if hidden.is_err() {
                    return; // sender dropped
                }
                failures = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    const FIXTURE: &str = r#"{"channels":[
      {"id":"testwave","title":"Test Wave","description":"Invented beats","dj":"DJ Nobody",
       "genre":"ambient|electronic","listeners":"42","lastPlaying":"A - B",
       "image":"http://x.invalid/a.png","largeimage":"http://x.invalid/b.png","xlimage":"http://x.invalid/c.png",
       "extra":123,
       "playlists":[
         {"url":"http://x.invalid/a.pls","format":"mp3","quality":"highest"},
         {"url":"http://x.invalid/b.pls","format":"aacp","quality":"high"},
         {"url":"http://x.invalid/c.pls","format":"aac","quality":"low"}]},
      {"title":"No Id"},
      {"id":"bare"}
    ]}"#;

    #[test]
    fn parses_fields_splits_genres_and_drops_aacp() {
        let s = parse_channels(FIXTURE).unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].id, "testwave");
        assert_eq!(s[0].genres, vec!["ambient", "electronic"]);
        assert_eq!(s[0].listeners, 42);
        assert_eq!(s[0].xl_image, "http://x.invalid/c.png");
        let formats: Vec<_> = s[0].playlists.iter().map(|p| p.format.as_str()).collect();
        assert_eq!(formats, vec!["mp3", "aac"]);
    }

    #[test]
    fn tolerates_missing_fields_and_bad_listeners() {
        let s = parse_channels(r#"{"channels":[{"id":"bare","listeners":"n/a"}]}"#).unwrap();
        assert_eq!(s[0].title, "");
        assert!(s[0].genres.is_empty());
        assert_eq!(s[0].listeners, 0);
        assert!(s[0].playlists.is_empty());
    }

    #[test]
    fn rejects_malformed_json_and_tolerates_missing_channels() {
        assert!(parse_channels("not json").is_err());
        assert!(parse_channels("{}").is_err(), "no channels array");
        assert!(parse_channels(r#"{"channels":[]}"#).is_err());
        assert!(parse_channels(r#"{"channels":[{"title":"x"}]}"#).is_err());
    }

    #[test]
    fn serialises_camel_case() {
        let s = parse_channels(FIXTURE).unwrap();
        let v = serde_json::to_value(&s[0]).unwrap();
        assert_eq!(v["lastPlaying"], "A - B");
        assert_eq!(v["largeImage"], "http://x.invalid/b.png");
    }

    #[test]
    fn backoff_doubles_then_caps() {
        let (b, c) = (Duration::from_secs(30), Duration::from_secs(300));
        let secs: Vec<u64> = (1..=6).map(|n| backoff_delay(b, c, n).as_secs()).collect();
        assert_eq!(secs, vec![30, 60, 120, 240, 300, 300]);
        assert_eq!(backoff_delay(b, c, 1000).as_secs(), 300);
    }

    /// Stub server: replies to each request with the next entry of `script`
    /// (status, body); repeats the last entry afterwards. Returns (url, hits).
    async fn stub(script: Vec<(u16, String)>) -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/channels.json", listener.local_addr().unwrap());
        let hits = Arc::new(AtomicUsize::new(0));
        let h = hits.clone();
        tokio::spawn(async move {
            loop {
                let (mut sock, _) = listener.accept().await.unwrap();
                let n = h.fetch_add(1, Ordering::SeqCst);
                let (status, body) = script[n.min(script.len() - 1)].clone();
                let mut buf = [0u8; 2048];
                let _ = sock.read(&mut buf).await;
                let resp = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = sock.write_all(resp.as_bytes()).await;
            }
        });
        (url, hits)
    }

    fn ok() -> (u16, String) {
        (200, FIXTURE.to_string())
    }

    #[tokio::test]
    async fn fetch_success_updates_snapshot_and_cache() {
        let dir = tempfile::tempdir().unwrap();
        let (url, _) = stub(vec![ok()]).await;
        let feed = Feed::new(&url, dir.path());
        assert!(!feed.snapshot().loaded);
        assert!(feed.refresh().await.unwrap());
        assert!(feed.snapshot().loaded);
        assert_eq!(feed.snapshot().stations.len(), 2);
        assert!(
            !feed.refresh().await.unwrap(),
            "unchanged data reports no change"
        );
        let cached = Feed::new("http://127.0.0.1:1/", dir.path());
        assert_eq!(cached.snapshot().stations.len(), 2);
    }

    #[tokio::test]
    async fn failure_without_cache_stays_unloaded() {
        let dir = tempfile::tempdir().unwrap();
        let (url, _) = stub(vec![(500, "boom".into())]).await;
        let feed = Feed::new(&url, dir.path());
        assert!(feed.refresh().await.is_err());
        assert_eq!(feed.snapshot(), StationsSnapshot::default());
    }

    #[tokio::test]
    async fn empty_feed_is_a_failure_and_keeps_cache() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(CACHE_FILE), FIXTURE).unwrap();
        let (url, _) = stub(vec![(200, "{}".into())]).await;
        let feed = Feed::new(&url, dir.path());
        assert!(feed.refresh().await.is_err());
        assert_eq!(feed.snapshot().stations.len(), 2);
        assert_eq!(
            std::fs::read_to_string(dir.path().join(CACHE_FILE)).unwrap(),
            FIXTURE
        );

        let empty = tempfile::tempdir().unwrap();
        let feed = Feed::new(&url, empty.path());
        assert!(feed.refresh().await.is_err());
        assert!(!feed.snapshot().loaded);
        assert!(!empty.path().join(CACHE_FILE).exists());
    }

    #[tokio::test]
    async fn cache_write_failure_still_updates_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let (url, _) = stub(vec![ok()]).await;
        // cache dir path is a file, so create_dir_all/write fail
        let blocker = dir.path().join("blocked");
        std::fs::write(&blocker, "x").unwrap();
        let feed = Feed::new(&url, &blocker);
        assert!(feed.refresh().await.unwrap());
        assert!(feed.snapshot().loaded);
    }

    #[tokio::test]
    async fn failure_with_cache_keeps_cached_data() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(CACHE_FILE), FIXTURE).unwrap();
        let (url, _) = stub(vec![(200, "garbage".into())]).await;
        let feed = Feed::new(&url, dir.path());
        assert!(feed.snapshot().loaded);
        assert!(feed.refresh().await.is_err());
        assert_eq!(feed.snapshot().stations.len(), 2);
    }

    #[tokio::test]
    async fn loop_retries_after_failure_then_emits_once() {
        let dir = tempfile::tempdir().unwrap();
        let (url, hits) = stub(vec![(503, String::new()), (503, String::new()), ok()]).await;
        let feed = Arc::new(Feed::new(&url, dir.path()));
        let (tx, rx) = watch::channel(true);
        let emitted = Arc::new(AtomicUsize::new(0));
        let e = emitted.clone();
        let sched = Schedule {
            interval: Duration::from_millis(400),
            backoff_base: Duration::from_millis(10),
        };
        let task = tokio::spawn(run_refresh_loop(feed.clone(), rx, sched, move |s| {
            assert!(s.loaded);
            e.fetch_add(1, Ordering::SeqCst);
        }));
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert_eq!(
            hits.load(Ordering::SeqCst),
            3,
            "two failures then success, on backoff"
        );
        assert_eq!(emitted.load(Ordering::SeqCst), 1);
        drop(tx);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn loop_pauses_while_hidden_and_refreshes_on_show() {
        let dir = tempfile::tempdir().unwrap();
        let (url, hits) = stub(vec![ok()]).await;
        let feed = Arc::new(Feed::new(&url, dir.path()));
        let (tx, rx) = watch::channel(false);
        let sched = Schedule {
            interval: Duration::from_millis(30),
            backoff_base: Duration::from_millis(10),
        };
        let task = tokio::spawn(run_refresh_loop(feed, rx, sched, |_| {}));
        tokio::time::sleep(Duration::from_millis(120)).await;
        assert_eq!(hits.load(Ordering::SeqCst), 0, "hidden: no fetches");
        tx.send(true).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            hits.load(Ordering::SeqCst) >= 2,
            "visible: periodic fetches"
        );
        tx.send(false).unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        let frozen = hits.load(Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(120)).await;
        assert_eq!(hits.load(Ordering::SeqCst), frozen, "hidden again: paused");
        drop(tx);
        task.await.unwrap();
    }

    #[tokio::test]
    #[ignore = "hits the live SomaFM feed; run with --ignored"]
    async fn live_feed_parses() {
        let dir = tempfile::tempdir().unwrap();
        let feed = Feed::new(FEED_URL, dir.path());
        feed.refresh().await.unwrap();
        assert!(!feed.snapshot().stations.is_empty());
    }
}
