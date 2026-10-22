use rustlink::server::create_router;
use rustlink::server::state::AppState;
use std::env;
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rustlink=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 2. Read configuration from environment or defaults (supports both RUSTLINK_ and LAVALINK_ prefixes)
    let host = env::var("RUSTLINK_SERVER_HOST")
        .or_else(|_| env::var("LAVALINK_SERVER_HOST"))
        .unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("RUSTLINK_SERVER_PORT")
        .or_else(|_| env::var("LAVALINK_SERVER_PORT"))
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(2333);
    let password = env::var("RUSTLINK_SERVER_PASSWORD")
        .or_else(|_| env::var("LAVALINK_SERVER_PASSWORD"))
        .unwrap_or_else(|_| "youshallnotpass".to_string());

    let state = AppState::new(password.clone());
    let app = create_router(state);

    let addr: SocketAddr = format!("{host}:{port}").parse()?;

    println!(r#"
     ____            _   _     _       _    
    |  _ \ _   _ ___| |_| |   (_)_ __ | | __
    | |_) | | | / __| __| |   | | '_ \| |/ /
    |  _ <| |_| \__ \ |_| |___| | | | |   < 
    |_| \_\\__,_|___/\__|_____|_|_| |_|_|\_\
                                            
    RustLink - High Performance Lavalink v4 Audio Node
    Ready for Discord voice connections
    "#);

    info!(
        bind_address = %addr,
        password = %password,
        "RustLink server listening for Lavalink v4 clients"
    );

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
