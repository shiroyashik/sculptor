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

// // WebSocket worker
// mod ws;
// use ws::handler;

// // API: Auth
// mod auth;
// use auth::{self as api_auth, UManager};

// // API: Server info
// mod info;
// use info as api_info;

// // API: Profile
// mod profile;
// use profile as api_profile;

// API
mod api;
use api::{
    figura::{ws, info as api_info, profile as api_profile, auth as api_auth},
    // v1::{},
};

// Auth
mod auth;
use auth::{UManager, check_auth};

// Config
mod state;
use state::Config;

// Utils
mod utils;
use utils::{check_updates, update_advanced_users};

// // Config
// mod config;
// use config::Config;

#[derive(Debug, Clone)]
pub struct AppState {
    /// User manager
    user_manager: Arc<UManager>,
    /// Send into WebSocket
    session: Arc<DashMap<Uuid, mpsc::Sender<Vec<u8>>>>,
    /// Ping broadcasts for WebSocket connections
    broadcasts: Arc<DashMap<Uuid, broadcast::Sender<Vec<u8>>>>,
    /// Current configuration
    config: Arc<Mutex<state::Config>>,
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

    info!("The Sculptor v{}{}", SCULPTOR_VERSION, check_updates("shiroyashik/sculptor", &SCULPTOR_VERSION).await?);
    
    let config_file = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "Config.toml".into());
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

    let api = Router::new()
        .nest("//auth", api_auth::router())
        .nest("/v1", api::v1::router())
        .route("/", get(check_auth))
        .route("/limits", get(api_info::limits))
        .route("/version", get(api_info::version))
        .route("/motd", get(api_info::motd))
        .route("/equip", post(api_profile::equip_avatar))
        .route("/:uuid", get(api_profile::user_info))
        .route("/:uuid/avatar", get(api_profile::download_avatar))
        .route("/avatar", put(api_profile::upload_avatar).layer(DefaultBodyLimit::disable()))
        .route("/avatar", delete(api_profile::delete_avatar));

    let app = Router::new()
        .nest("/api", api)
        .route("/ws", get(ws))
        .route("/health", get(|| async { "ok" }))
        .route_layer(from_extractor::<auth::Token>())
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
