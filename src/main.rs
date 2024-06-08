use anyhow::Result;
use axum::{
    extract::DefaultBodyLimit, middleware::from_extractor, routing::{delete, get, post, put}, Router
};
use dashmap::DashMap;
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
use auth as api_auth;

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
pub struct Userinfo {
    username: String,
    uuid: Uuid,
    auth_system: api_auth::AuthSystem,
}

#[derive(Debug, Clone)]
struct Authenticated {
    user_data: DashMap<String, Userinfo>,
    uuid: DashMap<Uuid, String>,
}

impl Authenticated {
    fn new() -> Self {
        Self {
            user_data: DashMap::new(),
            uuid: DashMap::new(),
        }
    }
    pub fn insert(&self, uuid: Uuid, token: String, userinfo: Userinfo) -> Option<Userinfo> {
        self.uuid.insert(uuid, token.clone());
        self.user_data.insert(token, userinfo)
    }
    pub fn get(
        &self,
        token: &String,
    ) -> Option<dashmap::mapref::one::Ref<'_, std::string::String, Userinfo>> {
        self.user_data.get(token)
    }
    pub fn get_by_uuid(
        &self,
        uuid: &Uuid,
    ) -> Option<dashmap::mapref::one::Ref<'_, std::string::String, Userinfo>> {
        if let Some(token) = self.uuid.get(uuid) {
            self.user_data.get(&token.clone())
        } else {
            None
        }
    }
    pub fn contains_token(&self, token: &String) -> bool {
        self.user_data.contains_key(token)
    }
    pub fn remove(&self, uuid: &Uuid) {
        let token = self.uuid.remove(uuid).unwrap().1;
        self.user_data.remove(&token);
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    // Users with incomplete authentication
    pending: Arc<DashMap<String, String>>, // <SHA1 serverId, USERNAME>
    // Authenticated users
    authenticated: Arc<Authenticated>, // <SHA1 serverId, Userinfo> NOTE: In the future, try it in a separate LockRw branch
    // Ping broadcasts for WebSocket connections
    broadcasts: Arc<DashMap<Uuid, broadcast::Sender<Vec<u8>>>>,
    // Current configuration
    config: Arc<Mutex<config::Config>>,
}

const LOGGER_ENV: &'static str = "RUST_LOG";
const SCULPTOR_VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
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
        pending: Arc::new(DashMap::new()),
        authenticated: Arc::new(Authenticated::new()),
        broadcasts: Arc::new(DashMap::new()),
        config: config,
    };

    // Automatic update of configuration while the server is running
    let config_update = state.config.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;

            let new_config = config::Config::parse(config_file.clone().into());
            let mut config = config_update.lock().await;

            if new_config != *config {
                info!("Server configuration modification detected!");
                *config = new_config;
            }
        }
    });

    let max_body_size = state.config.clone().lock().await.limitations.max_avatar_size as usize;
    let api = Router::new()
        .nest("//auth", api_auth::router())
        .route("/limits", get(api_info::limits))
        .route("/version", get(api_info::version))
        .route("/motd", get(api_info::motd))
        .route("/equip", post(api_profile::equip_avatar))
        .route("/:uuid", get(api_profile::user_info))
        .route("/:uuid/avatar", get(api_profile::download_avatar).layer(DefaultBodyLimit::max(max_body_size)))
        .route("/avatar", put(api_profile::upload_avatar).layer(DefaultBodyLimit::max(max_body_size)))
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
