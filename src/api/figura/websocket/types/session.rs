use dashmap::DashMap;
use tokio::{sync::{broadcast, mpsc}, task::AbortHandle};
// use uuid::Uuid;

pub struct WSSession {
    pub user: crate::auth::Userinfo,
    pub own_tx: mpsc::Sender<SessionMessage>,
    pub own_rx: mpsc::Receiver<SessionMessage>,
    pub subs_tx: broadcast::Sender<Vec<u8>>,
    pub sub_workers_aborthandles: DashMap<uuid::Uuid, AbortHandle>,
}

pub enum SessionMessage {
    Ping(Vec<u8>),
    // Sub(Uuid),
    Banned,
}