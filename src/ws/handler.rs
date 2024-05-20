use std::sync::Arc;

use axum::{extract::{ws::{Message, WebSocket}, State, WebSocketUpgrade}, response::Response};
use dashmap::DashMap;
use log::{debug, error, info, trace, warn};
use tokio::sync::{broadcast::{self, Receiver}, mpsc, Notify};
use uuid::Uuid;

use crate::{ws::{C2SMessage, S2CMessage}, AppState};

pub async fn handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

#[derive(Debug, Clone)]
struct WSOwner(Option<WSUser>);

#[derive(Debug, Clone)]
struct WSUser {
    username: String,
    token: String,
    uuid: Uuid,
}

impl WSOwner {
    fn name(&self) -> String {
        if let Some(user) = &self.0 {
            format!(" ({})", user.username)
        } else {
            String::new()
        }
    }
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    debug!("[WebSocket] New unknown connection!");
    let mut owner = WSOwner(None);
    let cutoff: DashMap<Uuid, Arc<Notify>> = DashMap::new();
    let (mtx, mut mrx) = mpsc::channel(64);
    let mut bctx: Option<broadcast::Sender<Vec<u8>>> = None;
    loop {
        tokio::select! {
            Some(msg) = socket.recv() => {
                trace!("[WebSocket{}] Raw: {msg:?}", owner.name());
                let mut msg = if let Ok(msg) = msg {
                    if let Message::Close(_) = msg {
                        info!("[WebSocket{}] Connection successfully closed!", owner.name());
                        if let Some(u) = owner.0 {
                            remove_broadcast(state.broadcasts.clone(), u.uuid).await;
                        }
                        return;
                    }
                    msg
                } else {
                    warn!("[WebSocket{}] Receive error! Connection terminated!", owner.name());
                    if let Some(u) = owner.0 {
                        remove_broadcast(state.broadcasts.clone(), u.uuid).await;
                    }
                    return;
                };
                // Далее код для обработки msg
                let msg_vec = msg.clone().into_data();
                let msg_array = msg_vec.as_slice();
                
                let newmsg = match C2SMessage::try_from(msg_array) {
                    Ok(data) => data,
                    Err(e) => {
                        error!("[WebSocket{}] This message is not from Figura! {e:?}", owner.name());
                        if let Some(u) = owner.0 {
                            remove_broadcast(state.broadcasts.clone(), u.uuid).await;
                        }
                        return;
                    },
                };
        
                debug!("[WebSocket{}] Raw: {newmsg:?}", owner.name());
        
                match newmsg {
                    C2SMessage::Token(token) => { // FIXME: Написать переменную спомощью которой бужет проверяться авторизовался ли пользователь или нет
                    debug!("[WebSocket{}] C2S : Token", owner.name());
                        let token = String::from_utf8(token.to_vec()).unwrap();
                        let authenticated = state.authenticated.lock().await;
                        match authenticated.get(&token) { // Принцип прост: если токена в authenticated нет, значит это trash
                            Some(t) => {
                                //username = t.username.clone();
                                owner.0 = Some(WSUser { username: t.username.clone(), token, uuid: t.uuid });
                                msg = Message::Binary(S2CMessage::Auth.to_vec());
                                let bcs = state.broadcasts.lock().await;
                                match bcs.get(&t.uuid) {
                                    Some(tx) => {
                                        bctx = Some(tx.to_owned());
                                    },
                                    None => {
                                        let (tx, _rx) = broadcast::channel(64);
                                        bcs.insert(t.uuid, tx.clone());
                                        bctx = Some(tx.to_owned());
                                    },
                                };
                            },
                            None => {
                                warn!("[WebSocket] Authenticaton error! Connection terminated!");
                                debug!("[WebSocket] Tried to log in with {token}"); // Tried to log in with token: {token}
                                if let Some(u) = owner.0 {
                                    remove_broadcast(state.broadcasts.clone(), u.uuid).await;
                                }
                                return; // TODO: Прописать код отключения
                            },
                        };
                    },
                    C2SMessage::Ping(_, _, _) => {
                        debug!("[WebSocket{}] C2S : Ping", owner.name());
                        let data = into_s2c_ping(msg_vec, owner.0.clone().unwrap().uuid);
                        match bctx.clone().unwrap().send(data) {
                            Ok(_) => (),
                            Err(_) => warn!("[WebSocket{}] Failed to send Ping! Maybe there's no one to send", owner.name()),
                        };
                        continue;
                    },
                    C2SMessage::Sub(uuid) => { // FIXME: Исключить возможность использования SUB без авторизации
                        debug!("[WebSocket{}] C2S : Sub", owner.name());
                        // Отбрасываю Sub на самого себя
                        if uuid == owner.0.clone().unwrap().uuid {
                            continue;
                        };
        
                        let broadcast = state.broadcasts.lock().await;
                        let rx =  match broadcast.get(&uuid) {
                            Some(rx) => rx.to_owned().subscribe(),
                            None => {
                                warn!("[WebSocket{}] Attention! The required UUID for subscription was not found!", owner.name());
                                let (tx, rx) = broadcast::channel(64);
                                broadcast.insert(uuid, tx);
                                rx
                            },
                        };
                        // .to_owned().subscribe();
                        let shutdown = Arc::new(Notify::new());
                        tokio::spawn(subscribe(mtx.clone(), rx, shutdown.clone()));
                        cutoff.insert(uuid, shutdown);
                        continue;
                    },
                    C2SMessage::Unsub(uuid) => {
                        debug!("[WebSocket{}] C2S : Unsub", owner.name());
                        // Отбрасываю Unsub на самого себя
                        if uuid == owner.0.clone().unwrap().uuid {
                            continue;
                        };
                        let shutdown = cutoff.remove(&uuid).unwrap().1;
                        shutdown.notify_one();
                        continue;
                    },
                }
        
                // Sending message
                debug!("[WebSocket{}] Answering: {msg:?}", owner.name());
                if socket.send(msg).await.is_err() {
                    warn!("[WebSocket{}] Send error! Connection terminated!", owner.name());
                    if let Some(u) = owner.0 {
                        remove_broadcast(state.broadcasts.clone(), u.uuid).await;
                    }
                    return;
                }
            }
            msg = mrx.recv() => {
                match socket.send(Message::Binary(msg.clone().unwrap())).await {
                    Ok(_) => {
                        debug!("[WebSocketSubscribe{}] Answering: {}", owner.name(), hex::encode(msg.unwrap()));
                    }
                    Err(_) => {
                        warn!("[WebSocketSubscriber{}] Send error! Connection terminated!", owner.name());
                        if let Some(u) = owner.0 {
                            remove_broadcast(state.broadcasts.clone(), u.uuid).await;
                        }
                        return;
                    }
                }
            }
        }
    }
}

async fn subscribe(socket: mpsc::Sender<Vec<u8>>, mut rx: Receiver<Vec<u8>>, shutdown: Arc<Notify>) {
    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                debug!("Shutdown SUB!");
                return;
            }
            msg = rx.recv() => {
                if socket.send(msg.unwrap()).await.is_err() {
                    error!("Forced shutdown SUB due error!");
                    return;
                };
            }
        }
    }
}

fn into_s2c_ping(buf: Vec<u8>, uuid: Uuid) -> Vec<u8> {
    use std::iter::once;
    once(1).chain(uuid.into_bytes().iter().copied()).chain(buf.as_slice()[1..].iter().copied()).collect()
}

async fn remove_broadcast(broadcasts: Arc<tokio::sync::Mutex<DashMap<Uuid, broadcast::Sender<Vec<u8>>>>>, uuid: Uuid) {
    let map = broadcasts.lock().await;
    map.remove(&uuid);
}