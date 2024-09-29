use anyhow::Result;
use axum::{
    extract::DefaultBodyLimit, routing::{delete, get, post, put}, Router
};
use dashmap::DashMap;
use tracing_panic::panic_hook;
use tracing_subscriber::{fmt::{self, time::ChronoLocal}, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use std::{path::PathBuf, sync::Arc, env::var};
use tokio::{fs, sync::{broadcast, mpsc, RwLock}, time::Instant};
use tower_http::trace::TraceLayer;
use uuid::Uuid;
use lazy_static::lazy_static;

// Consts
mod consts;
pub use consts::*;

// Errors
pub use api::errors::{ApiResult, ApiError};

// API
mod api;
use api::{
    figura::{ws, info as api_info, profile as api_profile, auth as api_auth, assets as api_assets},
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
use utils::*;

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
    /// Caching Figura Versions
    figura_versions: Arc<RwLock<Option<FiguraVersions>>>,
}

lazy_static! {
    pub static ref LOGGER_VAR: String = {
        var(LOGGER_ENV).unwrap_or(String::from("info"))
    };
    pub static ref CONFIG_VAR: String = {
        var(CONFIG_ENV).unwrap_or(String::from("Config.toml"))
    };
    pub static ref LOGS_VAR: String = {
        var(LOGS_ENV).unwrap_or(String::from("logs"))
    };
    pub static ref ASSETS_VAR: String = {
        var(ASSETS_ENV).unwrap_or(String::from("data/assets"))
    };
    pub static ref AVATARS_VAR: String = {
        var(AVATARS_ENV).unwrap_or(String::from("data/avatars"))
    };
}

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Set up env
    let _ = dotenvy::dotenv();

    // 2. Set up logging
    let file_appender = tracing_appender::rolling::never(&*LOGS_VAR, get_log_file(&*LOGS_VAR));
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
        .with(EnvFilter::from(&*LOGGER_VAR))
        .with(file_layer)
        .with(terminal_layer)
        .init();

    // std::panic::set_hook(Box::new(panic_hook));
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        panic_hook(panic_info);
        prev_hook(panic_info);
    }));

    // 3. Display info about current instance and check updates
    tracing::info!("The Sculptor v{SCULPTOR_VERSION} ({REPOSITORY})");
    // let _ = check_updates(REPOSITORY, SCULPTOR_VERSION).await; // Currently, there is no need to do anything with the result of the function

    match get_latest_version(REPOSITORY).await {
        Ok(latest_version) => {
            if latest_version > semver::Version::parse(SCULPTOR_VERSION).expect("SCULPTOR_VERSION does not match SemVer!") {
                tracing::info!("Available new v{latest_version}! Check https://github.com/{REPOSITORY}/releases");
            } else {
                tracing::info!("Sculptor are up to date!");
            }
        },
        Err(e) => {
            tracing::error!("Can't fetch Sculptor updates due: {e:?}");
        },
    }

    // 4. Starting an app() that starts to serve. If app() returns true, the sculptor will be restarted. for future
    loop {
        if !app().await? {
            break;
        }
    }

    Ok(())
}

async fn app() -> Result<bool> {
    // Preparing for launch
    {
        let path = PathBuf::from(&*AVATARS_VAR);
        if !path.exists() {
            fs::create_dir_all(path).await.expect("Can't create avatars folder!");
            tracing::info!("Created avatars directory");
        }
    }

    // Config
    let config = Arc::new(RwLock::new(Config::parse(CONFIG_VAR.clone().into())));
    let listen = config.read().await.listen.clone();
    let limit = get_limit_as_bytes(config.read().await.limitations.max_avatar_size.clone() as usize);

    if config.read().await.assets_updater_enabled {
        // Force update assets if folder or hash file doesn't exists.
        if !(PathBuf::from(&*ASSETS_VAR).is_dir() && get_path_to_assets_hash().is_file()) {
            tracing::debug!("Removing broken assets...");
            remove_assets().await
        }
        match get_commit_sha(FIGURA_ASSETS_COMMIT_URL).await {
            Ok(sha) => {
                if is_assets_outdated(&sha).await.unwrap_or_else(|e| {tracing::error!("Can't check assets state due: {:?}", e); false}) {
                    remove_assets().await;
                    match tokio::task::spawn_blocking(|| { download_assets() }).await.unwrap() {
                        Err(e) => tracing::error!("Assets outdated! Can't download new version due: {:?}", e),
                        Ok(_) => {
                            match write_sha_to_file(&sha).await {
                                Ok(_) => tracing::info!("Assets successfully updated!"),
                                Err(e) => tracing::error!("Assets successfully updated! Can't create assets hash file due: {:?}", e),
                            }
                        }
                    };
                } else { tracing::info!("Assets are up to date!") }
            },
            Err(e) => tracing::error!("Can't get assets last commit! Assets update check aborted due {:?}", e)
        }
    }

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
            let new_config = Config::parse(CONFIG_VAR.clone().into());
            let mut config = config_update.write().await;

            if new_config != *config {
                tracing::info!("Server configuration modification detected!");
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
        .nest("//assets", api_assets::router())
        .nest("/v1", api::v1::router(limit))
        .route("/limits", get(api_info::limits))
        .route("/version", get(api_info::version))
        .route("/motd", get(api_info::motd))
        .route("/equip", post(api_profile::equip_avatar))
        .route("/:uuid", get(api_profile::user_info))
        .route("/:uuid/avatar", get(api_profile::download_avatar))
        .route("/avatar", put(api_profile::upload_avatar).layer(DefaultBodyLimit::max(limit)))
        .route("/avatar", delete(api_profile::delete_avatar));

    let app = Router::new()
        .nest("/api", api)
        .route("/api/", get(check_auth))
        .route("/ws", get(ws))
        .with_state(state)
        .layer(TraceLayer::new_for_http().on_request(()))
        .route("/health", get(|| async { "ok" }));

    let listener = tokio::net::TcpListener::bind(listen).await?;
    tracing::info!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    tracing::info!("Serve stopped.");
    Ok(false)
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
            tracing::info!("Ctrl+C signal received");
        },
        () = terminate => {
            println!();
            tracing::info!("Terminate signal received");
        },
    }
}
