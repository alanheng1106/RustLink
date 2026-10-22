use rustlink::config::AppConfig;
use rustlink::server::create_router;
use rustlink::server::state::AppState;
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load or auto-generate application.yml configuration
    let config = AppConfig::load_or_create();

    // 2. Initialize logging based on config or env
    let default_filter = format!("rustlink={},tower_http=debug", config.logging.level);
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| default_filter.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 3. Resolve server settings (env vars take priority if set)
    let host = config.get_address();
    let port = config.get_port();
    let password = config.get_password();
    let sources = config.get_source_list();

    let state = AppState::new_with_sources(password.clone(), sources);

    // 4. Start background Session Reaper for zombie session garbage collection
    rustlink::server::reaper::start_session_reaper(state.clone());

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
