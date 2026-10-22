
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use dashmap::DashMap;
use tokio::sync::mpsc::UnboundedSender;
use parking_lot::RwLock;
use crate::model::protocol::ServerMessage;
use crate::audio::player::PlayerHandle;

pub struct Session {
    pub session_id: String,
    pub user_id: Option<String>,
    pub resuming: AtomicBool,
    pub timeout: AtomicU64,
    pub disconnected_at: Arc<RwLock<Option<Instant>>>,
    pub players: DashMap<String, Arc<PlayerHandle>>,
    pub ws_sender: Arc<RwLock<Option<UnboundedSender<ServerMessage>>>>,
}

impl Session {
    pub fn new(session_id: String, user_id: Option<String>) -> Self {
        Self {
            session_id,
            user_id,
            resuming: AtomicBool::new(false),
            timeout: AtomicU64::new(60),
            disconnected_at: Arc::new(RwLock::new(None)),
            players: DashMap::new(),
            ws_sender: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_sender(&self, sender: Option<UnboundedSender<ServerMessage>>) {
        let mut guard = self.ws_sender.write();
        *guard = sender;
    }

    pub fn mark_connected(&self) {
        *self.disconnected_at.write() = None;
    }

    pub fn mark_disconnected(&self) {
        *self.disconnected_at.write() = Some(Instant::now());
    }

    pub fn is_expired(&self) -> bool {
        let guard = self.disconnected_at.read();
        if let Some(disconnected_time) = *guard {
            // If someone actively connected, it's not expired
            if self.ws_sender.read().is_some() {
                return false;
            }
            let elapsed = disconnected_time.elapsed().as_secs();
            let is_resuming = self.resuming.load(Ordering::Relaxed);
            let allowed_timeout = if is_resuming {
                self.timeout.load(Ordering::Relaxed)
            } else {
                // If resuming was not enabled, allow a short 10s grace period for brief reconnects
                10
            };
            elapsed >= allowed_timeout
        } else {
            false
        }
    }

    pub fn set_resuming(&self, resuming: bool) {
        self.resuming.store(resuming, Ordering::Relaxed);
    }

    pub fn get_resuming(&self) -> bool {
        self.resuming.load(Ordering::Relaxed)
    }

    pub fn set_timeout(&self, timeout: u64) {
        self.timeout.store(timeout, Ordering::Relaxed);
    }

    pub fn get_timeout(&self) -> u64 {
        self.timeout.load(Ordering::Relaxed)
    }

    pub fn send(&self, msg: ServerMessage) {
        let guard = self.ws_sender.read();
        if let Some(ref sender) = *guard {
            let _ = sender.send(msg);
        }
    }
}
