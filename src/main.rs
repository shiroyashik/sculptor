#![allow(clippy::module_inception)]
use anyhow::Result;
use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{Request, StatusCode, header::HOST, uri::Authority},
    response::Redirect,
    routing::{any, delete, get, post, put},
};
use axum_server::tls_rustls::RustlsConfig;
use dashmap::DashMap;
use std::{
    env::var,
    net::{SocketAddr, TcpListener as StdTcpListener},
    path::PathBuf,
    sync::{Arc, LazyLock},
};
use tokio::{
    fs,
    sync::{RwLock, watch},
    time::Instant,
};
use tower_http::trace::TraceLayer;
use tracing_panic::panic_hook;
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, time::ChronoLocal},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

// Consts
mod consts;
pub use consts::*;

// Errors
pub use api::errors::{ApiError, ApiResult};

// Metrics
mod metrics;
pub use metrics::*;

// API
mod api;
use api::figura::{
    assets as api_assets, auth as api_auth, info as api_info, profile as api_profile, ws,
};

// Auth
mod auth;
use auth::{UManager, check_auth};

// Config
mod state;
use state::{AppState, Config};

// Utils
mod utils;
use utils::*;

pub static LOGGER_VAR: LazyLock<String> =
    LazyLock::new(|| var(LOGGER_ENV).unwrap_or(String::from("info")));
pub static CONFIG_VAR: LazyLock<String> =
    LazyLock::new(|| var(CONFIG_ENV).unwrap_or(String::from("Config.toml")));
pub static LOGS_VAR: LazyLock<String> =
    LazyLock::new(|| var(LOGS_ENV).unwrap_or(String::from("logs")));
pub static ASSETS_VAR: LazyLock<String> =
    LazyLock::new(|| var(ASSETS_ENV).unwrap_or(String::from("data/assets")));
pub static AVATARS_VAR: LazyLock<String> =
    LazyLock::new(|| var(AVATARS_ENV).unwrap_or(String::from("data/avatars")));

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Set up env
    let _ = dotenvy::dotenv();

    // 2. Set up logging
    let file_appender = tracing_appender::rolling::never(&*LOGS_VAR, get_log_file(&LOGS_VAR));
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

    let http_client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .unwrap();

    match get_latest_version(http_client.clone(), REPOSITORY).await {
        Ok(latest_version) => {
            if latest_version
                > semver::Version::parse(SCULPTOR_VERSION)
                    .expect("SCULPTOR_VERSION does not match SemVer!")
            {
                tracing::info!(
                    "Available new v{latest_version}! Check https://github.com/{REPOSITORY}/releases"
                );
            } else {
                tracing::info!("Sculptor are up to date!");
            }
        }
        Err(e) => {
            tracing::error!("Can't fetch Sculptor updates due: {e:?}");
        }
    }

    // Creating avatars folder
    let path = PathBuf::from(&*AVATARS_VAR);
    if !path.exists() {
        fs::create_dir_all(path)
            .await
            .expect("Can't create avatars folder!");
        tracing::info!("Created avatars directory");
    }

    // 4. Starting an app() that starts to serve. If app() returns true, the sculptor will be restarted. TODO: for future
    loop {
        if !app(http_client.clone()).await? {
            break;
        }
    }

    Ok(())
}

