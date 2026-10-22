use std::time::Duration;
use tracing::info;
use crate::server::state::AppState;

/// Spawns a background task that periodically reaps expired sessions
pub fn start_session_reaper(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;

            let mut to_reap = Vec::new();

            for entry in state.sessions.iter() {
                if entry.value().is_expired() {
                    to_reap.push(entry.key().clone());
                }
            }

            for session_id in to_reap {
                // Atomic remove_if ensures we don't reap if a client just reconnected
                if let Some((_, session)) = state.sessions.remove_if(&session_id, |_, s| s.is_expired()) {
                    let player_count = session.players.len();
                    info!(
                        session_id = %session_id,
                        player_count = player_count,
                        "Session expired, reaped by Session Reaper"
                    );

                    for player in session.players.iter() {
                        let _ = player.value().destroy().await;
                    }
                    session.players.clear();
                }
            }
        }
    });
}
