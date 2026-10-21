//! Station history: recent tracks from SomaFM's songs endpoint, fetched on demand
//! (no timer, no cache, nothing on disk).

use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::player::icy;

pub const SONGS_BASE_URL: &str = "https://somafm.com/songs";
const TIMEOUT: Duration = Duration::from_secs(15);

/// One played track as sent to the frontend. `time` is Unix seconds.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct HistoryEntry {
    pub time: u64,
    pub artist: String,
    pub title: String,
    /// Same string ICY `now_playing` yields for this track: the like key.
    pub raw: String,
}

fn text(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// SomaFM sends `date` as Unix seconds in a string; accept a number too.
fn parse_time(v: Option<&Value>) -> Option<u64> {
    match v? {
        Value::String(s) => s.trim().parse().ok(),
        Value::Number(n) => n.as_u64(),
        _ => None,
    }
}

/// Parse a `songs/<id>.json` body. Entries without a title or a usable date are
/// skipped; unknown fields are ignored.
pub fn parse_history(body: &str) -> Result<Vec<HistoryEntry>, String> {
    let root: Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let songs = root
        .get("songs")
        .and_then(Value::as_array)
        .ok_or("no `songs` array")?;
    Ok(songs
        .iter()
        .filter_map(|s| {
            let title = text(s, "title");
            if title.is_empty() {
                return None;
            }
            let artist = text(s, "artist");
            let joined = if artist.is_empty() {
                title.clone()
            } else {
                format!("{artist} - {title}")
            };
            Some(HistoryEntry {
                time: parse_time(s.get("date"))?,
                raw: icy::now_playing(&joined)?.raw,
                artist,
                title,
            })
        })
        .collect())
}

pub struct HistoryClient {
    base_url: String,
    client: reqwest::Client,
}

impl HistoryClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::with_timeout(base_url, TIMEOUT)
    }

    pub fn with_timeout(base_url: impl Into<String>, timeout: Duration) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .expect("http client"),
        }
    }

    /// GETs `<base>/<station_id>.json` once. No retry; any failure is an `Err`
    /// the frontend shows as "History unavailable".
    pub async fn fetch(&self, station_id: &str) -> Result<Vec<HistoryEntry>, String> {
        let safe = !station_id.is_empty()
            && station_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !safe {
            return Err(format!("invalid station id `{station_id}`"));
        }
        let body = self
            .client
            .get(format!("{}/{station_id}.json", self.base_url))
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;
        parse_history(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    const FIXTURE: &str = r#"{"id":"testwave","songs":[
      {"title":"Song One","artist":"Artist A","album":"Alb","albumArt":"x","date":"1700000100","extra":1},
      {"title":"Song Two","artist":"Artist B","date":1700000000},
      {"artist":"No Title","date":"1700000050"},
      {"title":"No Date","artist":"C"},
      "junk",
      {"title":"Bare","date":"1699999999"}
    ]}"#;

    #[test]
    fn parses_entries_and_skips_malformed_ones() {
        let h = parse_history(FIXTURE).unwrap();
        let got: Vec<_> = h
            .iter()
            .map(|e| (e.time, e.artist.as_str(), e.title.as_str()))
            .collect();
        assert_eq!(
            got,
            vec![
                (1700000100, "Artist A", "Song One"),
                (1700000000, "Artist B", "Song Two"),
                (1699999999, "", "Bare"),
            ]
        );
    }

    #[test]
    fn raw_matches_what_icy_would_produce() {
        let h = parse_history(FIXTURE).unwrap();
        assert_eq!(h[0].raw, "Artist A - Song One");
        assert_eq!(h[2].raw, "Bare", "blank artist: just the title");
        let blank = parse_history(
            r#"{"songs":[{"title":"T","artist":"   ","date":"1"},{"title":" T2 ","artist":" A ","date":"2"}]}"#,
        )
        .unwrap();
        assert_eq!(blank[0].raw, "T");
        assert_eq!(blank[1].raw, "A - T2");
        for e in h.iter().chain(&blank) {
            assert_eq!(crate::player::icy::now_playing(&e.raw).unwrap().raw, e.raw);
        }
    }

    #[test]
    fn rejects_invalid_json_and_missing_songs() {
        assert!(parse_history("not json").is_err());
        assert!(parse_history("{}").is_err());
        assert_eq!(parse_history(r#"{"songs":[]}"#).unwrap(), vec![]);
    }

    #[test]
    fn serialises_time_artist_title() {
        let v = serde_json::to_value(&parse_history(FIXTURE).unwrap()[0]).unwrap();
        assert_eq!(
            v,
            serde_json::json!({"time":1700000100,"artist":"Artist A","title":"Song One","raw":"Artist A - Song One"})
        );
    }

    /// Stub server answering every request with `(status, body)` after `delay`;
    /// returns (base url, hits, last request line).
    async fn stub(
        status: u16,
        body: &str,
        delay: Duration,
    ) -> (String, Arc<AtomicUsize>, Arc<std::sync::Mutex<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/songs", listener.local_addr().unwrap());
        let hits = Arc::new(AtomicUsize::new(0));
        let line = Arc::new(std::sync::Mutex::new(String::new()));
        let (h, l, body) = (hits.clone(), line.clone(), body.to_string());
        tokio::spawn(async move {
            loop {
                let (mut sock, _) = listener.accept().await.unwrap();
                h.fetch_add(1, Ordering::SeqCst);
                let mut buf = [0u8; 2048];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                *l.lock().unwrap() = String::from_utf8_lossy(&buf[..n])
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string();
                tokio::time::sleep(delay).await;
                let resp = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = sock.write_all(resp.as_bytes()).await;
            }
        });
        (base, hits, line)
    }

    #[tokio::test]
    async fn fetch_success_requests_the_station_json() {
        let (base, hits, line) = stub(200, FIXTURE, Duration::ZERO).await;
        let entries = HistoryClient::new(base).fetch("testwave").await.unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert_eq!(*line.lock().unwrap(), "GET /songs/testwave.json HTTP/1.1");
    }

    #[tokio::test]
    async fn http_error_invalid_json_and_timeout_are_errors() {
        let (base, hits, _) = stub(500, "boom", Duration::ZERO).await;
        assert!(HistoryClient::new(base).fetch("x").await.is_err());
        assert_eq!(hits.load(Ordering::SeqCst), 1, "no retry");

        let (base, _, _) = stub(200, "not json", Duration::ZERO).await;
        assert!(HistoryClient::new(base).fetch("x").await.is_err());

        let (base, _, _) = stub(200, FIXTURE, Duration::from_millis(500)).await;
        let c = HistoryClient::with_timeout(base, Duration::from_millis(50));
        assert!(c.fetch("x").await.is_err());
    }

    #[tokio::test]
    async fn unsafe_station_ids_are_refused_without_a_request() {
        let (base, hits, _) = stub(200, FIXTURE, Duration::ZERO).await;
        let c = HistoryClient::new(base);
        for id in ["", "../x", "a/b", "a?b"] {
            assert!(c.fetch(id).await.is_err(), "{id}");
        }
        assert_eq!(hits.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn constructing_the_client_makes_no_request() {
        // Nothing listens here; construction must not touch the network.
        let _ = HistoryClient::new("http://127.0.0.1:1/songs");
    }
}
