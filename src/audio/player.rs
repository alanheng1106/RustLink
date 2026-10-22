use async_trait::async_trait;
use songbird::driver::Driver;
use songbird::events::{Event, EventContext, EventHandler, TrackEvent};
use songbird::id::{ChannelId, GuildId, UserId};
use songbird::tracks::{Track as SongbirdTrack, TrackHandle as SongbirdTrackHandle};
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc::UnboundedSender, Mutex, RwLock};

use crate::audio::source::SourceManager;
use crate::model::protocol::{
    ExceptionData, Player, PlayerState, ServerEvent, ServerMessage, Track, VoiceState,
};

struct TrackEndNotifier {
    guild_id: String,
    track: Track,
    sender: Option<UnboundedSender<ServerMessage>>,
}

#[async_trait]
impl EventHandler for TrackEndNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(_) = ctx {
            if let Some(ref sender) = self.sender {
                let _ = sender.send(ServerMessage::Event(ServerEvent::TrackEndEvent {
                    guild_id: self.guild_id.clone(),
                    track: self.track.clone(),
                    reason: "finished".to_string(),
                }));
            }
        }
        None
    }
}

struct TrackStartNotifier {
    guild_id: String,
    track: Track,
    sender: Option<UnboundedSender<ServerMessage>>,
}

#[async_trait]
impl EventHandler for TrackStartNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(_) = ctx {
            if let Some(ref sender) = self.sender {
                let _ = sender.send(ServerMessage::Event(ServerEvent::TrackStartEvent {
                    guild_id: self.guild_id.clone(),
                    track: self.track.clone(),
                }));
            }
        }
        None
    }
}

struct TrackErrorNotifier {
    guild_id: String,
    track: Track,
    sender: Option<UnboundedSender<ServerMessage>>,
}

#[async_trait]
impl EventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(tracks) = ctx {
            let error_msg = if let Some((_, _)) = tracks.first() {
                "Audio stream error occurred during playback"
            } else {
                "Unknown track error"
            };

            if let Some(ref sender) = self.sender {
                let _ = sender.send(ServerMessage::Event(ServerEvent::TrackExceptionEvent {
                    guild_id: self.guild_id.clone(),
                    track: self.track.clone(),
                    exception: ExceptionData {
                        message: error_msg.to_string(),
                        severity: "common".to_string(),
                        cause: error_msg.to_string(),
                    },
                }));
            }
        }
        None
    }
}

pub struct PlayerHandle {
    pub guild_id: String,
    pub user_id: Option<String>,
    pub driver: Arc<Mutex<Driver>>,
    pub current_track: Arc<RwLock<Option<Track>>>,
    pub current_handle: Arc<RwLock<Option<SongbirdTrackHandle>>>,
    pub voice_state: Arc<RwLock<Option<VoiceState>>>,
    pub volume: AtomicU32,
    pub paused: AtomicBool,
    pub filters: Arc<RwLock<serde_json::Value>>,
}

impl PlayerHandle {
    pub fn new(guild_id: String, user_id: Option<String>) -> Self {
        Self {
            guild_id,
            user_id,
            driver: Arc::new(Mutex::new(Driver::new(Default::default()))),
            current_track: Arc::new(RwLock::new(None)),
            current_handle: Arc::new(RwLock::new(None)),
            voice_state: Arc::new(RwLock::new(None)),
            volume: AtomicU32::new(100),
            paused: AtomicBool::new(false),
            filters: Arc::new(RwLock::new(serde_json::json!({}))),
        }
    }

    pub async fn update_voice(&self, voice: VoiceState) -> Result<(), String> {
        let gid_u64 = self.guild_id.parse::<u64>().map_err(|e| e.to_string())?;
        let gid = NonZeroU64::new(gid_u64).ok_or("Guild ID cannot be zero")?;

        let uid_u64 = self
            .user_id
            .as_deref()
            .unwrap_or("0")
            .parse::<u64>()
            .unwrap_or(1);
        let uid = NonZeroU64::new(uid_u64).unwrap_or(NonZeroU64::new(1).unwrap());

        // Channel ID placeholder (Songbird driver handles the gateway voice handshake)
        let dummy_channel = NonZeroU64::new(1).unwrap();

        let info = songbird::ConnectionInfo {
            channel_id: ChannelId::from(dummy_channel),
            endpoint: voice.endpoint.clone(),
            guild_id: GuildId::from(gid),
            session_id: voice.session_id.clone(),
            token: voice.token.clone(),
            user_id: UserId::from(uid),
        };

        let mut driver = self.driver.lock().await;
        driver.connect(info).await.map_err(|e| format!("Failed to connect voice driver: {e}"))?;

        let mut vs = self.voice_state.write().await;
        *vs = Some(voice);

        Ok(())
    }

