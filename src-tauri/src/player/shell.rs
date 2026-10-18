//! Imperative shell: owns the `Core`, runs the reducer, and executes its effects
//! (resolve `.pls`, stream + decode on a dedicated thread, apply volume, emit).
//! Playback never touches the webview, so it survives the window being hidden.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::runtime::Handle;
use tokio::task::JoinHandle;

use super::decode::{DecodeEnd, PcmSink, decode_stream, f32_to_bits};
use super::icy::IcyReader;
use super::pls::{pick_playlist, resolve_mirrors};
use super::reducer::{Core, Effect, Event, reduce};
use super::source::StreamSource;
use super::{ErrorKind, PlayerState};
use crate::feed::Feed;

/// Creates the audio sink on the decode thread (cpal streams may not be `Send`).
pub type SinkFactory = Arc<dyn Fn() -> Result<Box<dyn PcmSink>, String> + Send + Sync>;
type Emit = Box<dyn Fn(PlayerState) + Send + Sync>;

/// Network time limits (spec 5.5). Injectable so tests need not wait real seconds.
#[derive(Clone, Copy, Debug)]
pub struct Timeouts {
    /// Per Mirror: TCP + TLS + first audio bytes.
    pub connect: Duration,
    /// While playing: no bytes for this long is a drop.
    pub stall: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(10),
            stall: Duration::from_secs(15),
        }
    }
}

/// The reqwest connect timeout fires at `connect`; the reader's own limit waits a
/// little longer so a network-level failure is reported by the pump (as `Offline`),
/// not pre-empted by the reader (which means "answered but silent").
const CONNECT_GRACE: Duration = Duration::from_millis(500);

#[derive(Default)]
struct ResolvedMirrors {
    attempt: u64,
    urls: Vec<String>,
}

#[derive(Default)]
struct Active {
    resolve: Option<JoinHandle<()>>,
    timer: Option<JoinHandle<()>>,
    cancel: Option<Arc<AtomicBool>>,
}

struct Inner {
    core: Mutex<Core>,
    feed: Arc<Feed>,
    http: reqwest::Client,
    cache_dir: PathBuf,
    rt: Handle,
    emit: Emit,
    sink_factory: SinkFactory,
    timeouts: Timeouts,
    volume: Arc<AtomicU32>,
    /// Mirrors resolved for connection attempt `attempt`.
    mirrors: Mutex<ResolvedMirrors>,
    active: Mutex<Active>,
}

#[derive(Clone)]
pub struct Player {
    inner: Arc<Inner>,
}

impl Player {
    pub fn new(
        feed: Arc<Feed>,
        cache_dir: &Path,
        rt: Handle,
        sink_factory: SinkFactory,
        emit: impl Fn(PlayerState) + Send + Sync + 'static,
    ) -> Self {
        Self::with_timeouts(feed, cache_dir, rt, sink_factory, Timeouts::default(), emit)
    }

    pub fn with_timeouts(
        feed: Arc<Feed>,
        cache_dir: &Path,
        rt: Handle,
        sink_factory: SinkFactory,
        timeouts: Timeouts,
        emit: impl Fn(PlayerState) + Send + Sync + 'static,
    ) -> Self {
        let state = PlayerState::default();
        Self {
            inner: Arc::new(Inner {
                volume: Arc::new(AtomicU32::new(f32_to_bits(state.volume))),
                core: Mutex::new(Core::new(state)),
                feed,
                http: reqwest::Client::builder()
                    .timeout(Duration::from_secs(15))
                    .build()
                    .expect("http client"),
                cache_dir: cache_dir.to_path_buf(),
                rt,
                emit: Box::new(emit),
                sink_factory,
                timeouts,
                mirrors: Mutex::new(ResolvedMirrors::default()),
                active: Mutex::new(Active::default()),
            }),
        }
    }

    pub fn state(&self) -> PlayerState {
        self.inner.core.lock().unwrap().state.clone()
    }

    /// Launch restore: loads `station_id` as `stopped` (Paused) and applies `volume`.
    /// Nothing connects and nothing is emitted; call before the webview asks for state.
    pub fn restore(&self, station_id: Option<String>, volume: f32) {
        let volume = volume.clamp(0.0, 1.0);
        let mut core = self.inner.core.lock().unwrap();
        core.state.station_id = station_id;
        core.state.volume = volume;
        self.inner
            .volume
            .store(f32_to_bits(volume), Ordering::Relaxed);
    }

