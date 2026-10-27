//! One Mirror's blocking network, ICY, decode, and PCM playback lifecycle.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::decode::{DecodeEnd, FADE_OUT_SLACK, Fade, decode_stream};
use super::icy::IcyReader;
use super::shell::{SinkFactory, Timeouts};
use super::source::StreamSource;
use super::{ErrorKind, reducer::Event};

/// Let reqwest report connection failures before classifying a silent response as unavailable.
const CONNECT_GRACE: Duration = Duration::from_millis(500);

/// Cancellation control for one running Mirror worker.
#[derive(Clone)]
pub(crate) struct WorkerHandle {
    cancel: Arc<AtomicBool>,
}

impl WorkerHandle {
    pub(super) fn new() -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// Spawn one Mirror lifecycle. `fade_snapshot` runs on the worker thread, at the same
/// point that the old shell worker read the setting. `guard` remains alive until it exits.
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn<G>(
    worker: WorkerHandle,
    url: Option<String>,
    attempt: u64,
    volume: Arc<AtomicU32>,
    timeouts: Timeouts,
    factory: SinkFactory,
    fade_snapshot: impl FnOnce() -> Option<Fade> + Send + 'static,
    notify: impl Fn(Event) + Send + Sync + 'static,
    guard: G,
) where
    G: Send + 'static,
{
    let worker_cancel = worker.cancel.clone();
    std::thread::spawn(move || {
        let _guard = guard;
        let fade = fade_snapshot();
        match url {
            Some(url) => stream_thread(
                &url,
                attempt,
                &worker_cancel,
                &volume,
                fade,
                timeouts,
                factory,
                notify,
            ),
            None => notify(Event::ConnectFailed {
                attempt,
                kind: ErrorKind::Unavailable,
            }),
        }
    });
}

enum Msg {
    /// HTTP status and `icy-metaint`, or `Err` when no response arrived.
    Head(Result<(u16, Option<usize>), ()>),
    Data(Vec<u8>),
    End,
}

/// Does the blocking HTTP work on its own thread so the decode thread can poll
/// `cancel` while the network is silent. It exits when the receiver is dropped.
fn spawn_pump(url: String, connect: Duration) -> mpsc::Receiver<Msg> {
    let (tx, rx) = mpsc::sync_channel(16);
    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(connect)
            .timeout(None) // live stream: no total timeout
            .build();
        let resp = client.and_then(|c| c.get(&url).header("Icy-MetaData", "1").send());
        let mut resp = match resp {
            Ok(r) => r,
            Err(_) => {
                let _ = tx.send(Msg::Head(Err(())));
                return;
            }
        };
        let metaint = resp
            .headers()
            .get("icy-metaint")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse().ok());
        if tx
            .send(Msg::Head(Ok((resp.status().as_u16(), metaint))))
            .is_err()
        {
            return;
        }
        let mut buf = [0u8; 8192];
        loop {
            let n = std::io::Read::read(&mut resp, &mut buf).unwrap_or(0);
            let msg = if n == 0 {
                Msg::End
            } else {
                Msg::Data(buf[..n].to_vec())
            };
            if tx.send(msg).is_err() || n == 0 {
                return;
            }
        }
    });
    rx
}

const POLL: Duration = Duration::from_millis(100);

/// Reads pump data, waking every `POLL` to check `cancel` and the time limits:
/// before `connected` is set the whole attempt gets `timeouts.connect`; afterwards
/// `timeouts.stall` without any bytes fails the read (a drop).
struct ChannelReader {
    rx: mpsc::Receiver<Msg>,
    cancel: Arc<AtomicBool>,
    /// While set, a cancel is not acted on until `cancel_grace` has passed, so
    /// the decode loop can finish its fade-out from the data already in flight.
    hold_cancel: Arc<AtomicBool>,
    cancel_grace: Duration,
    cancelled_at: Option<Instant>,
    connected: Arc<AtomicBool>,
    timeouts: Timeouts,
    started: Instant,
    last_data: Instant,
    pending: Vec<u8>,
    pos: usize,
}

