pub mod rest;
pub mod state;
pub mod ws;

use axum::{
    routing::{get, patch, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use crate::server::state::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/version", get(rest::get_version))
        .route("/v4/info", get(rest::get_info))
        .route("/v4/stats", get(rest::get_stats))
        .route("/v4/loadtracks", get(rest::load_tracks))
        .route("/v4/decode", post(rest::decode_single_track))
        .route("/v4/decodetracks", post(rest::decode_multiple_tracks))
        .route("/v4/websocket", get(ws::ws_handler))
        .route("/v4/sessions/{session_id}", patch(rest::update_session))
        .route("/v4/sessions/{session_id}/players", get(rest::get_players))
        .route(
            "/v4/sessions/{session_id}/players/{guild_id}",
            get(rest::get_player)
                .patch(rest::update_player)
                .delete(rest::delete_player),
        )
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
