//! ICY (SHOUTcast) in-band metadata: strips metadata blocks from the audio
//! bytes and extracts `StreamTitle` values (spec 5.1, 5.3).

use std::io::{self, Read};

use super::NowPlaying;

/// Extracts the `StreamTitle` value from one metadata block (length byte
/// already removed; trailing NUL padding allowed). `None` when absent or empty.
/// Titles may contain `'`, so the value ends at the first `';` (else at the
/// last `'`, else the end of the block). Non-UTF-8 bytes are replaced.
pub fn parse_stream_title(block: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(block);
    let text = text.trim_end_matches('\0');
    const KEY: &str = "StreamTitle=";
    let rest = &text[text.find(KEY)? + KEY.len()..];
    let rest = rest.strip_prefix('\'').unwrap_or(rest);
    let value = match rest.find("';") {
        Some(i) => &rest[..i],
        None => {
            let rest = rest.strip_suffix(';').unwrap_or(rest);
            rest.strip_suffix('\'').unwrap_or(rest)
        }
    };
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

/// Splits a raw title on the first " - " into artist and title (spec 5.3).
/// `None` for an empty title (no change).
pub fn now_playing(raw: &str) -> Option<NowPlaying> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let (artist, title) = match raw.split_once(" - ") {
        Some((a, t)) => (Some(a.trim().to_string()), t.trim().to_string()),
        None => (None, raw.to_string()),
    };
    Some(NowPlaying {
        raw: raw.to_string(),
        artist: artist.filter(|a| !a.is_empty()),
        title,
    })
}

/// Yields only the audio bytes of an ICY stream: after every `metaint` audio
/// bytes it consumes a length byte (x16) and that many metadata bytes, passing
/// any `StreamTitle` to `on_title`. `metaint: None` (or 0) is a pass-through.
pub struct IcyReader<R, F> {
    inner: R,
    metaint: Option<usize>,
    left: usize,
    on_title: F,
}

impl<R: Read, F: FnMut(String)> IcyReader<R, F> {
    pub fn new(inner: R, metaint: Option<usize>, on_title: F) -> Self {
        let metaint = metaint.filter(|&m| m > 0);
        Self {
            inner,
            metaint,
            left: metaint.unwrap_or(0),
            on_title,
        }
    }

    /// Reads one metadata block. `Ok(false)` on end of stream.
    fn read_block(&mut self) -> io::Result<bool> {
        let mut len = [0u8; 1];
        loop {
            match self.inner.read(&mut len) {
                Ok(0) => return Ok(false),
                Ok(_) => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        let mut block = vec![0u8; len[0] as usize * 16];
        match self.inner.read_exact(&mut block) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(false),
            Err(e) => return Err(e),
        }
        if let Some(title) = parse_stream_title(&block) {
            (self.on_title)(title);
        }
        Ok(true)
    }
}

impl<R: Read, F: FnMut(String)> Read for IcyReader<R, F> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let Some(metaint) = self.metaint else {
            return self.inner.read(out);
        };
        if out.is_empty() {
            return Ok(0);
        }
        if self.left == 0 {
            if !self.read_block()? {
                return Ok(0);
            }
            self.left = metaint;
        }
        let n = out.len().min(self.left);
        let got = self.inner.read(&mut out[..n])?;
        self.left -= got;
        Ok(got)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::io::Cursor;

    /// Builds an ICY-framed stream: a metadata block after every `metaint`
    /// audio bytes (`titles[i]`, or an empty block once they run out).
    pub fn frame(audio: &[u8], metaint: usize, titles: &[&str]) -> Vec<u8> {
        let mut out = Vec::new();
        for (i, chunk) in audio.chunks(metaint).enumerate() {
            out.extend_from_slice(chunk);
            if chunk.len() < metaint {
                break;
            }
            match titles.get(i) {
                Some(t) => {
                    let mut b = format!("StreamTitle='{t}';").into_bytes();
                    b.resize(b.len().div_ceil(16) * 16, 0);
                    out.push((b.len() / 16) as u8);
                    out.extend(b);
                }
                None => out.push(0),
            }
        }
        out
    }

