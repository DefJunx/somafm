//! Visualizer plumbing: decides when spectrum events may flow, taps the sink, and
//! emits the latest bands at about 30 fps while that holds.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::runtime::Handle;

use super::decode::PcmSink;
use super::spectrum::{BANDS, Bands, TapSink};

/// Event name carrying the 32 band values (0..1, lowest frequency first).
pub const SPECTRUM_EVENT: &str = "spectrum";
/// About 30 fps.
const TICK: Duration = Duration::from_millis(33);

pub type EmitBands = Arc<dyn Fn(&[f32; BANDS]) + Send + Sync>;

/// Events and FFT work happen only when something can show them.
pub fn should_emit(
    active: bool,
    playing: bool,
    show_visualizer: bool,
    window_visible: bool,
) -> bool {
    active && playing && show_visualizer && window_visible
}

pub struct Visualizer {
    rt: Handle,
    /// The Player screen is open (`set_visualizer_active`).
    active: AtomicBool,
    playing: AtomicBool,
    show: AtomicBool,
    window_visible: AtomicBool,
    /// `should_emit` of the four inputs; shared with the taps so they skip the FFT.
    enabled: Arc<AtomicBool>,
    ticking: AtomicBool,
    bands: Mutex<Option<Bands>>,
    emit: Mutex<Option<EmitBands>>,
}

impl Visualizer {
    pub fn new(rt: Handle) -> Arc<Self> {
        Arc::new(Self {
            rt,
            active: AtomicBool::new(false),
            playing: AtomicBool::new(false),
            show: AtomicBool::new(true),
            window_visible: AtomicBool::new(true),
            enabled: Arc::new(AtomicBool::new(false)),
            ticking: AtomicBool::new(false),
            bands: Mutex::new(None),
            emit: Mutex::new(None),
        })
    }

    pub fn set_emit(&self, emit: impl Fn(&[f32; BANDS]) + Send + Sync + 'static) {
        *self.emit.lock().unwrap() = Some(Arc::new(emit));
    }

    pub fn set_active(self: &Arc<Self>, on: bool) {
        self.active.store(on, Ordering::Relaxed);
        self.refresh();
    }

    pub fn set_playing(self: &Arc<Self>, on: bool) {
        self.playing.store(on, Ordering::Relaxed);
        self.refresh();
    }

    pub fn set_show(self: &Arc<Self>, on: bool) {
        self.show.store(on, Ordering::Relaxed);
        self.refresh();
    }

    pub fn set_window_visible(self: &Arc<Self>, on: bool) {
        self.window_visible.store(on, Ordering::Relaxed);
        self.refresh();
    }

    /// Whether new sinks get a tap: the setting, read when a decode thread starts.
    pub fn show(&self) -> bool {
        self.show.load(Ordering::Relaxed)
    }

    /// Wraps `sink` in a tap gated by this Visualizer; its bands feed the emitter.
    pub fn tap<S: PcmSink>(&self, sink: S) -> TapSink<S> {
        let tap = TapSink::gated(sink, self.enabled.clone());
        *self.bands.lock().unwrap() = Some(tap.bands());
        tap
    }

    #[cfg(test)]
    pub fn has_tap(&self) -> bool {
        self.bands.lock().unwrap().is_some()
    }

    fn refresh(self: &Arc<Self>) {
        let on = should_emit(
            self.active.load(Ordering::Relaxed),
            self.playing.load(Ordering::Relaxed),
            self.show.load(Ordering::Relaxed),
            self.window_visible.load(Ordering::Relaxed),
        );
        self.enabled.store(on, Ordering::Relaxed);
        if on && !self.ticking.swap(true, Ordering::AcqRel) {
            let this = self.clone();
            self.rt.spawn(async move { this.run().await });
        }
    }

    /// Emits until gated off. The loop only exists while events can flow.
    async fn run(self: Arc<Self>) {
        loop {
            tokio::time::sleep(TICK).await;
            if !self.enabled.load(Ordering::Relaxed) {
                self.ticking.store(false, Ordering::Release);
                // Re-enabled between the check and the store: carry on if nobody else took over.
                if self.enabled.load(Ordering::Relaxed)
                    && !self.ticking.swap(true, Ordering::AcqRel)
                {
                    continue;
                }
                return;
            }
            let bands = self
                .bands
                .lock()
                .unwrap()
                .as_ref()
                .and_then(|b| b.lock().ok().map(|b| *b));
            let emit = self.emit.lock().unwrap().clone();
            if let (Some(bands), Some(emit)) = (bands, emit) {
                emit(&bands);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_only_when_all_four_inputs_hold() {
        for bits in 0..16u8 {
            let b = |i: u8| bits & (1 << i) != 0;
            assert_eq!(
                should_emit(b(0), b(1), b(2), b(3)),
                bits == 0b1111,
                "{bits:04b}"
            );
        }
    }
}
