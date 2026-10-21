//! Player: pure `PlayerState` reducer plus the shell that executes its effects
//! (`.pls` -> Mirror -> HTTP stream -> symphonia decode -> rodio output).

pub mod decode;
pub mod icy;
pub mod pls;
pub mod reducer;
pub mod shell;
pub mod source;
pub mod spectrum;
pub mod visualizer;

use serde::Serialize;

pub use shell::{Player, SinkFactory};

pub const DEFAULT_VOLUME: f32 = 0.8;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub station_id: Option<String>,
    pub status: Status,
    pub volume: f32,
    pub now_playing: Option<NowPlaying>,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            station_id: None,
            status: Status::Stopped,
            volume: DEFAULT_VOLUME,
            now_playing: None,
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Status {
    Stopped,
    Connecting {
        reconnecting: bool,
    },
    Playing,
    #[serde(rename_all = "camelCase")]
    Error {
        error_kind: ErrorKind,
        message: String,
    },
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    Offline,
    Unavailable,
    Unsupported,
}

impl ErrorKind {
    pub fn message(self) -> &'static str {
        match self {
            ErrorKind::Offline => "You're offline",
            ErrorKind::Unavailable => "Station unavailable",
            ErrorKind::Unsupported => "Can't play this stream",
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct NowPlaying {
    pub raw: String,
    pub artist: Option<String>,
    pub title: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_serialises_as_spec() {
        let mut s = PlayerState::default();
        s.station_id = Some("groovesalad".into());
        s.status = Status::Error {
            error_kind: ErrorKind::Offline,
            message: "x".into(),
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["stationId"], "groovesalad");
        assert_eq!(v["status"]["kind"], "error");
        assert_eq!(v["status"]["errorKind"], "offline");
        assert!(v.get("quality").is_none());
        assert!((v["volume"].as_f64().unwrap() - 0.8).abs() < 1e-6);
        assert!(v["nowPlaying"].is_null());
        let c = serde_json::to_value(Status::Connecting {
            reconnecting: false,
        })
        .unwrap();
        assert_eq!(c["kind"], "connecting");
        assert_eq!(c["reconnecting"], false);
    }
}
