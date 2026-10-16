//! symphonia decode loop feeding a `PcmSink`. The sink is a trait so tests can
//! count samples instead of opening an audio device.

use std::io::{self, Read, Seek, SeekFrom};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Destination for decoded interleaved f32 PCM.
pub trait PcmSink {
    /// Queue samples; may block for back-pressure but must return once `cancel` is set.
    fn push(&mut self, samples: &[f32], channels: u16, sample_rate: u32, cancel: &AtomicBool);
    fn set_volume(&mut self, volume: f32);
}

#[derive(Debug, PartialEq)]
pub enum DecodeEnd {
    Cancelled,
    /// The container/codec could not be understood (before or during playback).
    Unsupported,
    /// The byte stream failed or ended. `after_audio` tells whether any audio was produced.
    Io {
        after_audio: bool,
    },
}

pub fn f32_to_bits(v: f32) -> u32 {
    v.to_bits()
}

/// Bytes allowed before the first decoded packet. A format symphonia cannot
/// recognise (e.g. ADTS with the MPEG-2 `0xFFF9` sync, used by SomaFM's HE-AAC
/// "aac" streams) makes the probe scan an endless live stream forever.
pub const PRE_AUDIO_BUDGET: usize = 128 * 1024;

struct Budgeted<S> {
    inner: S,
    remaining: usize,
    armed: Arc<AtomicBool>,
    tripped: Arc<AtomicBool>,
}

impl<S: Read> Read for Budgeted<S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.armed.load(Ordering::Relaxed) {
            if self.remaining == 0 {
                self.tripped.store(true, Ordering::Relaxed);
                return Err(io::Error::new(io::ErrorKind::InvalidData, "probe budget"));
            }
            let max = buf.len().min(self.remaining);
            let n = self.inner.read(&mut buf[..max])?;
            self.remaining -= n;
            return Ok(n);
        }
        self.inner.read(buf)
    }
}

impl<S: Seek> Seek for Budgeted<S> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl<S: MediaSource> MediaSource for Budgeted<S> {
    fn is_seekable(&self) -> bool {
        self.inner.is_seekable()
    }
    fn byte_len(&self) -> Option<u64> {
        self.inner.byte_len()
    }
}

/// Decode `source` into `sink` until cancelled, ended or failed. `on_first_audio`
/// is called once, after the first decoded packet has been pushed to the sink.
/// `sink` is created lazily by `make_sink` after the first packet, so a missing
/// audio device only matters once there is something to play.
pub fn decode_stream<S: MediaSource + 'static>(
    source: S,
    make_sink: impl FnOnce() -> Result<Box<dyn PcmSink>, String>,
    volume: &AtomicU32,
    cancel: &AtomicBool,
    mut on_first_audio: impl FnMut(),
) -> DecodeEnd {
    let armed = Arc::new(AtomicBool::new(true));
    let tripped = Arc::new(AtomicBool::new(false));
    let source = Budgeted {
        inner: source,
        remaining: PRE_AUDIO_BUDGET,
        armed: armed.clone(),
        tripped: tripped.clone(),
    };
    let unsupported_if_tripped = |end: DecodeEnd| {
        if tripped.load(Ordering::Relaxed) {
            DecodeEnd::Unsupported
        } else {
            end
        }
    };
    let mss = MediaSourceStream::new(Box::new(source), Default::default());
    let probed = match symphonia::default::get_probe().format(
        &Hint::new(),
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    ) {
        Ok(p) => p,
        Err(e) => return unsupported_if_tripped(map_err(e, false)),
    };
    let mut format = probed.format;
    let Some(track) = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
    else {
        return DecodeEnd::Unsupported;
    };
    let track_id = track.id;
    let mut decoder = match symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
    {
        Ok(d) => d,
        Err(_) => return DecodeEnd::Unsupported,
    };

    let mut make_sink = Some(make_sink);
    let mut sink: Option<Box<dyn PcmSink>> = None;
    let mut last_volume = u32::MAX;
    let mut buf: Option<SampleBuffer<f32>> = None;

    loop {
        if cancel.load(Ordering::Relaxed) {
            return DecodeEnd::Cancelled;
        }
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(e) => return unsupported_if_tripped(map_err(e, sink.is_some())),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            // A corrupt packet is skipped, not fatal.
            Err(SymError::DecodeError(_)) => continue,
            Err(e) => return map_err(e, sink.is_some()),
        };
        let spec = *decoded.spec();
        let samples =
            buf.get_or_insert_with(|| SampleBuffer::<f32>::new(decoded.capacity() as u64, spec));
        if samples.capacity() < decoded.capacity() * spec.channels.count() {
            *samples = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        }
        samples.copy_interleaved_ref(decoded);

        let first = sink.is_none();
        if first {
            armed.store(false, Ordering::Relaxed);
        }
        if first {
            match (make_sink.take().expect("sink factory used once"))() {
                Ok(s) => sink = Some(s),
                Err(_) => return DecodeEnd::Io { after_audio: false },
            }
        }
        let sink = sink.as_mut().expect("sink exists");
        let v = volume.load(Ordering::Relaxed);
        if v != last_volume {
            sink.set_volume(f32::from_bits(v));
            last_volume = v;
        }
        sink.push(
            samples.samples(),
            spec.channels.count() as u16,
            spec.rate,
            cancel,
        );
        if first {
            on_first_audio();
        }
    }
}