async fn app(http_client: reqwest::Client) -> Result<bool> {
    // Config
    let config = Config::parse(CONFIG_VAR.clone().into());
    let listen = config.listen.clone();
    let limit = get_limit_as_bytes(config.limitations.max_avatar_size as usize);

    if config.assets_updater_enabled {
        // Force update assets if folder or hash file doesn't exists.
        if !(PathBuf::from(&*ASSETS_VAR).is_dir() && get_path_to_assets_hash().is_file()) {
            tracing::debug!("Removing broken assets...");
            remove_assets().await
        }
        match get_commit_sha(http_client.clone(), FIGURA_ASSETS_COMMIT_URL).await {
            Ok(sha) => {
                if is_assets_outdated(&sha).await.unwrap_or_else(|e| {
                    tracing::error!("Can't check assets state due: {:?}", e);
                    false
                }) {
                    remove_assets().await;
                    match download_assets(http_client.clone()).await {
                        Err(e) => tracing::error!(
                            "Assets outdated! Can't download new version due: {:?}",
                            e
                        ),
                        Ok(_) => match write_sha_to_file(&sha).await {
                            Ok(_) => tracing::info!("Assets successfully updated!"),
                            Err(e) => tracing::error!(
                                "Assets successfully updated! Can't create assets hash file due: {:?}",
                                e
                            ),
                        },
                    };
                } else {
                    tracing::info!("Assets are up to date!")
                }
            }
            Err(e) => tracing::error!(
                "Can't get assets last commit! Assets update check aborted due {:?}",
                e
            ),
        }
    }

    // State
    let state = AppState {
        uptime: Instant::now(),
        user_manager: Arc::new(UManager::new()),
        session: Arc::new(DashMap::new()),
        subscribes: Arc::new(DashMap::new()),
        heartbeat_timestamps: Arc::new(DashMap::new()),
        config: Arc::new(RwLock::new(config.clone())),
        figura_versions: Arc::new(RwLock::new(None)),
        http_client,
    };

    // Automatic update of configuration/ban list while the server is running
    tokio::spawn(update_advanced_users(
        CONFIG_VAR.clone().into(),
        Arc::clone(&state.user_manager),
        Arc::clone(&state.session),
        Arc::clone(&state.config),
    ));
    // Blacklist auto update
    if config.mc_folder.exists() {
        tokio::spawn(update_bans_from_minecraft(
            state.config.read().await.mc_folder.clone(),
            Arc::clone(&state.user_manager),
            Arc::clone(&state.session),
        ));
    }
    tokio::spawn(heartbeat_monitor(state.clone()));

    let api = Router::new()
        .nest("//auth", api_auth::router()) // => /api//auth ¯\_(ツ)_/¯
        .nest("//assets", api_assets::router())
        .nest("/auth", api_auth::router())
        .nest("/assets", api_assets::router())
        .nest("/v1", api::sculptor::router(limit))
        .route("/limits", get(api_info::limits))
        .route("/version", get(api_info::version))
        .route("/motd", get(api_info::motd))
        .route("/equip", post(api_profile::equip_avatar))
        .route("/{uuid}", get(api_profile::user_info))
        .route("/{uuid}/avatar", get(api_profile::download_avatar))
        .route(
            "/avatar",
            put(api_profile::upload_avatar).layer(DefaultBodyLimit::max(limit)),
        )
        .route("/avatar", delete(api_profile::delete_avatar));

    let app = Router::new()
        .nest("/api", api)
        .route("/api/", get(check_auth))
        .route("/ws", get(ws))
        .merge(metrics::metrics_router(config.metrics_enabled))
        .with_state(state)
        .layer(TraceLayer::new_for_http()
            // .on_request(|request: &axum::http::Request<_>, _span: &tracing::Span| {
            //     // only for developing purposes
            //     tracing::trace!(headers = ?request.headers(), "started processing request");
            // })
            .on_response(|response: &axum::http::Response<_>, latency: std::time::Duration, _span: &tracing::Span| {
                tracing::trace!(latency = ?latency, status = ?response.status(), "finished processing request");
            })
            .on_request(())
        )
        .layer(axum::middleware::from_fn(track_metrics))
        .route("/health", get(|| async { "ok" }));

    if let Some(tls) = config.tls {
        let https_listen: SocketAddr = tls.https_listen.parse()?;
        let tls_config = RustlsConfig::from_pem_file(tls.cert, tls.key).await?;
        let http_listener = tokio::net::TcpListener::bind(&listen).await?;
        let http_addr = http_listener.local_addr()?;
        let https_listener = StdTcpListener::bind(https_listen)?;
        https_listener.set_nonblocking(true)?;
        let https_addr = https_listener.local_addr()?;
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        tracing::info!("Listening for HTTP redirects on {http_addr}");
        tracing::info!("Listening for HTTPS on {https_addr}");

        tokio::spawn(async move {
            shutdown_signal().await;
            let _ = shutdown_tx.send(true);
        });

        let https_handle = axum_server::Handle::new();
        let https_shutdown_handle = https_handle.clone();
        let mut https_shutdown_rx = shutdown_rx.clone();
        tokio::spawn(async move {
            wait_for_shutdown(&mut https_shutdown_rx).await;
            https_shutdown_handle.graceful_shutdown(None);
        });

        let redirect_app = Router::new().fallback(any(redirect_to_https));
        let mut http_shutdown_rx = shutdown_rx;
        let http_server = axum::serve(http_listener, redirect_app)
            .with_graceful_shutdown(async move { wait_for_shutdown(&mut http_shutdown_rx).await });
        let https_server = axum_server::from_tcp_rustls(https_listener, tls_config)?
            .handle(https_handle)
            .serve(app.into_make_service());

        tokio::try_join!(http_server, https_server)?;
    } else {
        let listener = tokio::net::TcpListener::bind(listen).await?;
        tracing::info!("Listening for HTTP on {}", listener.local_addr()?);

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;
    }

    tracing::info!("Serve stopped.");
    Ok(false)
}

async fn redirect_to_https(
    request: Request<axum::body::Body>,
) -> std::result::Result<Redirect, StatusCode> {
    let authority = request
        .headers()
        .get(HOST)
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .parse::<Authority>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    if authority.as_str().contains('@') {
        return Err(StatusCode::BAD_REQUEST);
    }

    let host = authority.host();
    let host = if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_owned()
    };
    let location = format!(
        "https://{host}{}",
        request
            .uri()
            .path_and_query()
            .map_or("/", |value| value.as_str())
    );

    Ok(Redirect::permanent(&location))
}

async fn wait_for_shutdown(receiver: &mut watch::Receiver<bool>) {
    if !*receiver.borrow() {
        let _ = receiver.changed().await;
    }
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
            tracing::info!("Ctrl+C signal received");
        },
        () = terminate => {
            tracing::info!("Terminate signal received");
        },
    }
}
