//! Live spectrum analysis for the Visualizer: a pure `Spectrum` analyzer and a
//! `TapSink` that feeds it from `PcmSink::push` without altering the audio.

use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};

use super::decode::PcmSink;

pub const BANDS: usize = 32;
pub const FRAME: usize = 2048;
const MIN_HZ: f32 = 40.0;
const MAX_HZ: f32 = 16_000.0;
/// Bars span this many dB below full scale.
const FLOOR_DB: f32 = 60.0;
/// Fraction of the level kept per frame when a band falls (slow release).
const RELEASE: f32 = 0.85;

/// Latest smoothed band magnitudes (0..1), shared between the audio thread and a reader.
pub type Bands = Arc<Mutex<[f32; BANDS]>>;

pub struct Spectrum {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    frame: Vec<f32>,
    buf: Vec<Complex32>,
    scratch: Vec<Complex32>,
    /// Half-open FFT bin range per band, for `rate`.
    ranges: [(usize, usize); BANDS],
    rate: u32,
    bands: [f32; BANDS],
}

impl Spectrum {
    pub fn new() -> Self {
        let fft = FftPlanner::new().plan_fft_forward(FRAME);
        let scratch = vec![Complex32::default(); fft.get_inplace_scratch_len()];
        let window = (0..FRAME)
            .map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / FRAME as f32).cos())
            .collect();
        Self {
            fft,
            window,
            frame: Vec::with_capacity(FRAME),
            buf: vec![Complex32::default(); FRAME],
            scratch,
            ranges: [(0, 1); BANDS],
            rate: 0,
            bands: [0.0; BANDS],
        }
    }

    /// Feed interleaved samples; every completed 2048-sample mono frame updates the bands.
    pub fn push(&mut self, samples: &[f32], channels: u16, sample_rate: u32) {
        if channels == 0 || sample_rate == 0 {
            return;
        }
        if sample_rate != self.rate {
            self.rate = sample_rate;
            self.ranges = band_ranges(sample_rate);
            self.frame.clear();
        }
        let ch = channels as usize;
        for chunk in samples.chunks_exact(ch) {
            self.frame.push(chunk.iter().sum::<f32>() / ch as f32);
            if self.frame.len() == FRAME {
                self.analyze();
                self.frame.clear();
            }
        }
    }

    /// Drops the partial frame and the smoothed levels.
    pub fn reset(&mut self) {
        self.frame.clear();
        self.bands = [0.0; BANDS];
    }

    /// Smoothed band magnitudes in 0..1, lowest frequency first.
    pub fn bands(&self) -> &[f32; BANDS] {
        &self.bands
    }

    fn analyze(&mut self) {
        for (i, c) in self.buf.iter_mut().enumerate() {
            *c = Complex32::new(self.frame[i] * self.window[i], 0.0);
        }
        self.fft
            .process_with_scratch(&mut self.buf, &mut self.scratch);
        // A full-scale sine peaks at FRAME / 4 after a Hann window.
        let norm = 4.0 / FRAME as f32;
        for (band, &(lo, hi)) in self.bands.iter_mut().zip(&self.ranges) {
            let peak = self.buf[lo..hi].iter().fold(0f32, |m, c| m.max(c.norm()));
            let db = 20.0 * (peak * norm).max(1e-9).log10();
            let level = ((db + FLOOR_DB) / FLOOR_DB).clamp(0.0, 1.0);
            *band = if level > *band {
                level
            } else {
                (*band * RELEASE).max(level)
            };
        }
    }
}

impl Default for Spectrum {
    fn default() -> Self {
        Self::new()
    }
}

/// Log-spaced band edges between `MIN_HZ` and `MAX_HZ` (kept below Nyquist), as FFT bins.
/// Every band covers at least one bin.
fn band_ranges(rate: u32) -> [(usize, usize); BANDS] {
    let bin_hz = rate as f32 / FRAME as f32;
    let max_hz = MAX_HZ.min(rate as f32 / 2.0 * 0.95);
    let ratio = (max_hz / MIN_HZ).powf(1.0 / BANDS as f32);
    let mut out = [(1, 2); BANDS];
    for (i, r) in out.iter_mut().enumerate() {
        let lo_hz = MIN_HZ * ratio.powi(i as i32);
        let lo = ((lo_hz / bin_hz).round() as usize).max(1);
        let hi = ((lo_hz * ratio / bin_hz).round() as usize).max(lo + 1);
        *r = (lo, hi.min(FRAME / 2));
    }
    out
}

/// Wraps a sink: forwards every push unchanged and analyzes the same stream.
pub struct TapSink<S> {
    inner: S,
    spectrum: Spectrum,
    shared: Bands,
    /// When set and false, pushes are forwarded without any analysis.
    enabled: Option<Arc<AtomicBool>>,
}

impl<S: PcmSink> TapSink<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            spectrum: Spectrum::new(),
            shared: Arc::new(Mutex::new([0.0; BANDS])),
            enabled: None,
        }
    }

    /// Like `new`, but analyzes only while `enabled` is true.
    pub fn gated(inner: S, enabled: Arc<AtomicBool>) -> Self {
        Self {
            enabled: Some(enabled),
            ..Self::new(inner)
        }
    }

    /// Handle a reader (e.g. a throttled emitter) polls for the latest bands.
    pub fn bands(&self) -> Bands {
        self.shared.clone()
    }
}

