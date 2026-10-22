use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;

use crate::audio::codec::decode_track;
use crate::model::protocol::{
    LoadResult, Player, SessionResponse, Track, TrackInfo, UpdatePlayerRequest,
    UpdateSessionRequest,
};
use crate::server::state::AppState;

#[derive(Deserialize)]
pub struct LoadTracksQuery {
    pub identifier: String,
}

#[derive(Deserialize)]
pub struct PlayerQuery {
    #[serde(default)]
    pub no_replace: bool,
}

fn check_auth(headers: &HeaderMap, state: &AppState) -> Result<(), StatusCode> {
    if let Some(auth) = headers.get("authorization") {
        if let Ok(pass) = auth.to_str() {
            if pass == *state.password {
                return Ok(());
            }
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}

pub async fn get_version() -> &'static str {
    "4.0.0"
}

pub async fn get_info(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::model::protocol::InfoResponse>, StatusCode> {
    check_auth(&headers, &state)?;
    Ok(Json(state.info()))
}

pub async fn get_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::model::protocol::StatsResponse>, StatusCode> {
    check_auth(&headers, &state)?;
    Ok(Json(state.stats()))
}

pub async fn load_tracks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<LoadTracksQuery>,
) -> Result<Json<LoadResult>, StatusCode> {
    check_auth(&headers, &state)?;
    let result = state.source_manager.resolve(&query.identifier).await;
    Ok(Json(result))
}

pub async fn decode_single_track(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Result<Json<TrackInfo>, (StatusCode, String)> {
    check_auth(&headers, &state).map_err(|s| (s, "Unauthorized".to_string()))?;
    let info = decode_track(&body).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    Ok(Json(info))
}

pub async fn decode_multiple_tracks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(tracks): Json<Vec<String>>,
) -> Result<Json<Vec<TrackInfo>>, (StatusCode, String)> {
    check_auth(&headers, &state).map_err(|s| (s, "Unauthorized".to_string()))?;
    let mut decoded = Vec::new();
    for t in tracks {
        if let Ok(info) = decode_track(&t) {
            decoded.push(info);
        }
    }
    Ok(Json(decoded))
}

pub async fn update_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(req): Json<UpdateSessionRequest>,
) -> Result<Json<SessionResponse>, StatusCode> {
    check_auth(&headers, &state)?;
    let session = state
        .get_session(&session_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(r) = req.resuming {
        session.set_resuming(r);
    }
    if let Some(t) = req.timeout {
        session.set_timeout(t);
    }

    Ok(Json(SessionResponse {
        resuming: session.get_resuming(),
        timeout: session.get_timeout(),
    }))
}

pub async fn get_players(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<Vec<Player>>, StatusCode> {
    check_auth(&headers, &state)?;
    let session = state
        .get_session(&session_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut list = Vec::new();
    for p in session.players.iter() {
        list.push(p.get_player_dto().await);
    }
    Ok(Json(list))
}

pub async fn get_player(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((session_id, guild_id)): Path<(String, String)>,
) -> Result<Json<Player>, StatusCode> {
    check_auth(&headers, &state)?;
    let session = state
        .get_session(&session_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let player = session
        .players
        .get(&guild_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(player.get_player_dto().await))
}

pub async fn update_player(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((session_id, guild_id)): Path<(String, String)>,
    Query(query): Query<PlayerQuery>,
    Json(req): Json<UpdatePlayerRequest>,
) -> Result<Json<Player>, (StatusCode, String)> {
    check_auth(&headers, &state).map_err(|s| (s, "Unauthorized".to_string()))?;

    let session = state
        .get_session(&session_id)
        .ok_or((StatusCode::NOT_FOUND, "Session not found".to_string()))?;

    let player = state.get_or_create_player(&session, &guild_id);

    // 1. Update Voice State if supplied
    if let Some(voice) = req.voice {
        player
            .update_voice(voice)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    }

    // 2. Volume update
    if let Some(volume) = req.volume {
        let _ = player.set_volume(volume).await;
    }

    // 3. Pause update
    if let Some(paused) = req.paused {
        let _ = player.set_pause(paused).await;
    }

    // 4. Seek position
    if let Some(pos) = req.position {
        let _ = player.seek(pos).await;
    }

    // 5. Track update
    if let Some(track_req) = req.track {
        if track_req.encoded.is_none() && track_req.identifier.is_none() {
            // Null track means stop
            let _ = player.stop().await;
        } else {
            let already_playing = player.current_handle.read().await.is_some();
            if !(query.no_replace && already_playing) {
                let resolved_track = if let Some(ref enc) = track_req.encoded {
                    let info = decode_track(enc).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
                    Track {
                        encoded: enc.clone(),
                        info,
                        plugin_info: serde_json::json!({}),
                        user_data: track_req.user_data,
                    }
                } else if let Some(ref ident) = track_req.identifier {
                    match state.source_manager.resolve(ident).await {
                        LoadResult::Track(t) => t,
                        _ => {
                            return Err((
                                StatusCode::NOT_FOUND,
                                format!("Track not found for identifier: {ident}"),
                            ))
                        }
                    }
                } else {
                    return Err((StatusCode::BAD_REQUEST, "Missing track identifier or encoded data".to_string()));
                };

                let event_sender = session.ws_sender.read().clone();
                player
                    .play(resolved_track, req.position, &state.source_manager, event_sender)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
            }
        }
    }

    Ok(Json(player.get_player_dto().await))
}

pub async fn delete_player(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((session_id, guild_id)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    check_auth(&headers, &state)?;
    let session = state
        .get_session(&session_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some((_, player)) = session.players.remove(&guild_id) {
        let _ = player.destroy().await;
    }

    Ok(StatusCode::NO_CONTENT)
}