    /// Err only for an unknown station id (a programmer error).
    pub fn play(&self, station_id: &str) -> Result<(), String> {
        if self.inner.feed.station(station_id).is_none() {
            return Err(format!("unknown station `{station_id}`"));
        }
        self.inner.dispatch(Event::PlayRequested {
            station_id: station_id.to_string(),
        });
        Ok(())
    }

    pub fn stop(&self) {
        self.inner.dispatch(Event::StopRequested);
    }

    pub fn set_volume(&self, volume: f32) {
        self.inner.dispatch(Event::SetVolume(volume));
    }
}

impl Inner {
    // Effects run under the core lock: they must never re-enter `dispatch` synchronously.
    /// Reduce under the core lock and execute the effects before releasing it,
    /// so snapshots are emitted in state order.
    fn dispatch(self: &Arc<Self>, event: Event) {
        let mut guard = self.core.lock().unwrap();
        let (next, effects) = reduce(guard.clone(), event, Instant::now());
        *guard = next;
        for effect in effects {
            self.execute(effect, &guard.state);
        }
    }

    fn execute(self: &Arc<Self>, effect: Effect, state: &PlayerState) {
        match effect {
            Effect::EmitSnapshot => (self.emit)(state.clone()),
            Effect::ApplyVolume(v) => self.volume.store(f32_to_bits(v), Ordering::Relaxed),
            Effect::Disconnect => {
                let mut active = self.active.lock().unwrap();
                if let Some(h) = active.resolve.take() {
                    h.abort();
                }
                if let Some(h) = active.timer.take() {
                    h.abort();
                }
                if let Some(c) = active.cancel.take() {
                    c.store(true, Ordering::Relaxed);
                }
            }
            Effect::ArmTimer { attempt, at } => {
                let this = self.clone();
                let handle = self.rt.spawn(async move {
                    tokio::time::sleep_until(tokio::time::Instant::from_std(at)).await;
                    this.dispatch(Event::TimerFired { attempt });
                });
                if let Some(old) = self.active.lock().unwrap().timer.replace(handle) {
                    old.abort();
                }
            }
            Effect::Resolve {
                attempt,
                station_id,
            } => {
                let this = self.clone();
                let handle = self.rt.spawn(async move {
                    let event = match this.resolve(attempt, &station_id).await {
                        Ok(count) => Event::MirrorsResolved { attempt, count },
                        Err(kind) => Event::ResolveFailed { attempt, kind },
                    };
                    this.dispatch(event);
                });
                self.active.lock().unwrap().resolve = Some(handle);
            }
            Effect::ConnectMirror { attempt, index } => {
                let url = {
                    let m = self.mirrors.lock().unwrap();
                    (m.attempt == attempt)
                        .then(|| m.urls.get(index).cloned())
                        .flatten()
                };
                let cancel = Arc::new(AtomicBool::new(false));
                if let Some(old) = self.active.lock().unwrap().cancel.replace(cancel.clone()) {
                    old.store(true, Ordering::Relaxed);
                }
                let this = self.clone();
                let volume = self.volume.clone();
                let factory = self.sink_factory.clone();
                let timeouts = self.timeouts;
                std::thread::spawn(move || {
                    let notify = move |e: Event| this.dispatch(e);
                    match url {
                        Some(url) => stream_thread(
                            &url, attempt, &cancel, &volume, timeouts, factory, notify,
                        ),
                        None => notify(Event::ConnectFailed {
                            attempt,
                            kind: ErrorKind::Unavailable,
                        }),
                    }
                });
            }
        }
    }