    pub async fn play(
        &self,
        track: Track,
        position: Option<u64>,
        source_manager: &SourceManager,
        event_sender: Option<UnboundedSender<ServerMessage>>,
    ) -> Result<(), String> {
        let uri = track
            .info
            .uri
            .as_deref()
            .unwrap_or(&track.info.identifier);

        let input = source_manager
            .create_input(uri)
            .ok_or_else(|| format!("Unsupported audio source: {uri}"))?;

        let mut track_obj = SongbirdTrack::from(input);

        // Apply current volume (100 = 1.0)
        let vol = self.volume.load(Ordering::Relaxed) as f32 / 100.0;
        track_obj = track_obj.volume(vol);

        if self.paused.load(Ordering::Relaxed) {
            track_obj = track_obj.pause();
        }

        let mut driver = self.driver.lock().await;
        let handle = driver.play_only(track_obj);

        // Initial position seek if requested
        if let Some(pos) = position {
            let _ = handle.seek(Duration::from_millis(pos));
        }

        // Register event listeners
        let _ = handle.add_event(
            Event::Track(TrackEvent::Play),
            TrackStartNotifier {
                guild_id: self.guild_id.clone(),
                track: track.clone(),
                sender: event_sender.clone(),
            },
        );

        let _ = handle.add_event(
            Event::Track(TrackEvent::End),
            TrackEndNotifier {
                guild_id: self.guild_id.clone(),
                track: track.clone(),
                sender: event_sender.clone(),
            },
        );

        let _ = handle.add_event(
            Event::Track(TrackEvent::Error),
            TrackErrorNotifier {
                guild_id: self.guild_id.clone(),
                track: track.clone(),
                sender: event_sender,
            },
        );

        let mut cur_track = self.current_track.write().await;
        *cur_track = Some(track);

        let mut cur_handle = self.current_handle.write().await;
        *cur_handle = Some(handle);

        Ok(())
    }

    pub async fn set_pause(&self, pause: bool) -> Result<(), String> {
        self.paused.store(pause, Ordering::Relaxed);
        let cur_handle = self.current_handle.read().await;
        if let Some(ref handle) = *cur_handle {
            if pause {
                let _ = handle.pause();
            } else {
                let _ = handle.play();
            }
        }
        Ok(())
    }

    pub async fn set_volume(&self, volume: u32) -> Result<(), String> {
        self.volume.store(volume, Ordering::Relaxed);
        let cur_handle = self.current_handle.read().await;
        if let Some(ref handle) = *cur_handle {
            let _ = handle.set_volume(volume as f32 / 100.0);
        }
        Ok(())
    }

    pub async fn seek(&self, position_ms: u64) -> Result<(), String> {
        let cur_handle = self.current_handle.read().await;
        if let Some(ref handle) = *cur_handle {
            let _ = handle.seek(Duration::from_millis(position_ms));
        }
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), String> {
        let mut cur_handle = self.current_handle.write().await;
        if let Some(ref handle) = *cur_handle {
            let _ = handle.stop();
        }
        *cur_handle = None;

        let mut cur_track = self.current_track.write().await;
        *cur_track = None;

        let mut driver = self.driver.lock().await;
        driver.stop();

        Ok(())
    }

    pub async fn destroy(&self) -> Result<(), String> {
        self.stop().await?;
        let mut driver = self.driver.lock().await;
        driver.leave();
        Ok(())
    }

    pub async fn get_state(&self) -> PlayerState {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut position = 0u64;
        let mut connected = false;

        let vs = self.voice_state.read().await;
        if vs.is_some() {
            connected = true;
        }

        let handle_guard = self.current_handle.read().await;
        if let Some(ref handle) = *handle_guard {
            if let Ok(info) = handle.get_info().await {
                position = info.position.as_millis() as u64;
            }
        }

        PlayerState {
            time: now_ms,
            position,
            connected,
            ping: 10,
        }
    }

    pub async fn get_player_dto(&self) -> Player {
        let track = self.current_track.read().await.clone();
        let state = self.get_state().await;
        let voice = self
            .voice_state
            .read()
            .await
            .clone()
            .unwrap_or_else(|| VoiceState {
                token: String::new(),
                endpoint: String::new(),
                session_id: String::new(),
            });
        let filters = self.filters.read().await.clone();

        Player {
            guild_id: self.guild_id.clone(),
            track,
            volume: self.volume.load(Ordering::Relaxed),
            paused: self.paused.load(Ordering::Relaxed),
            state,
            voice,
            filters,
        }
    }
}
