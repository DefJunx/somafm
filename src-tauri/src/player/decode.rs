//! symphonia decode loop feeding a `PcmSink`. The sink is a trait so tests can
//! count samples instead of opening an audio device.

use std::io::{self, Read, Seek, SeekFrom};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

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

/// Fade timings (ADR 0003). The ramps run per sample inside the decode loop,
/// multiplying whatever the sink's own volume does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fade {
    /// Ramp from silence on the first audio of a new sink.
    pub fade_in: Duration,
    /// Ramp to silence on cancel, before the sink is dropped.
    pub fade_out: Duration,
}

impl Fade {
    pub const DEFAULT: Fade = Fade {
        fade_in: Duration::from_millis(600),
        fade_out: Duration::from_millis(250),
    };
}

/// Extra wall time a fade-out may take beyond its ramp (draining the sink backlog)
/// before the decode loop gives up and drops the sink anyway.
pub const FADE_OUT_SLACK: Duration = Duration::from_millis(200);

/// Packets the sink may queue ahead of the playhead (~25 ms each). Fades need a
/// short queue, or the ramp would only be heard that much later.
pub const BACKLOG_NORMAL: usize = 40;
pub const BACKLOG_FADE: usize = 6;

/// Linear gain ramp over interleaved samples: one gain per frame, shared by all
/// channels. A pure value, so it is tested without any decoding.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ramp {
    gain: f32,
    target: f32,
    step: f32,
    remaining: u64,
}

impl Ramp {
    pub fn new(gain: f32) -> Self {
        Self {
            gain,
            target: gain,
            step: 0.0,
            remaining: 0,
        }
    }

    /// Heads for `target` over `frames` frames, starting from the current gain
    /// (so an interrupted ramp continues smoothly). 0 frames jumps there at once.
    pub fn ramp_to(&mut self, target: f32, frames: u64) {
        self.target = target;
        self.remaining = frames;
        if frames == 0 {
            self.gain = target;
            self.step = 0.0;
        } else {
            self.step = (target - self.gain) / frames as f32;
        }
    }

    pub fn gain(&self) -> f32 {
        self.gain
    }

    /// True once the target gain is reached.
    pub fn settled(&self) -> bool {
        self.remaining == 0
    }

    /// Scales `samples` (interleaved, `channels` per frame) in place and advances.
    pub fn apply(&mut self, samples: &mut [f32], channels: usize) {
        for frame in samples.chunks_mut(channels.max(1)) {
            if self.gain != 1.0 {
                for s in frame {
                    *s *= self.gain;
                }
            }
            if self.remaining > 0 {
                self.remaining -= 1;
                self.gain = if self.remaining == 0 {
                    self.target
                } else {
                    self.gain + self.step
                };
            }
        }
    }
}

fn frames_in(d: Duration, rate: u32) -> u64 {
    (d.as_secs_f64() * f64::from(rate)) as u64
}

pub fn f32_to_bits(v: f32) -> u32 {
    v.to_bits()
}