    async fn resolve(&self, attempt: u64, station_id: &str) -> Result<usize, ErrorKind> {
        let station = self
            .feed
            .station(station_id)
            .ok_or(ErrorKind::Unavailable)?;
        let playlist = pick_playlist(&station.playlists).ok_or(ErrorKind::Unavailable)?;
        let mirrors =
            resolve_mirrors(&self.http, &playlist.url, &self.cache_dir, station_id).await?;
        let count = mirrors.len();
        *self.mirrors.lock().unwrap() = ResolvedMirrors {
            attempt,
            urls: mirrors,
        };
        Ok(count)
    }
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
                return Err(std::io::Error::other("cancelled"));
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
/// dropping the sink on return stops the audio.
fn stream_thread(
    url: &str,
    attempt: u64,
    cancel: &Arc<AtomicBool>,
    volume: &AtomicU32,
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
    let mut reader = ChannelReader {
        rx: spawn_pump(url.to_string(), timeouts.connect),
        cancel: cancel.clone(),
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
        move || factory(),
        volume,
        cancel,
        {
            let notify = notify.clone();
            move || {
                connected.store(true, Ordering::Relaxed);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::decode::tests::{Counts, MP3, NullSink};
    use crate::player::{Status, pls::resolve_mirrors};
    use std::collections::HashMap;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    type Routes = HashMap<String, (u16, Vec<u8>)>;

    /// In-process HTTP server: path -> (status, body). Closes after each response.
    struct Stub {
        base: String,
    }

    impl Stub {
        /// `build` receives the server's base URL so routes can point back at it.
        async fn start(build: impl FnOnce(&str) -> Routes) -> Stub {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            let routes = build(&base);
            tokio::spawn(async move {
                loop {
                    let Ok((mut sock, _)) = listener.accept().await else {
                        return;
                    };
                    let routes = routes.clone();
                    tokio::spawn(async move {
                        let mut req = Vec::new();
                        let mut b = [0u8; 1024];
                        while !req.windows(4).any(|w| w == b"\r\n\r\n") {
                            match sock.read(&mut b).await {
                                Ok(0) | Err(_) => return,
                                Ok(n) => req.extend_from_slice(&b[..n]),
                            }
                        }
                        let line = String::from_utf8_lossy(&req);
                        let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
                        let (status, body) = routes.get(&path).cloned().unwrap_or((404, vec![]));
                        let head = format!(
                            "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = sock.write_all(head.as_bytes()).await;
                        let _ = sock.write_all(&body).await;
                        let _ = sock.shutdown().await;
                    });
                }
            });
            Stub { base }
        }

        fn url(&self, path: &str) -> String {
            format!("{}{path}", self.base)
        }
    }

    fn routes(items: &[(&str, u16, &[u8])]) -> Routes {
        items
            .iter()
            .map(|(p, s, b)| (p.to_string(), (*s, b.to_vec())))
            .collect()
    }

    fn pls(urls: &[String]) -> Vec<u8> {
        urls.iter()
            .enumerate()
            .map(|(i, u)| format!("File{}={u}\n", i + 1))
            .collect::<String>()
            .into_bytes()
    }

    /// Routes for a feed with one station "s1" whose `.pls` files list the given
    /// stream paths, plus the stream bodies themselves.
    fn station_routes(base: &str, mp3_mirrors: &[&str], streams: &[(&str, u16, &[u8])]) -> Routes {
        let urls: Vec<String> = mp3_mirrors.iter().map(|p| format!("{base}{p}")).collect();
        station_routes_abs(base, &urls, streams)
    }

    /// Like `station_routes`, with Mirrors given as full URLs (e.g. other servers).
    fn station_routes_abs(
        base: &str,
        mirror_urls: &[String],
        streams: &[(&str, u16, &[u8])],
    ) -> Routes {
        let channels = serde_json::json!({"channels": [{
        "id": "s1", "title": "S1", "genre": "ambient", "listeners": "3",
        "playlists": [
            {"url": format!("{base}/mp3.pls"), "format": "mp3", "quality": "highest"},
        ]}]});
        let mut r = routes(streams);
        r.insert(
            "/channels.json".into(),
            (200, channels.to_string().into_bytes()),
        );
        r.insert("/mp3.pls".into(), (200, pls(mirror_urls)));
        r
    }

    #[tokio::test]
    async fn pls_fetch_caches_and_falls_back() {
        let dir = tempfile::tempdir().unwrap();
        let client = reqwest::Client::new();
        let ok = Stub::start(|_| {
            routes(&[(
                "/s.pls",
                200,
                b"[playlist]\nFile1=http://a/1\nFile2=http://b/2\n",
            )])
        })
        .await;
        let got = resolve_mirrors(&client, &ok.url("/s.pls"), dir.path(), "s1").await;
        assert_eq!(
            got,
            Ok(vec!["http://a/1".to_string(), "http://b/2".to_string()])
        );

        // Server error: the cached list is used.
        let broken = Stub::start(|_| routes(&[("/s.pls", 500, b"")])).await;
        let again = resolve_mirrors(&client, &broken.url("/s.pls"), dir.path(), "s1").await;
        assert_eq!(again, got);

        // Empty .pls with no cache.
        let empty = Stub::start(|_| routes(&[("/s.pls", 200, b"nothing here")])).await;
        let r = resolve_mirrors(&client, &empty.url("/s.pls"), dir.path(), "other").await;
        assert_eq!(r, Err(ErrorKind::Unavailable));

        // Nothing listening and no cache: offline.
        let r = resolve_mirrors(&client, "http://127.0.0.1:1/s.pls", dir.path(), "other").await;
        assert_eq!(r, Err(ErrorKind::Offline));
    }

    struct Rig {
        player: Player,
        snaps: Arc<Mutex<Vec<PlayerState>>>,
        counts: Counts,
        _dir: tempfile::TempDir,
    }

    async fn rig(stub: &Stub) -> Rig {
        rig_with(stub, Timeouts::default()).await
    }

    async fn rig_with(stub: &Stub, timeouts: Timeouts) -> Rig {
        let dir = tempfile::tempdir().unwrap();
        let feed = Arc::new(Feed::new(stub.url("/channels.json"), dir.path()));
        assert!(feed.refresh().await.unwrap());
        let counts = Counts::default();
        let c = counts.clone();
        let factory: SinkFactory =
            Arc::new(move || Ok(Box::new(NullSink(c.clone())) as Box<dyn PcmSink>));
        let snaps = Arc::new(Mutex::new(Vec::new()));
        let s2 = snaps.clone();
        let player = Player::with_timeouts(
            feed,
            dir.path(),
            Handle::current(),
            factory,
            timeouts,
            move |s| s2.lock().unwrap().push(s),
        );
        Rig {
            player,
            snaps,
            counts,
            _dir: dir,
        }
    }

    /// Waits until a snapshot emitted at index >= `from` satisfies `pred`.
    async fn wait_for(rig: &Rig, from: usize, pred: impl Fn(&PlayerState) -> bool) {
        for _ in 0..200 {
            if rig.snaps.lock().unwrap()[from..].iter().any(&pred) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("timed out; snapshots: {:?}", rig.snaps.lock().unwrap());
    }

    fn is_playing(s: &PlayerState) -> bool {
        s.status == Status::Playing
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn restore_loads_station_stopped_without_connecting_or_emitting() {
        let stub = Stub::start(|base| station_routes(base, &["/mp3"], &[("/mp3", 200, MP3)])).await;
        let rig = rig(&stub).await;
        rig.player.restore(Some("s1".into()), 0.3);
        let st = rig.player.state();
        assert_eq!(st.station_id.as_deref(), Some("s1"));
        assert_eq!(st.status, Status::Stopped);
        assert_eq!(st.volume, 0.3);
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(rig.snaps.lock().unwrap().is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn plays_mp3_through_second_mirror_then_stops() {
        let stub = Stub::start(|base| {
            station_routes(
                base,
                &["/dead", "/mp3"],
                &[("/mp3", 200, MP3), ("/dead", 404, b"")],
            )
        })
        .await;
        let rig = rig(&stub).await;

        assert!(rig.player.play("nope").is_err());
        rig.player.play("s1").unwrap();
        wait_for(&rig, 0, is_playing).await;
        assert!(*rig.counts.samples.lock().unwrap() > 0);
        assert_eq!(
            rig.snaps.lock().unwrap()[0].status,
            Status::Connecting {
                reconnecting: false
            }
        );

        rig.player.set_volume(0.25);
        assert_eq!(rig.player.state().volume, 0.25);
        rig.player.stop();
        let st = rig.player.state();
        assert_eq!(st.status, Status::Stopped);
        assert_eq!(st.station_id.as_deref(), Some("s1"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn all_mirrors_dead_ends_in_unavailable() {
        let stub =
            Stub::start(|base| station_routes(base, &["/dead", "/dead"], &[("/dead", 404, b"")]))
                .await;
        let rig = rig(&stub).await;
        rig.player.play("s1").unwrap();
        wait_for(&rig, 0, |s| {
            matches!(
                s.status,
                Status::Error {
                    error_kind: ErrorKind::Unavailable,
                    ..
                }
            )
        })
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn undecodable_stream_is_unsupported() {
        let stub = Stub::start(|base| {
            station_routes(base, &["/junk"], &[("/junk", 200, &[0x42u8; 20000])])
        })
        .await;
        let rig = rig(&stub).await;
        rig.player.play("s1").unwrap();
        wait_for(&rig, 0, |s| {
            matches!(
                s.status,
                Status::Error {
                    error_kind: ErrorKind::Unsupported,
                    ..
                }
            )
        })
        .await;
    }

    /// Raw TCP server that accepts, optionally sends `prefix`, then stays silent.
    fn silent_server(prefix: Vec<u8>) -> String {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/s", l.local_addr().unwrap());
        std::thread::spawn(move || {
            let mut held = Vec::new();
            for sock in l.incoming().flatten() {
                let mut sock = sock;
                let mut b = [0u8; 1024];
                let _ = std::io::Read::read(&mut sock, &mut b);
                let _ = std::io::Write::write_all(&mut sock, &prefix);
                held.push(sock); // never answer further, never close
            }
        });
        url
    }

    /// Serves `/channels.json` and a `.pls` listing `mirrors`; returns the rig.
    async fn rig_for_mirrors(mirrors: Vec<String>, timeouts: Timeouts) -> (Stub, Rig) {
        let stub = Stub::start(|base| station_routes_abs(base, &mirrors, &[])).await;
        let r = rig_with(&stub, timeouts).await;
        (stub, r)
    }

    const FAST: Timeouts = Timeouts {
        connect: Duration::from_millis(400),
        stall: Duration::from_millis(500),
    };

    fn error_kind(s: &PlayerState) -> Option<ErrorKind> {
        match s.status {
            Status::Error { error_kind, .. } => Some(error_kind),
            _ => None,
        }
    }

    fn is_reconnecting(s: &PlayerState) -> bool {
        s.status == Status::Connecting { reconnecting: true }
    }

    fn http_head() -> Vec<u8> {
        b"HTTP/1.1 200 OK\r\nContent-Length: 10000000\r\n\r\n".to_vec()
    }

    /// Raw server: `script(connection_index, socket)` runs on its own thread after
    /// the request head has been read.
    fn scripted_server(
        script: impl Fn(usize, std::net::TcpStream) + Send + Sync + 'static,
    ) -> String {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/s", l.local_addr().unwrap());
        let script = Arc::new(script);
        std::thread::spawn(move || {
            for (i, mut sock) in l.incoming().flatten().enumerate() {
                let script = script.clone();
                std::thread::spawn(move || {
                    let mut req = Vec::new();
                    let mut b = [0u8; 1024];
                    while !req.windows(4).any(|w| w == b"\r\n\r\n") {
                        match std::io::Read::read(&mut sock, &mut b) {
                            Ok(0) | Err(_) => return,
                            Ok(n) => req.extend_from_slice(&b[..n]),
                        }
                    }
                    script(i, sock);
                });
            }
        });
        url
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn first_connect_to_a_silent_server_times_out_as_unavailable() {
        let (_stub, rig) = rig_for_mirrors(vec![silent_server(Vec::new())], FAST).await;
        let started = Instant::now();
        rig.player.play("s1").unwrap();
        wait_for(&rig, 0, |s| error_kind(s) == Some(ErrorKind::Unavailable)).await;
        assert!(
            started.elapsed() >= FAST.connect,
            "failed before the timeout"
        );
        assert!(!rig.snaps.lock().unwrap().iter().any(is_reconnecting));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn first_connect_refused_is_offline_without_retrying() {
        let (_stub, rig) = rig_for_mirrors(vec!["http://127.0.0.1:1/s".into()], FAST).await;
        rig.player.play("s1").unwrap();
        wait_for(&rig, 0, |s| error_kind(s) == Some(ErrorKind::Offline)).await;
        assert!(!rig.snaps.lock().unwrap().iter().any(is_reconnecting));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn mid_stream_cut_reconnects_to_the_same_mirror() {
        // Every connection serves the whole tone, then the server closes: a cut.
        let url = scripted_server(|_, mut sock| {
            let mut head =
                format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", MP3.len()).into_bytes();
            head.extend_from_slice(MP3);
            let _ = std::io::Write::write_all(&mut sock, &head);
        });
        let (_stub, rig) = rig_for_mirrors(vec![url], FAST).await;
        rig.player.play("s1").unwrap();
        wait_for(&rig, 0, is_reconnecting).await;
        let n = rig.snaps.lock().unwrap().len();
        wait_for(&rig, n, is_playing).await; // the same Mirror serves again
        assert!(
            !rig.snaps
                .lock()
                .unwrap()
                .iter()
                .any(|s| error_kind(s).is_some())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stall_while_playing_is_a_drop_and_reconnecting_survives_until_stop() {
        // First connection: head + some audio, then silence. Later ones: 404.
        let url = scripted_server(|i, mut sock| {
            if i == 0 {
                let mut out = http_head();
                out.extend_from_slice(&MP3[..6000]);
                let _ = std::io::Write::write_all(&mut sock, &out);
                std::thread::sleep(Duration::from_secs(30));
            } else {
                let _ = std::io::Write::write_all(
                    &mut sock,
                    b"HTTP/1.1 404 X\r\nContent-Length: 0\r\n\r\n",
                );
            }
        });
        let (_stub, rig) = rig_for_mirrors(vec![url], FAST).await;
        rig.player.play("s1").unwrap();
        wait_for(&rig, 0, is_playing).await;
        let n = rig.snaps.lock().unwrap().len();
        wait_for(&rig, n, is_reconnecting).await; // stall = drop
        // The failing reconnect backs off instead of erroring out.
        tokio::time::sleep(Duration::from_millis(600)).await;
        assert!(is_reconnecting(&rig.player.state()));
        rig.player.stop();
        assert_eq!(rig.player.state().status, Status::Stopped);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn short_underrun_causes_no_status_change() {
        let url = scripted_server(|_, mut sock| {
            let mut out = http_head();
            out.extend_from_slice(&MP3[..6000]);
            let _ = std::io::Write::write_all(&mut sock, &out);
            std::thread::sleep(Duration::from_millis(250)); // shorter than the stall limit
            let _ = std::io::Write::write_all(&mut sock, &MP3[6000..12000]);
            std::thread::sleep(Duration::from_secs(30));
        });
        let (_stub, rig) = rig_for_mirrors(vec![url], FAST).await;
        rig.player.play("s1").unwrap();
        wait_for(&rig, 0, is_playing).await;
        let n = rig.snaps.lock().unwrap().len();
        tokio::time::sleep(Duration::from_millis(400)).await; // gap + resume, before a stall
        assert_eq!(
            rig.snaps.lock().unwrap().len(),
            n,
            "no snapshot during underrun"
        );
        assert!(is_playing(&rig.player.state()));
    }

    #[test]
    fn cancel_interrupts_a_stalled_connection_promptly() {
        let mut head = b"HTTP/1.1 200 OK\r\nContent-Length: 10000000\r\n\r\n".to_vec();
        head.extend_from_slice(&MP3[..6000]);
        for (name, prefix) in [("no response", Vec::new()), ("mid-stream", head)] {
            let url = silent_server(prefix);
            let cancel = Arc::new(AtomicBool::new(false));
            let (done_tx, done_rx) = mpsc::channel();
            let c = cancel.clone();
            std::thread::spawn(move || {
                let counts = Counts::default();
                let factory: SinkFactory =
                    Arc::new(move || Ok(Box::new(NullSink(counts.clone())) as Box<dyn PcmSink>));
                stream_thread(
                    &url,
                    1,
                    &c,
                    &AtomicU32::new(0),
                    Timeouts::default(),
                    factory,
                    |_| {},
                );
                let _ = done_tx.send(());
            });
            std::thread::sleep(Duration::from_millis(400));
            assert!(done_rx.try_recv().is_err(), "{name}: exited before cancel");
            cancel.store(true, Ordering::Relaxed);
            done_rx
                .recv_timeout(Duration::from_secs(2))
                .unwrap_or_else(|_| panic!("{name}: thread still alive after cancel"));
        }
    }

    /// Live debugging aid: `SOMAFM_LIVE_URL=... cargo test live_stream -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn live_stream() {
        let url = std::env::var("SOMAFM_LIVE_URL").expect("SOMAFM_LIVE_URL");
        let cancel = Arc::new(AtomicBool::new(false));
        let c = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(12));
            c.store(true, Ordering::Relaxed);
        });
        let counts = Counts::default();
        let k = counts.clone();
        let factory: SinkFactory =
            Arc::new(move || Ok(Box::new(NullSink(k.clone())) as Box<dyn PcmSink>));
        let k2 = counts.clone();
        stream_thread(
            &url,
            1,
            &cancel,
            &AtomicU32::new(0),
            Timeouts::default(),
            factory,
            move |e| eprintln!("event: {e:?} samples={}", k2.samples.lock().unwrap()),
        );
        eprintln!("samples={}", counts.samples.lock().unwrap());
    }

    /// One-shot raw HTTP server: asserts on the request, replies with `headers`
    /// (extra response header lines) and `body`, then closes.
    fn raw_server(headers: &'static str, body: Vec<u8>, want_icy_header: bool) -> String {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/s", l.local_addr().unwrap());
        std::thread::spawn(move || {
            let (mut sock, _) = l.accept().unwrap();
            let mut req = Vec::new();
            let mut b = [0u8; 1024];
            while !req.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = std::io::Read::read(&mut sock, &mut b).unwrap();
                req.extend_from_slice(&b[..n]);
            }
            let req = String::from_utf8_lossy(&req).to_lowercase();
            assert_eq!(req.contains("icy-metadata: 1"), want_icy_header);
            let head = format!("HTTP/1.0 200 OK\r\n{headers}\r\n");
            let _ = std::io::Write::write_all(&mut sock, head.as_bytes());
            let _ = std::io::Write::write_all(&mut sock, &body);
        });
        url
    }

    /// Runs `stream_thread` to stream end; returns events and decoded sample count.
    fn run_stream(url: &str) -> (Vec<Event>, usize) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let e2 = events.clone();
        let counts = Counts::default();
        let c = counts.clone();
        let factory: SinkFactory =
            Arc::new(move || Ok(Box::new(NullSink(c.clone())) as Box<dyn PcmSink>));
        stream_thread(
            url,
            1,
            &Arc::new(AtomicBool::new(false)),
            &AtomicU32::new(0),
            Timeouts::default(),
            factory,
            move |e| e2.lock().unwrap().push(e),
        );
        let samples = *counts.samples.lock().unwrap();
        let events = events.lock().unwrap().clone();
        (events, samples)
    }

    #[test]
    fn icy_framed_stream_decodes_and_reports_titles() {
        let titles = ["Artist One - Song A", "Plain title"];
        let framed = crate::player::icy::tests::frame(MP3, 2048, &titles);
        let url = raw_server("icy-metaint: 2048\r\n", framed, true);
        let (events, samples) = run_stream(&url);
        assert!(samples > 0, "decode must still yield PCM: {events:?}");
        let got: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Event::TitleChanged { attempt: 1, raw } => Some(raw.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(got, titles);
        assert!(events.contains(&Event::Connected { attempt: 1 }));
    }

    #[test]
    fn stream_without_icy_header_is_untouched() {
        let url = raw_server("", MP3.to_vec(), true);
        let (events, samples) = run_stream(&url);
        assert!(samples > 0);
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::TitleChanged { .. }))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unrecognised_stream_larger_than_the_probe_budget_is_unsupported() {
        let body = vec![0x42u8; 300 * 1024];
        let stub =
            Stub::start(|base| station_routes(base, &["/junk"], &[("/junk", 200, &body)])).await;
        let rig = rig(&stub).await;
        rig.player.play("s1").unwrap();
        wait_for(&rig, 0, |s| {
            matches!(
                s.status,
                Status::Error {
                    error_kind: ErrorKind::Unsupported,
                    ..
                }
            )
        })
        .await;
        assert_eq!(*rig.counts.samples.lock().unwrap(), 0);
    }
}
