#![allow(dead_code)]

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
    pub resuming: bool,
    pub timeout: u64,
    pub last_active: Instant,
    pub players: DashMap<String, Arc<PlayerHandle>>,
    pub ws_sender: Arc<RwLock<Option<UnboundedSender<ServerMessage>>>>,
}

impl Session {
    pub fn new(session_id: String, user_id: Option<String>) -> Self {
        Self {
            session_id,
            user_id,
            resuming: false,
            timeout: 60,
            last_active: Instant::now(),
            players: DashMap::new(),
            ws_sender: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_sender(&self, sender: Option<UnboundedSender<ServerMessage>>) {
        let mut guard = self.ws_sender.write();
        *guard = sender;
    }

    pub fn send(&self, msg: ServerMessage) {
        let guard = self.ws_sender.read();
        if let Some(ref sender) = *guard {
            let _ = sender.send(msg);
        }
    }
}