/// Bytes allowed before the first decoded packet. A format symphonia cannot
/// recognise (e.g. ADTS with the MPEG-2 `0xFFF9` sync, as in SomaFM's HE-AAC
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
/// With `fade`, the first audio ramps in and a cancel ramps out (then drains the
/// sink backlog) before returning `Cancelled`; without it the samples are untouched.
/// `sink` is created lazily by `make_sink` after the first packet, so a missing
/// audio device only matters once there is something to play.
pub fn decode_stream<S: MediaSource + 'static>(
    source: S,
    make_sink: impl FnOnce() -> Result<Box<dyn PcmSink>, String>,
    volume: &AtomicU32,
    cancel: &AtomicBool,
    fade: Option<Fade>,
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
    let mut ramp = Ramp::new(1.0);
    let mut rate = 0;
    // Set once a cancel is seen with a live sink and fading on: (deadline, tail packets pushed).
    let mut fading_out: Option<(Instant, usize)> = None;
    let never = AtomicBool::new(false);

    loop {
        if cancel.load(Ordering::Relaxed) {
            let (Some(f), Some(_)) = (fade, &sink) else {
                return DecodeEnd::Cancelled;
            };
            match &mut fading_out {
                None => {
                    ramp.ramp_to(0.0, frames_in(f.fade_out, rate));
                    fading_out = Some((Instant::now() + f.fade_out + FADE_OUT_SLACK, 0));
                }
                // Silent and the backlog has played out, or the sink is stuck.
                Some((deadline, tail)) => {
                    if Instant::now() >= *deadline || *tail > BACKLOG_FADE {
                        return DecodeEnd::Cancelled;
                    }
                }
            }
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
            rate = spec.rate;
            if let Some(f) = fade {
                ramp = Ramp::new(0.0);
                ramp.ramp_to(1.0, frames_in(f.fade_in, rate));
            }
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
        let out = samples.samples_mut();
        if fade.is_some() {
            ramp.apply(out, spec.channels.count());
        }
        // During a fade-out the sink keeps its back-pressure (a set `cancel` would
        // make it return at once), so the tail is heard at playback pace.
        let push_cancel = if fading_out.is_some() { &never } else { cancel };
        sink.push(out, spec.channels.count() as u16, spec.rate, push_cancel);
        if let Some((_, tail)) = &mut fading_out
            && ramp.settled()
        {
            *tail += 1;
        }
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
    /// Queued packets allowed before `push` blocks.
    max_backlog: usize,
}

impl RodioSink {
    /// `low_latency` keeps the queue short so Fades are heard when they happen.
    pub fn open(low_latency: bool) -> Result<Self, String> {
        let device = rodio::DeviceSinkBuilder::open_default_sink().map_err(|e| e.to_string())?;
        let player = rodio::Player::connect_new(device.mixer());
        Ok(Self {
            _device: device,
            player,
            max_backlog: if low_latency {
                BACKLOG_FADE
            } else {
                BACKLOG_NORMAL
            },
        })
    }
}

impl PcmSink for RodioSink {
    fn push(&mut self, samples: &[f32], channels: u16, sample_rate: u32, cancel: &AtomicBool) {
        // Roughly one decoded packet (~25 ms) per queued source: cap the backlog.
        while self.player.len() > self.max_backlog {
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

    /// Keeps every pushed sample, plus when the sink was dropped.
    #[derive(Default, Clone)]
    pub struct Rec {
        pub samples: Arc<Mutex<Vec<f32>>>,
        /// Number of samples pushed when the sink was dropped.
        pub dropped_at: Arc<Mutex<Option<usize>>>,
    }

    pub struct RecSink(pub Rec);

    impl PcmSink for RecSink {
        fn push(&mut self, s: &[f32], _ch: u16, _rate: u32, _c: &AtomicBool) {
            self.0.samples.lock().unwrap().extend_from_slice(s);
        }
        fn set_volume(&mut self, _v: f32) {}
    }

    impl Drop for RecSink {
        fn drop(&mut self) {
            let n = self.0.samples.lock().unwrap().len();
            *self.0.dropped_at.lock().unwrap() = Some(n);
        }
    }

    fn decode_rec(
        fade: Option<Fade>,
        cancel: &AtomicBool,
        cancel_on_first: bool,
    ) -> (DecodeEnd, Rec) {
        let rec = Rec::default();
        let r = rec.clone();
        let end = decode_stream(
            StreamSource::new(Cursor::new(MP3.to_vec())),
            move || Ok(Box::new(RecSink(r)) as Box<dyn PcmSink>),
            &AtomicU32::new(f32_to_bits(1.0)),
            cancel,
            fade,
            || {
                if cancel_on_first {
                    cancel.store(true, Ordering::Relaxed);
                }
            },
        );
        (end, rec)
    }

    fn peak(s: &[f32]) -> f32 {
        s.iter().fold(0f32, |a, b| a.max(b.abs()))
    }

    #[test]
    fn ramp_up_and_down_hit_their_endpoints() {
        let mut r = Ramp::new(0.0);
        r.ramp_to(1.0, 4);
        let mut buf = [1.0f32; 8]; // 4 stereo frames
        r.apply(&mut buf, 2);
        assert_eq!(buf, [0.0, 0.0, 0.25, 0.25, 0.5, 0.5, 0.75, 0.75]);
        assert!(r.settled());
        assert_eq!(r.gain(), 1.0);
        r.ramp_to(0.0, 2);
        let mut buf = [1.0f32; 4];
        r.apply(&mut buf, 2);
        assert_eq!(buf, [1.0, 1.0, 0.5, 0.5]);
        assert_eq!(r.gain(), 0.0);
        let mut buf = [1.0f32; 2];
        r.apply(&mut buf, 2);
        assert_eq!(buf, [0.0, 0.0]);
    }

    #[test]
    fn interrupted_ramp_continues_from_the_current_gain() {
        let mut r = Ramp::new(0.0);
        r.ramp_to(1.0, 4);
        r.apply(&mut [1.0f32; 2], 1); // gains 0, 0.25 -> now 0.5
        assert_eq!(r.gain(), 0.5);
        r.ramp_to(0.0, 2);
        let mut buf = [1.0f32; 3];
        r.apply(&mut buf, 1);
        assert_eq!(buf, [0.5, 0.25, 0.0]);
    }

    #[test]
    fn ramp_is_continuous_across_buffer_boundaries() {
        let mut whole = Ramp::new(0.0);
        whole.ramp_to(1.0, 10);
        let mut a = [1.0f32; 12];
        whole.apply(&mut a, 2);
        let mut split = Ramp::new(0.0);
        split.ramp_to(1.0, 10);
        let mut b = [1.0f32; 12];
        let (x, y) = b.split_at_mut(6);
        split.apply(x, 2);
        split.apply(y, 2);
        assert_eq!(a, b);
    }

    #[test]
    fn zero_length_ramp_is_instant() {
        let mut r = Ramp::new(1.0);
        r.ramp_to(0.0, 0);
        assert!(r.settled());
        let mut buf = [1.0f32; 4];
        r.apply(&mut buf, 2);
        assert_eq!(buf, [0.0; 4]);
    }

    #[test]
    fn fade_off_leaves_samples_bit_identical() {
        let (_, plain) = decode_rec(None, &AtomicBool::new(false), false);
        let (_, instant) = decode_rec(
            Some(Fade {
                fade_in: Duration::ZERO,
                fade_out: Duration::ZERO,
            }),
            &AtomicBool::new(false),
            false,
        );
        let plain = plain.samples.lock().unwrap();
        let instant = instant.samples.lock().unwrap();
        assert!(peak(&plain) > 0.1);
        let bits = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
        assert_eq!(bits(&plain), bits(&instant));
    }

    #[test]
    fn fade_in_ramps_from_silence_to_full_level() {
        let fade = Fade {
            fade_in: Duration::from_millis(500),
            fade_out: Duration::ZERO,
        };
        let (_, rec) = decode_rec(Some(fade), &AtomicBool::new(false), false);
        let (_, plain) = decode_rec(None, &AtomicBool::new(false), false);
        let got = rec.samples.lock().unwrap();
        let want = plain.samples.lock().unwrap();
        assert_eq!(got.len(), want.len());
        // The first packet is encoder delay; 2400..3600 is early in the ramp.
        assert!(peak(&got[2400..3600]) < 0.3 * peak(&want), "starts quiet");
        assert!(peak(&got[..2]) == 0.0);
        let tail = got.len() - 20_000;
        assert_eq!(got[tail..], want[tail..], "full level after the ramp");
    }

    #[test]
    fn cancel_fades_out_to_silence_before_the_sink_drops() {
        let fade = Fade {
            fade_in: Duration::ZERO,
            fade_out: Duration::from_millis(100),
        };
        let (end, rec) = decode_rec(Some(fade), &AtomicBool::new(false), true);
        assert_eq!(end, DecodeEnd::Cancelled);
        let s = rec.samples.lock().unwrap();
        let dropped = rec.dropped_at.lock().unwrap().expect("sink dropped");
        assert_eq!(dropped, s.len(), "nothing pushed after the drop");
        assert_eq!(peak(&s[s.len() - 2..]), 0.0, "silent when dropped");
        // Falls: the peak of each quarter of the pushed audio never rises.
        let q = s.len() / 4;
        let peaks: Vec<f32> = s.chunks(q).take(4).map(peak).collect();
        assert!(peaks.windows(2).all(|w| w[1] <= w[0] + 1e-6), "{peaks:?}");
        assert!(s.len() > 2 * 4410, "faded over real audio");
    }

    #[test]
    fn cancel_without_fade_drops_the_sink_at_once() {
        let (end, rec) = decode_rec(None, &AtomicBool::new(false), true);
        assert_eq!(end, DecodeEnd::Cancelled);
        let s = rec.samples.lock().unwrap();
        assert!(s.len() < 5000, "one packet at most, got {}", s.len());
    }

    fn run(bytes: &[u8], counts: &Counts) -> (DecodeEnd, usize) {
        let c = counts.clone();
        let mut firsts = 0;
        let end = decode_stream(
            StreamSource::new(Cursor::new(bytes.to_vec())),
            move || Ok(Box::new(NullSink(c)) as Box<dyn PcmSink>),
            &AtomicU32::new(f32_to_bits(0.5)),
            &AtomicBool::new(false),
            None,
            || firsts += 1,
        );
        (end, firsts)
    }

    #[test]
    fn mp3_fixture_yields_pcm() {
        for (name, bytes) in [("mp3", MP3)] {
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
            None,
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
            None,
            || {},
        );
        assert_eq!(end, DecodeEnd::Io { after_audio: false });
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
        let junk = vec![0x42u8; 4096];
        let counts = Counts::default();
        let c = counts.clone();
        let end = decode_stream(
            StreamSource::new(Endless(junk, 0)),
            move || Ok(Box::new(NullSink(c)) as Box<dyn PcmSink>),
            &AtomicU32::new(0),
            &AtomicBool::new(false),
            None,
            || {},
        );
        assert_eq!(end, DecodeEnd::Unsupported);
        assert_eq!(*counts.samples.lock().unwrap(), 0);
    }
}
