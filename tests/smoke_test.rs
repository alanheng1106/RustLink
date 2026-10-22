use axum::http::{Request, StatusCode};
use rustlink::audio::codec::{decode_track, encode_track};
use rustlink::model::protocol::TrackInfo;
use rustlink::server::create_router;
use rustlink::server::state::AppState;
use tower::ServiceExt;

#[test]
fn test_track_codec_roundtrip() {
    let original = TrackInfo {
        identifier: "https://example.com/audio.mp3".to_string(),
        is_seekable: true,
        author: "Test Author".to_string(),
        length: 120000,
        is_stream: false,
        position: 0,
        title: "Test Song".to_string(),
        uri: Some("https://example.com/audio.mp3".to_string()),
        artwork_url: None,
        isrc: None,
        source_name: "http".to_string(),
    };

    let encoded = encode_track(&original);
    assert!(!encoded.is_empty());

    let decoded = decode_track(&encoded).expect("Failed to decode track");
    assert_eq!(decoded.title, original.title);
    assert_eq!(decoded.identifier, original.identifier);
    assert_eq!(decoded.author, original.author);
}

#[tokio::test]
async fn test_rest_version_and_info() {
    let state = AppState::new("youshallnotpass".to_string());
    let app = create_router(state);

    // 1. Test /version
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/version")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // 2. Test /v4/info without auth -> 401 Unauthorized
    let unauth_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v4/info")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(unauth_response.status(), StatusCode::UNAUTHORIZED);

    // 3. Test /v4/info with auth -> 200 OK
    let auth_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v4/info")
                .header("authorization", "youshallnotpass")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(auth_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_player_rest_lifecycle() {
    let state = AppState::new("youshallnotpass".to_string());
    let app = create_router(state.clone());

    // 1. Create a session in state
    let (session, _) = state.get_or_create_session("test-session-123", Some("123456789".to_string()));

    // 2. Query players for session -> should be empty
    let list_req = Request::builder()
        .uri("/v4/sessions/test-session-123/players")
        .header("authorization", "youshallnotpass")
        .body(axum::body::Body::empty())
        .unwrap();

    let list_res = app.clone().oneshot(list_req).await.unwrap();
    assert_eq!(list_res.status(), StatusCode::OK);

    // 3. Update player (set volume to 80, paused to true)
    let update_body = serde_json::json!({
        "volume": 80,
        "paused": true
    });

    let update_req = Request::builder()
        .method("PATCH")
        .uri("/v4/sessions/test-session-123/players/987654321")
        .header("authorization", "youshallnotpass")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(serde_json::to_vec(&update_body).unwrap()))
        .unwrap();

    let update_res = app.clone().oneshot(update_req).await.unwrap();
    assert_eq!(update_res.status(), StatusCode::OK);

    // 4. Player should now exist in session
    assert_eq!(session.players.len(), 1);
    {
        let player = session.players.get("987654321").unwrap();
        assert_eq!(player.volume.load(std::sync::atomic::Ordering::Relaxed), 80);
        assert!(player.paused.load(std::sync::atomic::Ordering::Relaxed));
    } // drop DashMap read lock before delete_player acquires write lock

    // 5. Delete player
    let del_req = Request::builder()
        .method("DELETE")
        .uri("/v4/sessions/test-session-123/players/987654321")
        .header("authorization", "youshallnotpass")
        .body(axum::body::Body::empty())
        .unwrap();

    let del_res = app.clone().oneshot(del_req).await.unwrap();
    assert_eq!(del_res.status(), StatusCode::NO_CONTENT);
    assert_eq!(session.players.len(), 0);
}

#[tokio::test]
async fn test_youtube_source_input() {
    let source_manager = rustlink::audio::source::SourceManager::new();

    // 1. YouTube direct watch URL input creation
    let yt_url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
    let input1 = source_manager.create_input(yt_url);
    assert!(input1.is_some());

    // 2. YouTube short URL (youtu.be) input creation
    let yt_short = "https://youtu.be/dQw4w9WgXcQ";
    let input2 = source_manager.create_input(yt_short);
    assert!(input2.is_some());

    // 3. YouTube search input creation
    let yt_search = "ytsearch:never gonna give you up";
    let input3 = source_manager.create_input(yt_search);
    assert!(input3.is_some());
}

#[test]
fn test_config_parsing() {
    let yaml = r#"
server:
  port: 8080
  address: "127.0.0.1"
  password: "custom_password"

sources:
  youtube: true
  http: true
  local: false
  hls: true
"#;

    let cfg: rustlink::config::AppConfig = serde_yaml::from_str(yaml).expect("Failed to parse config");
    assert_eq!(cfg.get_port(), 8080);
    assert_eq!(cfg.get_address(), "127.0.0.1");
    assert_eq!(cfg.get_password(), "custom_password");
    assert!(!cfg.get_source_list().contains(&"local".to_string()));
    assert!(cfg.get_source_list().contains(&"youtube".to_string()));
}

#[test]
fn test_serialize_server_messages() {
    use rustlink::model::protocol::{CpuStats, MemoryStats, ServerMessage, StatsResponse};

    let stats = StatsResponse {
        players: 1,
        playing_players: 1,
        uptime: 1000,
        memory: MemoryStats {
            free: 100,
            used: 200,
            allocated: 300,
            reservable: 400,
        },
        cpu: CpuStats {
            cores: 4,
            system_load: 0.1,
            lavalink_load: 0.05,
        },
        frame_stats: None,
    };

    let msg = ServerMessage::Stats(stats);
    let json = serde_json::to_string(&msg);
    println!("Serialized Stats: {:?}", json);
    assert!(json.is_ok());

    let track = rustlink::model::protocol::Track {
        encoded: "test".to_string(),
        info: TrackInfo {
            identifier: "id".to_string(),
            is_seekable: true,
            author: "a".to_string(),
            length: 1000,
            is_stream: false,
            position: 0,
            title: "t".to_string(),
            uri: None,
            artwork_url: None,
            isrc: None,
            source_name: "http".to_string(),
        },
        plugin_info: serde_json::json!({}),
        user_data: serde_json::json!({}),
    };
    let ready = ServerMessage::Ready {
        resumed: false,
        session_id: "test-session".to_string(),
    };
    let ready_json = serde_json::to_string(&ready).unwrap();
    println!("Serialized Ready: {}", ready_json);

    let event = ServerMessage::Event(rustlink::model::protocol::ServerEvent::TrackStartEvent {
        guild_id: "123".to_string(),
        track,
    });
    let event_json = serde_json::to_string(&event).unwrap();
    println!("Serialized Event: {}", event_json);
    assert_eq!(ready_json, "{\"op\":\"ready\",\"resumed\":false,\"sessionId\":\"test-session\"}");
    assert!(event_json.contains("\"type\":\"TrackStartEvent\""));
    assert!(event_json.contains("\"guildId\":\"123\""));
}
