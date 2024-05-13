use anyhow::Result;
use axum::{
    extract::Path,
    routing::{delete, get, post, put},
    Router,
};
use chrono::prelude::*;
use dashmap::DashMap;
use fern::colors::{Color, ColoredLevelConfig};
use log::info;
use std::sync::{Arc, Mutex};
use tower_http::trace::TraceLayer;

// WebSocket worker
mod ws;
use ws::handler;

// API
mod auth;
use auth as api_auth;

#[derive(Debug, Clone)]
pub struct Userinfo {
    id: usize
}

#[derive(Debug, Clone)]
pub struct AppState {
    authenticated: Arc<Mutex<DashMap<String, String>>>, // <SHA1, USERNAME>
    pending: Arc<Mutex<DashMap<String, String>>>
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("The Sculptor");
    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .debug(Color::Magenta)
        .trace(Color::Cyan)
        .warn(Color::Yellow);
    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                Local::now().to_rfc3339_opts(SecondsFormat::Millis, true),
                colors.color(record.level()),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        // .level_for("hyper", log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .apply()?;

    // Config init here
    let listen = "0.0.0.0:6665";

    // State init here
    let state = AppState {
        authenticated: Arc::new(Mutex::new(DashMap::new())),
        pending: Arc::new(Mutex::new(DashMap::new()))
    };

    let api = Router::new()
        .nest(
            "//auth",
            api_auth::router()
        ) // check Auth; return 200 OK if token valid
        .route(
            "/limits",
            get(|| async { "@toomanylimits" })
        ) // Need more info :( TODO:
        .route(
            "/version",
            get(|| async { "{\"release\":\"2.7.1\",\"prerelease\":\"2.7.1\"}" }),
        )
        .route(
            "/motd",
            get(|| async { "\"written by an black cat :3 mew\"" }),
        )
        .route(
            "/equip",
            post(|| async { "Do it! NOW!" })
        ) // set Equipped; TODO:
        .route(
            "/:owner/:id",
            get(|Path((owner, id)): Path<(String, String)>| async move {
                format!("getting user {id}, owner {owner}")
            }),
        ) // get Avatar
        .route(
            "/:avatar",
            put(|Path(avatar): Path<String>| async move { format!("put {avatar}") }),
        ) // put Avatar
        .route(
            "/:avatar",
            delete(|Path(avatar): Path<String>| async move { format!("delete {avatar}") }),
        ); // delete Avatar

    let app = Router::new()
        .nest("/api", api)
        .route("/ws", get(handler))
        .layer(TraceLayer::new_for_http().on_request(()))
        .with_state(state);

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
