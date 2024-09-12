use anyhow::Result;
use axum::{
    extract::DefaultBodyLimit, routing::{delete, get, post, put}, Router
};
use dashmap::DashMap;
use tracing_panic::panic_hook;
use tracing_subscriber::{fmt::{self, time::ChronoLocal}, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use std::{path::PathBuf, sync::Arc};
use tokio::{fs, sync::{broadcast, mpsc, RwLock}, time::Instant};
use tower_http::trace::TraceLayer;
use tracing::info;
use uuid::Uuid;

// Consts
mod consts;
pub use consts::*;

// Errors
pub use api::errors::{ApiResult, ApiError};

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
use utils::{check_updates, get_log_file, update_advanced_users, update_bans_from_minecraft, FiguraVersions};

#[derive(Debug, Clone)]
pub struct AppState {
    /// Uptime
    uptime: Instant,
    /// User manager
    user_manager: Arc<UManager>,
    /// Send into WebSocket
    session: Arc<DashMap<Uuid, mpsc::Sender<Vec<u8>>>>,
    /// Ping broadcasts for WebSocket connections
    broadcasts: Arc<DashMap<Uuid, broadcast::Sender<Vec<u8>>>>,
    /// Current configuration
    config: Arc<RwLock<state::Config>>,
    /// Figura Versions
    figura_versions: Arc<RwLock<Option<FiguraVersions>>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    // "trace,axum=info,tower_http=info,tokio=info,tungstenite=info,tokio_tungstenite=info",
    let logger_env = std::env::var(LOGGER_ENV).unwrap_or_else(|_| "info".into());
    let config_file = std::env::var(CONFIG_ENV).unwrap_or_else(|_| "Config.toml".into());
    let logs_folder = std::env::var(LOGS_ENV).unwrap_or_else(|_| "logs".into());

    let file_appender = tracing_appender::rolling::never(&logs_folder, get_log_file(&logs_folder));
    let timer = ChronoLocal::new(String::from("%Y-%m-%dT%H:%M:%S%.3f%:z"));

    let file_layer = fmt::layer()
        .with_ansi(false) // Disable ANSI colors for file logs
        .with_timer(timer.clone())
        .pretty()
        .with_writer(file_appender);

    // Create a layer for the terminal
    let terminal_layer = fmt::layer()
        .with_ansi(true)
        .with_timer(timer)
        .pretty()
        .with_writer(std::io::stdout);

    // Combine the layers and set the global subscriber
    tracing_subscriber::registry()
        .with(EnvFilter::from(logger_env))
        .with(file_layer)
        .with(terminal_layer)
        .init();

    std::panic::set_hook(Box::new(panic_hook));
    // let prev_hook = std::panic::take_hook();
    // std::panic::set_hook(Box::new(move |panic_info| {
    //     panic_hook(panic_info);
    //     prev_hook(panic_info);
    // }));

    info!("The Sculptor v{}{}", SCULPTOR_VERSION, check_updates(REPOSITORY, &SCULPTOR_VERSION).await?);
    
    {
        let path = PathBuf::from("avatars");
        if !path.exists() {
            fs::create_dir(path).await.expect("Can't create avatars folder!");
            info!("Created avatars directory");
        }
    }

    // Config
    let config = Arc::new(RwLock::new(Config::parse(config_file.clone().into())));
    let listen = config.read().await.listen.clone();

    // State
    let state = AppState {
        uptime: Instant::now(),
        user_manager: Arc::new(UManager::new()),
        session: Arc::new(DashMap::new()),
        broadcasts: Arc::new(DashMap::new()),
        figura_versions: Arc::new(RwLock::new(None)),
        config,
    };

    // Automatic update of configuration while the server is running
    let config_update = Arc::clone(&state.config);
    let user_manager = Arc::clone(&state.user_manager);
    update_advanced_users(&config_update.read().await.advanced_users.clone(), &user_manager);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            let new_config = Config::parse(config_file.clone().into());
            let mut config = config_update.write().await;

            if new_config != *config {
                info!("Server configuration modification detected!");
                *config = new_config;
                update_advanced_users(&config.advanced_users.clone(), &user_manager);
            }
        }
    });
    if state.config.read().await.mc_folder.exists() {
        tokio::spawn(update_bans_from_minecraft(
            state.config.read().await.mc_folder.clone(),
            Arc::clone(&state.user_manager)
        ));
    }

    let api = Router::new()
        .nest("//auth", api_auth::router()) // => /api//auth ¯\_(ツ)_/¯
        .nest("/v1", api::v1::router())
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
        .route("/api/", get(check_auth))
        .route("/ws", get(ws))
        .with_state(state)
        .layer(TraceLayer::new_for_http().on_request(()))
        .route("/health", get(|| async { "ok" }));

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
        () = ctrl_c => {
            println!();
            info!("Ctrl+C signal received");
        },
        () = terminate => {
            println!();
            info!("Terminate signal received");
        },
    }
}