impl ChannelReader {
    fn next(&mut self) -> std::io::Result<Option<Msg>> {
        loop {
            if self.cancel.load(Ordering::Relaxed) {
                let since = *self.cancelled_at.get_or_insert_with(Instant::now);
                if !self.hold_cancel.load(Ordering::Relaxed) || since.elapsed() >= self.cancel_grace
                {
                    return Err(std::io::Error::other("cancelled"));
                }
            }
            let timed_out = if self.connected.load(Ordering::Relaxed) {
                self.last_data.elapsed() >= self.timeouts.stall
            } else {
                self.started.elapsed() >= self.timeouts.connect + CONNECT_GRACE
            };
            if timed_out {
                return Err(std::io::ErrorKind::TimedOut.into());
            }
            match self.rx.recv_timeout(POLL) {
                Ok(m) => {
                    self.last_data = Instant::now();
                    return Ok(Some(m));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(None),
            }
        }
    }
}

impl std::io::Read for ChannelReader {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        while self.pos >= self.pending.len() {
            match self.next()? {
                Some(Msg::Data(d)) => {
                    self.pending = d;
                    self.pos = 0;
                }
                _ => return Ok(0),
            }
        }
        let n = out.len().min(self.pending.len() - self.pos);
        out[..n].copy_from_slice(&self.pending[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

/// One Mirror attempt: connect, decode, play until cancelled or the stream ends.
/// Cancel takes effect within ~`POLL` even while the server is silent, and
/// dropping the sink on return stops the audio. With `fade`, a cancel first
/// fades the audio out (see `decode_stream`).
#[allow(clippy::too_many_arguments)]
pub(super) fn stream_thread(
    url: &str,
    attempt: u64,
    cancel: &Arc<AtomicBool>,
    volume: &AtomicU32,
    fade: Option<Fade>,
    timeouts: Timeouts,
    factory: SinkFactory,
    notify: impl Fn(Event) + Send + Sync + 'static,
) {
    let notify = Arc::new(notify);
    let connect_failed = |kind| {
        if !cancel.load(Ordering::Relaxed) {
            notify(Event::ConnectFailed { attempt, kind });
        }
    };
    let connected = Arc::new(AtomicBool::new(false));
    let hold_cancel = Arc::new(AtomicBool::new(false));
    let mut reader = ChannelReader {
        rx: spawn_pump(url.to_string(), timeouts.connect),
        cancel: cancel.clone(),
        hold_cancel: hold_cancel.clone(),
        cancel_grace: fade.map_or(Duration::ZERO, |f| f.fade_out + FADE_OUT_SLACK),
        cancelled_at: None,
        connected: connected.clone(),
        timeouts,
        started: Instant::now(),
        last_data: Instant::now(),
        pending: Vec::new(),
        pos: 0,
    };
    let metaint = match reader.next() {
        Ok(Some(Msg::Head(Ok((status, metaint))))) if (200..300).contains(&status) => metaint,
        Ok(Some(Msg::Head(Ok(_)))) => return connect_failed(ErrorKind::Unavailable),
        Ok(Some(Msg::Head(Err(())))) => return connect_failed(ErrorKind::Offline),
        Ok(_) => return connect_failed(ErrorKind::Unavailable),
        Err(_) => return connect_failed(ErrorKind::Unavailable), // timed out (no-op if cancelled)
    };
    // ICY stripping sits between the network reader and symphonia, so the
    // decoder (and its pre-audio budget) only ever sees audio bytes.
    let on_title = {
        let notify = notify.clone();
        move |raw| notify(Event::TitleChanged { attempt, raw })
    };
    let end = decode_stream(
        StreamSource::new(IcyReader::new(reader, metaint, on_title)),
        move || factory(fade.is_some()),
        volume,
        cancel,
        fade,
        {
            let notify = notify.clone();
            move || {
                connected.store(true, Ordering::Relaxed);
                hold_cancel.store(fade.is_some(), Ordering::Relaxed);
                notify(Event::Connected { attempt })
            }
        },
    );
    if cancel.load(Ordering::Relaxed) {
        return;
    }
    match end {
        DecodeEnd::Cancelled => {}
        DecodeEnd::Unsupported => notify(Event::DecodeFailed { attempt }),
        DecodeEnd::Io { after_audio: false } => connect_failed(ErrorKind::Unavailable),
        DecodeEnd::Io { after_audio: true } => notify(Event::StreamDropped { attempt }),
    }
}
