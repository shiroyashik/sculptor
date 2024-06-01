use anyhow::Result;
use axum::{
    middleware::from_extractor,
    routing::{delete, get, post, put},
    Router,
};
use dashmap::DashMap;
use log::info;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tower_http::trace::TraceLayer;
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
    // Advanced configured users
    advanced_users: Arc<Mutex<toml::Table>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::fmt()
        .with_env_filter("trace,axum=info,tower_http=info,tokio=info,tungstenite=info,tokio_tungstenite=info")
        .pretty()
        .init();

    let config_file = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "Config.toml".into());

    info!("The Sculptor MMSI edition v{}", env!("CARGO_PKG_VERSION"));
    // Config
    let config = config::Config::parse(config_file.clone().into());
    let listen = config.listen.as_str();

    // State
    let state = AppState {
        pending: Arc::new(DashMap::new()),
        authenticated: Arc::new(Authenticated::new()),
        broadcasts: Arc::new(DashMap::new()),
        advanced_users: Arc::new(Mutex::new(config.advanced_users)),
    };

    // Automatic update of advanced_users while the server is running
    let advanced_users = state.advanced_users.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;

            let new_config = config::Config::parse(config_file.clone().into()).advanced_users;
            let mut config = advanced_users.lock().await;

            if new_config != *config {
                *config = new_config;
            }
        }
    });

    let api = Router::new()
        .nest("//auth", api_auth::router())
        .route("/limits", get(api_info::limits)) // TODO:
        .route("/version", get(api_info::version))
        .route("/motd", get(|| async { config.motd }))
        .route("/equip", post(api_profile::equip_avatar))
        .route("/:uuid", get(api_profile::user_info))
        .route("/:uuid/avatar", get(api_profile::download_avatar))
        .route("/avatar", put(api_profile::upload_avatar))
        .route("/avatar", delete(api_profile::delete_avatar)); // delete Avatar

    let app = Router::new()
        .nest("/api", api)
        .route("/api/", get(api_auth::status))
        .route("/ws", get(handler))
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
