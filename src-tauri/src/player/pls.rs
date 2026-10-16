//! `.pls` parsing, playlist choice and the per-station Mirror-list cache (spec 5.6).

use std::path::{Path, PathBuf};

use super::ErrorKind;
use crate::feed::Playlist;

/// Ordered Mirror URLs from a `.pls` body: `FileN=` entries sorted by `N`
/// (file order for equal `N`), blanks skipped.
pub fn parse_pls(body: &str) -> Vec<String> {
    let mut entries: Vec<(u32, String)> = body
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            let key = key.trim();
            if key.len() < 5 || !key[..4].eq_ignore_ascii_case("file") {
                return None;
            }
            let n: u32 = key[4..].parse().ok()?;
            let value = value.trim();
            (!value.is_empty()).then(|| (n, value.to_string()))
        })
        .collect();
    entries.sort_by_key(|(n, _)| *n);
    entries.into_iter().map(|(_, url)| url).collect()
}

/// The MP3 playlist: prefer quality "highest", else the first.
pub fn pick_playlist(playlists: &[Playlist]) -> Option<&Playlist> {
    let mut matching = playlists.iter().filter(|p| p.format == "mp3");
    let first = matching.next()?;
    if first.quality == "highest" {
        return Some(first);
    }
    Some(matching.find(|p| p.quality == "highest").unwrap_or(first))
}

#[derive(Debug, PartialEq)]
pub enum FetchFailure {
    /// No HTTP answer (DNS, connect, timeout).
    Network,
    /// Server answered with an error status or an empty/garbage `.pls`.
    Unavailable,
}

/// Fresh Mirrors win; on failure fall back to the cache; with no cache the
/// failure decides the error kind.
pub fn choose_mirrors(
    fetched: Result<Vec<String>, FetchFailure>,
    cached: Option<Vec<String>>,
) -> Result<Vec<String>, ErrorKind> {
    let failure = match fetched {
        Ok(m) if !m.is_empty() => return Ok(m),
        Ok(_) => FetchFailure::Unavailable,
        Err(f) => f,
    };
    match cached {
        Some(m) if !m.is_empty() => Ok(m),
        _ => Err(match failure {
            FetchFailure::Network => ErrorKind::Offline,
            FetchFailure::Unavailable => ErrorKind::Unavailable,
        }),
    }
}

fn cache_file(dir: &Path, station_id: &str) -> PathBuf {
    let safe: String = station_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    dir.join(format!("mirrors-{safe}.txt"))
}

fn read_cache(path: &Path) -> Option<Vec<String>> {
    let body = std::fs::read_to_string(path).ok()?;
    Some(body.lines().map(str::to_string).collect())
}

fn write_cache(path: &Path, mirrors: &[String]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("txt.tmp");
    std::fs::write(&tmp, mirrors.join("\n"))?;
    std::fs::rename(&tmp, path)
}

async fn fetch(client: &reqwest::Client, url: &str) -> Result<Vec<String>, FetchFailure> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|_| FetchFailure::Network)?;
    if !resp.status().is_success() {
        return Err(FetchFailure::Unavailable);
    }
    let body = resp.text().await.map_err(|_| FetchFailure::Network)?;
    Ok(parse_pls(&body))
}

/// Fetch the `.pls`, refresh the cache on success, fall back to the cache on failure.
pub async fn resolve_mirrors(
    client: &reqwest::Client,
    pls_url: &str,
    cache_dir: &Path,
    station_id: &str,
) -> Result<Vec<String>, ErrorKind> {
    let path = cache_file(cache_dir, station_id);
    let fetched = fetch(client, pls_url).await;
    if let Ok(m) = &fetched
        && !m.is_empty()
        && let Err(e) = write_cache(&path, m)
    {
        eprintln!("somafm: could not write mirror cache: {e}");
    }
    choose_mirrors(fetched, read_cache(&path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pl(format: &str, quality: &str, url: &str) -> Playlist {
        Playlist {
            url: url.into(),
            format: format.into(),
            quality: quality.into(),
        }
    }

    #[test]
    fn parses_ordered_mirrors() {
        let body = "[playlist]\r\nNumberOfEntries=3\r\nFile2=https://b.example/s\r\nTitle2=x\r\nFile1=https://a.example/s\r\nfile3 = https://c.example/s \r\nLength1=-1\r\nVersion=2\r\n";
        assert_eq!(
            parse_pls(body),
            [
                "https://a.example/s",
                "https://b.example/s",
                "https://c.example/s"
            ]
        );
    }

    #[test]
    fn parse_tolerates_junk() {
        let cases: [(&str, Vec<&str>); 5] = [
            ("", vec![]),
            ("<html>nope</html>", vec![]),
            ("[playlist]\nFile1=\nFile2=   \n", vec![]),
            (
                "File=https://x\nFileX=https://y\nFile1=https://z",
                vec!["https://z"],
            ),
            ("File1=https://a?x=1&y=2", vec!["https://a?x=1&y=2"]),
        ];
        for (body, want) in cases {
            assert_eq!(parse_pls(body), want, "{body:?}");
        }
    }

    #[test]
    fn picks_mp3_playlist_preferring_highest() {
        let lists = [
            pl("mp3", "low", "m-low"),
            pl("aac", "highest", "a-high"),
            pl("mp3", "highest", "m-high"),
        ];
        assert_eq!(pick_playlist(&lists).unwrap().url, "m-high");
        let only_low = [pl("mp3", "low", "m-low")];
        assert_eq!(pick_playlist(&only_low).unwrap().url, "m-low");
        assert!(pick_playlist(&[pl("aac", "highest", "a")]).is_none());
        assert!(pick_playlist(&[]).is_none());
    }

    #[test]
    fn mirror_choice_table() {
        let fresh = vec!["f".to_string()];
        let old = vec!["o".to_string()];
        assert_eq!(
            choose_mirrors(Ok(fresh.clone()), Some(old.clone())),
            Ok(fresh)
        );
        assert_eq!(
            choose_mirrors(Err(FetchFailure::Network), Some(old.clone())),
            Ok(old.clone())
        );
        assert_eq!(choose_mirrors(Ok(vec![]), Some(old.clone())), Ok(old));
        assert_eq!(
            choose_mirrors(Err(FetchFailure::Network), None),
            Err(ErrorKind::Offline)
        );
        assert_eq!(
            choose_mirrors(Err(FetchFailure::Unavailable), None),
            Err(ErrorKind::Unavailable)
        );
        assert_eq!(
            choose_mirrors(Ok(vec![]), None),
            Err(ErrorKind::Unavailable)
        );
        assert_eq!(
            choose_mirrors(Err(FetchFailure::Network), Some(vec![])),
            Err(ErrorKind::Offline)
        );
    }

    #[test]
    fn cache_roundtrip_and_safe_names() {
        let dir = tempfile::tempdir().unwrap();
        let path = cache_file(dir.path(), "../evil id");
        assert_eq!(path.parent().unwrap(), dir.path());
        assert!(
            path.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with(".txt")
        );
        assert_eq!(read_cache(&path), None);
        let m = vec!["https://a".to_string(), "https://b".to_string()];
        write_cache(&path, &m).unwrap();
        assert_eq!(read_cache(&path), Some(m));
    }
}
