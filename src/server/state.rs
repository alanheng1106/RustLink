use dashmap::DashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::audio::player::PlayerHandle;
use crate::audio::source::SourceManager;
use crate::model::protocol::{
    CpuStats, FrameStats, GitInfo, InfoResponse, MemoryStats, PluginInfo, StatsResponse,
    VersionInfo,
};
use crate::model::session::Session;

#[derive(Clone)]
pub struct AppState {
    pub sessions: Arc<DashMap<String, Arc<Session>>>,
    pub source_manager: Arc<SourceManager>,
    pub password: Arc<String>,
    pub start_time: Instant,
}

impl AppState {
    pub fn new(password: String) -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            source_manager: Arc::new(SourceManager::new()),
            password: Arc::new(password),
            start_time: Instant::now(),
        }
    }

    pub fn get_or_create_session(&self, session_id: &str, user_id: Option<String>) -> (Arc<Session>, bool) {
        if let Some(session) = self.sessions.get(session_id) {
            return (session.clone(), true);
        }

        let session = Arc::new(Session::new(session_id.to_string(), user_id));
        self.sessions.insert(session_id.to_string(), session.clone());
        (session, false)
    }

    pub fn get_session(&self, session_id: &str) -> Option<Arc<Session>> {
        self.sessions.get(session_id).map(|s| s.clone())
    }

    pub fn get_or_create_player(
        &self,
        session: &Session,
        guild_id: &str,
    ) -> Arc<PlayerHandle> {
        if let Some(p) = session.players.get(guild_id) {
            return p.clone();
        }

        let player = Arc::new(PlayerHandle::new(
            guild_id.to_string(),
            session.user_id.clone(),
        ));
        session.players.insert(guild_id.to_string(), player.clone());
        player
    }

    pub fn info(&self) -> InfoResponse {
        InfoResponse {
            version: VersionInfo {
                semver: "4.0.0".to_string(),
                major: 4,
                minor: 0,
                patch: 0,
                pre_release: None,
                build: None,
            },
            build_time: 1729500000000,
            git: GitInfo {
                branch: "main".to_string(),
                commit: "rustlink".to_string(),
                commit_time: 1729500000000,
            },
            jvm: "Rust Native (Tokio + Axum)".to_string(),
            lavaplayer: "Songbird 0.6.1 + Symphonia 0.5.5".to_string(),
            source_managers: vec![
                "youtube".to_string(),
                "http".to_string(),
                "local".to_string(),
                "hls".to_string(),
            ],
            filters: vec![
                "volume".to_string(),
                "equalizer".to_string(),
                "timescale".to_string(),
            ],
            plugins: vec![PluginInfo {
                name: "rust-native-node".to_string(),
                version: "0.1.0".to_string(),
            }],
        }
    }

    pub fn stats(&self) -> StatsResponse {
        let mut total_players = 0;
        let mut playing_players = 0;

        for s in self.sessions.iter() {
            total_players += s.players.len();
            for p in s.players.iter() {
                if let Ok(handle_guard) = p.current_handle.try_read() {
                    if handle_guard.is_some() {
                        playing_players += 1;
                    }
                }
            }
        }

        StatsResponse {
            players: total_players,
            playing_players,
            uptime: self.start_time.elapsed().as_millis() as u64,
            memory: MemoryStats {
                free: 512 * 1024 * 1024,
                used: 32 * 1024 * 1024,
                allocated: 64 * 1024 * 1024,
                reservable: 1024 * 1024 * 1024,
            },
            cpu: CpuStats {
                cores: std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(4),
                system_load: 0.02,
                lavalink_load: 0.005,
            },
            frame_stats: Some(FrameStats {
                sent: 1000,
                nulled: 0,
                deficit: 0,
            }),
        }
    }
}
