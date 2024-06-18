use anyhow::Result;
use axum::{
    extract::DefaultBodyLimit, middleware::from_extractor, routing::{delete, get, post, put}, Router
};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
use tower_http::trace::TraceLayer;
use tracing::info;
use uuid::Uuid;

// WebSocket worker
mod ws;
use ws::handler;

// API: Auth
mod auth;
use auth::{self as api_auth, UManager};

// API: Server info
mod info;
use info as api_info;

// API: Profile
mod profile;
use profile as api_profile;

// Utils
mod utils;
use utils::update_advanced_users;

// Config
mod config;
use config::Config;

#[derive(Debug, Clone)]
pub struct AppState {
    /// User manager
    user_manager: Arc<UManager>,
    /// Send into WebSocket
    session: Arc<DashMap<Uuid, mpsc::Sender<Vec<u8>>>>,
    /// Ping broadcasts for WebSocket connections
    broadcasts: Arc<DashMap<Uuid, broadcast::Sender<Vec<u8>>>>,
    /// Current configuration
    config: Arc<Mutex<config::Config>>,
}

const LOGGER_ENV: &'static str = "RUST_LOG";
const SCULPTOR_VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    // "trace,axum=info,tower_http=info,tokio=info,tungstenite=info,tokio_tungstenite=info",
    let logger_env = std::env::var(LOGGER_ENV).unwrap_or_else(|_| "info".into());

    tracing_subscriber::fmt::fmt()
        .with_env_filter(
            logger_env
        )
        .pretty()
        .init();

    let config_file = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "Config.toml".into());

    info!("The Sculptor v{}", SCULPTOR_VERSION);
    // Config
    let config = Arc::new(Mutex::new(Config::parse(config_file.clone().into())));
    let listen = config.lock().await.listen.clone();

    // State
    let state = AppState {
        user_manager: Arc::new(UManager::new()),
        session: Arc::new(DashMap::new()),
        broadcasts: Arc::new(DashMap::new()),
        config: config,
    };

    // Automatic update of configuration while the server is running
    let config_update = Arc::clone(&state.config);
    let user_manager = Arc::clone(&state.user_manager);
    update_advanced_users(&config_update.lock().await.advanced_users, &user_manager);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            let new_config = Config::parse(config_file.clone().into());
            let mut config = config_update.lock().await;

            if new_config != *config {
                info!("Server configuration modification detected!");
                *config = new_config;
                update_advanced_users(&config.advanced_users, &user_manager);
            }
        }
    });

    let v1 = Router::new()
        .nest("/", ws::http2ws_router())
        .nest("/user", api_auth::router_v1());

    let api = Router::new()
        .nest("//auth", api_auth::router())
        .nest("/v1", v1)
        .route("/limits", get(api_info::limits))
        .route("/version", get(api_info::version))
        .route("/motd", get(api_info::motd))
        .route("/equip", post(api_profile::equip_avatar))
        .route("/:uuid", get(api_profile::user_info))
        .route("/:uuid/avatar", get(api_profile::download_avatar).layer(DefaultBodyLimit::disable()))
        .route("/avatar", put(api_profile::upload_avatar).layer(DefaultBodyLimit::disable()))
        .route("/avatar", delete(api_profile::delete_avatar));

    let app = Router::new()
        .nest("/api", api)
        .route("/api/", get(api_auth::status))
        .route("/ws", get(handler))
        .route("/health", get(|| async { "ok" }))
        .route_layer(from_extractor::<api_auth::Token>())
        .with_state(state)
        .layer(TraceLayer::new_for_http().on_request(()));

    let listener = tokio::net::TcpListener::bind(listen).await?;
    info!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    info!("Serve stopped. Closing...");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    info!("Terminate signal received");
}