impl<S: PcmSink> PcmSink for TapSink<S> {
    fn push(&mut self, samples: &[f32], channels: u16, sample_rate: u32, cancel: &AtomicBool) {
        self.inner.push(samples, channels, sample_rate, cancel);
        if self
            .enabled
            .as_ref()
            .is_some_and(|e| !e.load(Ordering::Relaxed))
        {
            self.spectrum.reset(); // re-enabling starts from silence, not stale levels
            return;
        }
        self.spectrum.push(samples, channels, sample_rate);
        if let Ok(mut b) = self.shared.lock() {
            *b = *self.spectrum.bands();
        }
    }

    fn set_volume(&mut self, volume: f32) {
        self.inner.set_volume(volume);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::decode::tests::{Counts, NullSink};

    fn sine(hz: f32, rate: u32, channels: u16, frames: usize) -> Vec<f32> {
        (0..frames)
            .flat_map(|i| {
                let s = 0.8 * (2.0 * PI * hz * i as f32 / rate as f32).sin();
                std::iter::repeat_n(s, channels as usize)
            })
            .collect()
    }

    fn argmax(b: &[f32; BANDS]) -> usize {
        (0..BANDS).fold(0, |m, i| if b[i] > b[m] { i } else { m })
    }

    fn band_of(hz: f32, rate: u32) -> usize {
        let bin = (hz / (rate as f32 / FRAME as f32)).round() as usize;
        band_ranges(rate)
            .iter()
            .position(|&(lo, hi)| (lo..hi).contains(&bin))
            .unwrap()
    }

    #[test]
    fn tone_band_is_max_for_mono_stereo_and_rates() {
        for rate in [44_100, 48_000] {
            for ch in [1u16, 2] {
                for hz in [200.0, 1000.0, 5000.0] {
                    let mut s = Spectrum::new();
                    s.push(&sine(hz, rate, ch, FRAME * 3), ch, rate);
                    let b = s.bands();
                    assert_eq!(argmax(b), band_of(hz, rate), "{hz} Hz {ch}ch {rate}");
                    assert!(b.iter().all(|v| (0.0..=1.0).contains(v)));
                    assert!(b[argmax(b)] > 0.5);
                }
            }
        }
    }

    #[test]
    fn silence_is_all_zeros() {
        let mut s = Spectrum::new();
        s.push(&vec![0.0; FRAME * 2], 2, 44_100);
        assert!(s.bands().iter().all(|&v| v == 0.0));
    }

    #[test]
    fn decays_monotonically_after_tone_stops() {
        let mut s = Spectrum::new();
        s.push(&sine(1000.0, 44_100, 1, FRAME * 3), 1, 44_100);
        let peak = band_of(1000.0, 44_100);
        let mut prev = s.bands()[peak];
        assert!(prev > 0.5);
        for _ in 0..10 {
            s.push(&vec![0.0; FRAME], 1, 44_100);
            let now = s.bands()[peak];
            assert!(now < prev, "{now} !< {prev}");
            prev = now;
        }
    }

    #[test]
    fn tap_forwards_bit_identical_and_publishes_bands() {
        let counts = Counts::default();
        let mut tap = TapSink::new(NullSink(counts.clone()));
        let bands = tap.bands();
        let data = sine(1000.0, 48_000, 2, FRAME * 2);
        tap.push(&data, 2, 48_000, &AtomicBool::new(false));
        tap.set_volume(0.25);
        assert_eq!(*counts.samples.lock().unwrap(), data.len());
        assert_eq!(*counts.channels.lock().unwrap(), 2);
        assert_eq!(*counts.rate.lock().unwrap(), 48_000);
        assert_eq!(*counts.volume.lock().unwrap(), Some(0.25));
        let peak = data.iter().fold(0f32, |a, b| a.max(b.abs()));
        assert_eq!(counts.peak.lock().unwrap().to_bits(), peak.to_bits());
        assert_eq!(argmax(&bands.lock().unwrap()), band_of(1000.0, 48_000));
    }

    #[test]
    fn gated_off_forwards_without_analysis() {
        let counts = Counts::default();
        let enabled = Arc::new(AtomicBool::new(false));
        let mut tap = TapSink::gated(NullSink(counts.clone()), enabled.clone());
        let bands = tap.bands();
        let data = sine(1000.0, 48_000, 2, FRAME * 2);
        let cancel = AtomicBool::new(false);
        tap.push(&data, 2, 48_000, &cancel);
        assert_eq!(*counts.samples.lock().unwrap(), data.len());
        assert_eq!(*bands.lock().unwrap(), [0.0; BANDS]);
        enabled.store(true, Ordering::Relaxed);
        tap.push(&data, 2, 48_000, &cancel);
        assert!(bands.lock().unwrap().iter().any(|&v| v > 0.5));
    }
}
