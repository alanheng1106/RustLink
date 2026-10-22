use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::model::protocol::ServerMessage;
use crate::model::session::Session;
use crate::server::state::AppState;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    // 1. Auth check
    let auth = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok());

    if auth != Some(&*state.password) {
        warn!(
            expected = %state.password,
            received = ?auth,
            "Unauthorized WebSocket connection attempt: password mismatch"
        );
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let user_id = headers
        .get("user-id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let client_name = headers
        .get("client-name")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("Unknown Client");

    let resuming_session_id = headers
        .get("session-id")
        .and_then(|h| h.to_str().ok());

    let (session, resumed) = if let Some(sid) = resuming_session_id {
        state.get_or_create_session(sid, user_id.clone())
    } else {
        let new_id = Uuid::new_v4().to_string();
        state.get_or_create_session(&new_id, user_id.clone())
    };

    info!(
        session_id = %session.session_id,
        client = %client_name,
        resumed = %resumed,
        "Lavalink client connected via WebSocket"
    );

    let state_clone = state.clone();

    ws.on_upgrade(move |socket| handle_socket(socket, session, state_clone, resumed))
        .into_response()
}

async fn handle_socket(
    socket: WebSocket,
    session: Arc<Session>,
    state: AppState,
    resumed: bool,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    // Send Ready event immediately
    let ready_msg = ServerMessage::Ready {
        resumed,
        session_id: session.session_id.clone(),
    };
    if let Ok(json) = serde_json::to_string(&ready_msg) {
        let _ = ws_sender.send(Message::Text(json.into())).await;
    }

    session.set_sender(Some(tx.clone()));

    // Forward messages from rx to ws_sender
    let forward_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if ws_sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Spawn 5-second periodic PlayerUpdate & Stats broadcaster
    let session_for_ticker = session.clone();
    let tx_for_ticker = tx.clone();
    let state_for_ticker = state.clone();

    let ticker_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;

            // 1. Send player updates
            for p in session_for_ticker.players.iter() {
                let state = p.get_state().await;
                let _ = tx_for_ticker.send(ServerMessage::PlayerUpdate {
                    guild_id: p.guild_id.clone(),
                    state,
                });
            }

            // 2. Send stats
            let stats = state_for_ticker.stats();
            let _ = tx_for_ticker.send(ServerMessage::Stats(stats));
        }
    });

    // Listen for incoming WebSocket messages (heartbeats, close frames, etc.)
    while let Some(msg_result) = ws_receiver.next().await {
        match msg_result {
            Ok(Message::Close(frame)) => {
                let code = frame.as_ref().map(|f| f.code);
                let reason = frame.as_ref().map(|f| f.reason.to_string());
                info!(session_id = %session.session_id, ?code, ?reason, "Client sent WebSocket close frame");
                break;
            }
            Ok(Message::Text(text)) => {
                info!(session_id = %session.session_id, "Received WebSocket text from client: {text}");
            }
            Ok(Message::Ping(_)) => {}
            Ok(Message::Pong(_)) => {}
            Err(e) => {
                warn!(session_id = %session.session_id, "WebSocket receive error: {e}");
                break;
            }
            _ => {}
        }
    }

    // Connection ended: cleanup tasks
    ticker_task.abort();
    forward_task.abort();
    session.set_sender(None);

    info!(session_id = %session.session_id, "WebSocket client disconnected");
}
