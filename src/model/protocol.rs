#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub semver: String,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
    pub build: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitInfo {
    pub branch: String,
    pub commit: String,
    pub commit_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoResponse {
    pub version: VersionInfo,
    pub build_time: i64,
    pub git: GitInfo,
    pub jvm: String,
    pub lavaplayer: String,
    pub source_managers: Vec<String>,
    pub filters: Vec<String>,
    pub plugins: Vec<PluginInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStats {
    pub free: u64,
    pub used: u64,
    pub allocated: u64,
    pub reservable: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuStats {
    pub cores: usize,
    pub system_load: f64,
    pub lavalink_load: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameStats {
    pub sent: u64,
    pub nulled: u64,
    pub deficit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsResponse {
    pub players: usize,
    pub playing_players: usize,
    pub uptime: u64,
    pub memory: MemoryStats,
    pub cpu: CpuStats,
    pub frame_stats: Option<FrameStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackInfo {
    pub identifier: String,
    pub is_seekable: bool,
    pub author: String,
    pub length: u64,
    pub is_stream: bool,
    pub position: u64,
    pub title: String,
    pub uri: Option<String>,
    pub artwork_url: Option<String>,
    pub isrc: Option<String>,
    pub source_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub encoded: String,
    pub info: TrackInfo,
    #[serde(default)]
    pub plugin_info: Value,
    #[serde(default)]
    pub user_data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "loadType", content = "data")]
pub enum LoadResult {
    Track(Track),
    Playlist(PlaylistData),
    Search(Vec<Track>),
    Empty(Value),
    Error(ExceptionData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistInfo {
    pub name: String,
    pub selected_track: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistData {
    pub info: PlaylistInfo,
    pub plugin_info: Value,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionData {
    pub message: String,
    pub severity: String,
    pub cause: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub time: u64,
    pub position: u64,
    pub connected: bool,
    pub ping: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceState {
    pub token: String,
    pub endpoint: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub guild_id: String,
    pub track: Option<Track>,
    pub volume: u32,
    pub paused: bool,
    pub state: PlayerState,
    pub voice: VoiceState,
    #[serde(default)]
    pub filters: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerUpdateTrack {
    pub encoded: Option<String>,
    pub identifier: Option<String>,
    #[serde(default)]
    pub user_data: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePlayerRequest {
    pub track: Option<PlayerUpdateTrack>,
    pub position: Option<u64>,
    pub end_time: Option<u64>,
    pub volume: Option<u32>,
    pub paused: Option<bool>,
    pub filters: Option<Value>,
    pub voice: Option<VoiceState>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSessionRequest {
    pub resuming: Option<bool>,
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    pub resuming: bool,
    pub timeout: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum ServerMessage {
    #[serde(rename_all = "camelCase")]
    Ready {
        resumed: bool,
        session_id: String,
    },
    #[serde(rename_all = "camelCase")]
    PlayerUpdate {
        guild_id: String,
        state: PlayerState,
    },
    Stats(StatsResponse),
    Event(ServerEvent),
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
    #[serde(rename_all = "camelCase")]
    TrackStartEvent {
        guild_id: String,
        track: Track,
    },
    #[serde(rename_all = "camelCase")]
    TrackEndEvent {
        guild_id: String,
        track: Track,
        reason: String,
    },
    #[serde(rename_all = "camelCase")]
    TrackExceptionEvent {
        guild_id: String,
        track: Track,
        exception: ExceptionData,
    },
    #[serde(rename_all = "camelCase")]
    TrackStuckEvent {
        guild_id: String,
        track: Track,
        threshold_ms: u64,
    },
    #[serde(rename_all = "camelCase")]
    WebSocketClosedEvent {
        guild_id: String,
        code: u16,
        reason: String,
        by_remote: bool,
    },
}
