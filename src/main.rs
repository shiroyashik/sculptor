use anyhow::Result;
use axum::{
    extract::DefaultBodyLimit, middleware::from_extractor, routing::{delete, get, post, put}, Router
};
use dashmap::DashMap;
use utils::collect_advanced_users;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
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

// Config
mod config;

#[derive(Debug, Clone)]
pub struct AppState {
    /// Users with incomplete authentication
    //pending: Arc<DashMap<String, String>>, // <SHA1 serverId, USERNAME>
    /// Authenticated users
    //authenticated: Arc<Authenticated>, // <SHA1 serverId, Userinfo> NOTE: In the future, try it in a separate LockRw branch
    /// User manager
    user_manager: Arc<UManager>,
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
    let config = Arc::new(Mutex::new(config::Config::parse(config_file.clone().into())));
    let listen = config.lock().await.listen.clone();

    // State
    let state = AppState {
        user_manager: Arc::new(UManager::new()),
        broadcasts: Arc::new(DashMap::new()),
        config: config,
    };

    // Automatic update of configuration while the server is running
    let config_update = state.config.clone();
    let user_manager = Arc::clone(&state.user_manager);
    tokio::spawn(async move {
        loop {
            let new_config = config::Config::parse(config_file.clone().into());
            let mut config = config_update.lock().await;

            if new_config != *config {
                info!("Server configuration modification detected!");
                *config = new_config;
                // let collected = collect_advanced_users(&config.advanced_users);
                // for (uuid, userinfo) in collected {
                //     user_manager.insert_user(uuid, userinfo);
                // }
            }
            let collected = collect_advanced_users(&config.advanced_users);
            for (uuid, userinfo) in collected {
                user_manager.insert_user(uuid, userinfo);
            }
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    let api = Router::new()
        .nest("//auth", api_auth::router())
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