fn map_err(e: SymError, after_audio: bool) -> DecodeEnd {
    match e {
        SymError::IoError(_) => DecodeEnd::Io { after_audio },
        SymError::ResetRequired => DecodeEnd::Io { after_audio },
        _ => DecodeEnd::Unsupported,
    }
}

/// rodio-backed sink: a `Player` on the default output device. Not `Send`
/// (cpal streams may not be), so it is created on the thread that uses it.
pub struct RodioSink {
    _device: rodio::stream::MixerDeviceSink,
    player: rodio::Player,
}

impl RodioSink {
    pub fn open() -> Result<Self, String> {
        let device = rodio::DeviceSinkBuilder::open_default_sink().map_err(|e| e.to_string())?;
        let player = rodio::Player::connect_new(device.mixer());
        Ok(Self {
            _device: device,
            player,
        })
    }
}

impl PcmSink for RodioSink {
    fn push(&mut self, samples: &[f32], channels: u16, sample_rate: u32, cancel: &AtomicBool) {
        // Roughly one decoded packet (~25 ms) per queued source: cap the backlog near 1 s.
        while self.player.len() > 40 {
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        if let (Some(c), Some(r)) = (
            std::num::NonZero::new(channels),
            std::num::NonZero::new(sample_rate),
        ) {
            self.player
                .append(rodio::buffer::SamplesBuffer::new(c, r, samples.to_vec()));
        }
    }

    fn set_volume(&mut self, volume: f32) {
        self.player.set_volume(volume);
    }
}

impl Drop for RodioSink {
    fn drop(&mut self) {
        self.player.stop();
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::player::source::StreamSource;
    use std::io::Cursor;
    use std::sync::{Arc, Mutex};

    pub const MP3: &[u8] = include_bytes!("../../tests/fixtures/tone.mp3");
    pub const AAC: &[u8] = include_bytes!("../../tests/fixtures/tone.aac");

    #[derive(Default, Clone)]
    pub struct Counts {
        pub samples: Arc<Mutex<usize>>,
        pub channels: Arc<Mutex<u16>>,
        pub rate: Arc<Mutex<u32>>,
        pub volume: Arc<Mutex<Option<f32>>>,
        pub peak: Arc<Mutex<f32>>,
    }

    pub struct NullSink(pub Counts);

    impl PcmSink for NullSink {
        fn push(&mut self, s: &[f32], channels: u16, rate: u32, _c: &AtomicBool) {
            *self.0.samples.lock().unwrap() += s.len();
            *self.0.channels.lock().unwrap() = channels;
            *self.0.rate.lock().unwrap() = rate;
            let p = s.iter().fold(0f32, |a, b| a.max(b.abs()));
            let mut peak = self.0.peak.lock().unwrap();
            *peak = peak.max(p);
        }
        fn set_volume(&mut self, v: f32) {
            *self.0.volume.lock().unwrap() = Some(v);
        }
    }

    fn run(bytes: &[u8], counts: &Counts) -> (DecodeEnd, usize) {
        let c = counts.clone();
        let mut firsts = 0;
        let end = decode_stream(
            StreamSource::new(Cursor::new(bytes.to_vec())),
            move || Ok(Box::new(NullSink(c)) as Box<dyn PcmSink>),
            &AtomicU32::new(f32_to_bits(0.5)),
            &AtomicBool::new(false),
            || firsts += 1,
        );
        (end, firsts)
    }

    #[test]
    fn mp3_and_aac_fixtures_yield_pcm() {
        for (name, bytes) in [("mp3", MP3), ("aac", AAC)] {
            let counts = Counts::default();
            let (end, firsts) = run(bytes, &counts);
            assert_eq!(end, DecodeEnd::Io { after_audio: true }, "{name}");
            assert_eq!(firsts, 1, "{name}");
            let n = *counts.samples.lock().unwrap();
            // ~2 s of stereo (the encoder may resample to 22.05 kHz): >1 s of PCM.
            assert!(n > 80_000, "{name}: only {n} samples");
            assert_eq!(*counts.channels.lock().unwrap(), 2, "{name}");
            let rate = *counts.rate.lock().unwrap();
            assert!((8_000..=48_000).contains(&rate), "{name}: rate {rate}");
            assert_eq!(*counts.volume.lock().unwrap(), Some(0.5), "{name}");
            assert!(*counts.peak.lock().unwrap() > 0.1, "{name}: silent output");
        }
    }

    #[test]
    fn garbage_is_unsupported_and_empty_is_io() {
        let counts = Counts::default();
        let (end, firsts) = run(&[0x42; 8192], &counts);
        assert_eq!(end, DecodeEnd::Unsupported);
        assert_eq!(firsts, 0);
        let (end, _) = run(&[], &counts);
        assert!(matches!(
            end,
            DecodeEnd::Io { after_audio: false } | DecodeEnd::Unsupported
        ));
    }

    #[test]
    fn cancel_stops_decoding() {
        let counts = Counts::default();
        let c = counts.clone();
        let cancel = AtomicBool::new(true);
        let end = decode_stream(
            StreamSource::new(Cursor::new(MP3.to_vec())),
            move || Ok(Box::new(NullSink(c)) as Box<dyn PcmSink>),
            &AtomicU32::new(0),
            &cancel,
            || {},
        );
        assert_eq!(end, DecodeEnd::Cancelled);
        assert_eq!(*counts.samples.lock().unwrap(), 0);
    }

    #[test]
    fn sink_failure_is_reported_before_audio() {
        let end = decode_stream(
            StreamSource::new(Cursor::new(MP3.to_vec())),
            || Err("no device".into()),
            &AtomicU32::new(0),
            &AtomicBool::new(false),
            || {},
        );
        assert_eq!(end, DecodeEnd::Io { after_audio: false });
    }

    /// Rewrites every ADTS header's ID bit (0xFFF1 -> 0xFFF9, MPEG-2 style), which
    /// symphonia's ADTS reader does not recognise.
    pub fn mpeg2_adts(src: &[u8]) -> Vec<u8> {
        let mut out = src.to_vec();
        let mut i = 0;
        while i + 7 <= out.len() && out[i] == 0xff && out[i + 1] & 0xf0 == 0xf0 {
            let len = ((out[i + 3] as usize & 0x03) << 11)
                | ((out[i + 4] as usize) << 3)
                | (out[i + 5] as usize >> 5);
            out[i + 1] |= 0x08;
            i += len.max(7);
        }
        out
    }

    /// Endless repetition of `data`, like a live stream.
    struct Endless(Vec<u8>, usize);
    impl Read for Endless {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            for b in buf.iter_mut() {
                *b = self.0[self.1 % self.0.len()];
                self.1 += 1;
            }
            Ok(buf.len())
        }
    }

    #[test]
    fn unrecognised_endless_stream_ends_unsupported_instead_of_hanging() {
        let patched = mpeg2_adts(AAC);
        assert_eq!(patched[1], 0xf9);
        let counts = Counts::default();
        let c = counts.clone();
        let end = decode_stream(
            StreamSource::new(Endless(patched, 0)),
            move || Ok(Box::new(NullSink(c)) as Box<dyn PcmSink>),
            &AtomicU32::new(0),
            &AtomicBool::new(false),
            || {},
        );
        assert_eq!(end, DecodeEnd::Unsupported);
        assert_eq!(*counts.samples.lock().unwrap(), 0);
    }
}