    #[test]
    fn stream_title_table() {
        let cases: [(&[u8], Option<&str>); 11] = [
            (b"StreamTitle='A - B';", Some("A - B")),
            (
                b"StreamTitle='A - B';StreamUrl='http://x';\0\0\0",
                Some("A - B"),
            ),
            (
                b"StreamTitle='Don't Stop - Me';StreamUrl='';",
                Some("Don't Stop - Me"),
            ),
            (b"StreamTitle='A - B", Some("A - B")),
            (b"StreamTitle='A - B';\0\0\0\0\0", Some("A - B")),
            (b"StreamTitle='';", None),
            (b"StreamTitle='  ';", None),
            (b"StreamUrl='x';", None),
            (b"", None),
            (b"\0\0\0\0", None),
            (b"StreamTitle=Plain;", Some("Plain")),
        ];
        for (block, want) in cases {
            assert_eq!(
                parse_stream_title(block).as_deref(),
                want,
                "{}",
                String::from_utf8_lossy(block)
            );
        }
    }

    #[test]
    fn stream_title_non_utf8_is_lossy() {
        let t = parse_stream_title(b"StreamTitle='Caf\xe9 - X';").unwrap();
        assert_eq!(t, "Caf\u{fffd} - X");
    }

    #[test]
    fn split_table() {
        let cases = [
            ("A - B", Some("A"), "B"),
            ("A - B - C", Some("A"), "B - C"),
            ("Just a title", None, "Just a title"),
            ("A-B", None, "A-B"),
            ("  A - B  ", Some("A"), "B"),
            ("- B", None, "- B"),
        ];
        for (raw, artist, title) in cases {
            let np = now_playing(raw).unwrap();
            assert_eq!(np.artist.as_deref(), artist, "{raw}");
            assert_eq!(np.title, title, "{raw}");
            assert_eq!(np.raw, raw.trim());
        }
        assert_eq!(now_playing(""), None);
        assert_eq!(now_playing("   "), None);
    }

    /// Reads in awkward sizes so block boundaries fall inside reads.
    fn drain(mut r: impl Read, step: usize) -> Vec<u8> {
        let mut out = Vec::new();
        let mut buf = vec![0u8; step];
        loop {
            let n = r.read(&mut buf).unwrap();
            if n == 0 {
                return out;
            }
            out.extend_from_slice(&buf[..n]);
        }
    }

    /// Yields at most `n` bytes per read, splitting blocks across chunks.
    struct Dribble<R>(R, usize);
    impl<R: Read> Read for Dribble<R> {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            let n = out.len().min(self.1);
            self.0.read(&mut out[..n])
        }
    }

    #[test]
    fn strips_metadata_whatever_the_chunking() {
        let audio: Vec<u8> = (0..=255u8).cycle().take(1000).collect();
        let framed = frame(&audio, 64, &["A - One", "", "B - Two"]);
        for (chunk, step) in [(1, 7), (3, 64), (5, 1000), (1000, 1000), (17, 13)] {
            let mut titles = Vec::new();
            let r = IcyReader::new(Dribble(Cursor::new(framed.clone()), chunk), Some(64), |t| {
                titles.push(t)
            });
            let got = drain(r, step);
            assert_eq!(got, audio, "chunk {chunk} step {step}");
            assert_eq!(titles, ["A - One", "B - Two"], "chunk {chunk} step {step}");
        }
    }

    #[test]
    fn no_metaint_passes_bytes_through_untouched() {
        let data = b"StreamTitle='x';abc".to_vec();
        for metaint in [None, Some(0)] {
            let mut called = false;
            let r = IcyReader::new(Cursor::new(data.clone()), metaint, |_| called = true);
            assert_eq!(drain(r, 5), data);
            assert!(!called);
        }
    }

    #[test]
    fn truncated_block_ends_the_stream_cleanly() {
        let mut framed = frame(&[1u8; 64], 64, &["A - B"]);
        framed.truncate(64 + 1 + 4);
        let r = IcyReader::new(Cursor::new(framed), Some(64), |_| {});
        assert_eq!(drain(r, 100).len(), 64);
    }
}
