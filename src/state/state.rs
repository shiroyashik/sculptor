use std::sync::Arc;

use dashmap::DashMap;
use tokio::{sync::*, time::Instant};
use uuid::Uuid;

use crate::{api::figura::SessionMessage, auth::UManager, FiguraVersions};

#[derive(Debug, Clone)]
pub struct AppState {
    /// Uptime
    pub uptime: Instant,
    /// User manager
    pub user_manager: Arc<UManager>,
    /// Send into WebSocket
    pub session: Arc<DashMap<Uuid, mpsc::Sender<SessionMessage>>>,
    /// Send messages for subscribers
    pub subscribes: Arc<DashMap<Uuid, broadcast::Sender<Vec<u8>>>>,
    /// Current configuration
    pub config: Arc<RwLock<super::Config>>,
    /// Caching Figura Versions
    pub figura_versions: Arc<RwLock<Option<FiguraVersions>>>,
}