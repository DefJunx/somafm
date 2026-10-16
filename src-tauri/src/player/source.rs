//! Adapter turning a blocking byte stream (the HTTP response body) into a
//! symphonia `MediaSource`. ICY metadata stripping lands here in slice 04.

use std::io::{self, Read, Seek, SeekFrom};
use std::sync::Mutex;

use symphonia::core::io::MediaSource;

pub struct StreamSource<R> {
    // The Mutex only makes the wrapper `Sync`; access is always through `&mut self`.
    inner: Mutex<R>,
}

impl<R: Read + Send> StreamSource<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner: Mutex::new(inner),
        }
    }
}

impl<R: Read + Send> Read for StreamSource<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
            .read(buf)
    }
}

impl<R> Seek for StreamSource<R> {
    fn seek(&mut self, _pos: SeekFrom) -> io::Result<u64> {
        Err(io::Error::new(io::ErrorKind::Unsupported, "live stream"))
    }
}

impl<R: Read + Send> MediaSource for StreamSource<R> {
    fn is_seekable(&self) -> bool {
        false
    }
    fn byte_len(&self) -> Option<u64> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_through_and_is_unseekable() {
        let mut s = StreamSource::new(io::Cursor::new(b"abc".to_vec()));
        let mut out = Vec::new();
        s.read_to_end(&mut out).unwrap();
        assert_eq!(out, b"abc");
        assert!(!s.is_seekable());
        assert_eq!(s.byte_len(), None);
        assert!(s.seek(SeekFrom::Start(0)).is_err());
    }
}
