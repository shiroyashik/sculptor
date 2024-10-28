use anyhow::bail;
use axum::extract::{ws::{Message, WebSocket}, State};
use dashmap::DashMap;
use tokio::sync::{broadcast, mpsc};

use crate::{auth::Userinfo, AppState};

use super::{processor::*, AuthModeError, S2CMessage, C2SMessage, WSSession, SessionMessage, RADError};

pub async fn initial(
    ws: axum::extract::WebSocketUpgrade,
    State(state): State<AppState>
) -> axum::response::Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut ws: WebSocket, state: AppState) {
    // Trying authenticate & get user data or dropping connection
    match authenticate(&mut ws, &state).await {
        Ok(user) => {

            // Creating session & creating/getting channels
            let mut session = {
                let sub_workers_aborthandles = DashMap::new();
                
                // Channel for receiving messages from internal functions.
                let (own_tx, own_rx) = mpsc::channel(32);
                state.session.insert(user.uuid, own_tx.clone());

                // Channel for sending messages to subscribers
                let subs_tx = match state.subscribes.get(&user.uuid) {
                    Some(tx) => tx.clone(),
                    None => {
                        tracing::debug!("[Subscribes] Can't find own subs channel for {}, creating new...", user.uuid);
                        let (subs_tx, _) = broadcast::channel(32);
                        state.subscribes.insert(user.uuid, subs_tx.clone());
                        subs_tx
                    },
                };

                WSSession { user: user.clone(), own_tx, own_rx, subs_tx, sub_workers_aborthandles }
            };

            // Starting main worker
            match main_worker(&mut session, &mut ws, &state).await {
                Ok(_) => (),
                Err(kind) => tracing::error!("[WebSocket] Main worker halted due to {}.",  kind),
            }

            for (_, handle) in session.sub_workers_aborthandles {
                handle.abort();
            }
        
            // Removing session data
            state.session.remove(&user.uuid);
            state.user_manager.remove(&user.uuid);
        },
        Err(kind) => {
            tracing::info!("[WebSocket] Can't authenticate: {}", kind);
        }
    }

    // Closing connection
    if let Err(kind) = ws.close().await { tracing::trace!("[WebSocket] Closing fault: {}", kind) }
}

async fn main_worker(session: &mut WSSession, ws: &mut WebSocket, state: &AppState) -> anyhow::Result<()> {
    tracing::debug!("WebSocket control for {} is transferred to the main worker", session.user.username);
    loop {
        tokio::select! {
            external_msg = ws.recv_and_decode() => {

                // Getting a value or halt the worker without an error
                let external_msg = match external_msg {
                    Ok(m) => m,
                    Err(kind) => {
                        match kind {
                            RADError::Close(_) => return Ok(()),
                            RADError::StreamClosed => return Ok(()),
                            _ => return Err(kind.into())
                        }
                    },
                };

                // Processing message
                match external_msg {
                    C2SMessage::Token(_) => bail!("authentication passed, but the client sent the Token again"),
                    C2SMessage::Ping(func_id, echo, data) => {
                        let s2c_ping: Vec<u8> = S2CMessage::Ping(session.user.uuid, func_id, echo, data).into();
                        
                        // Echo check
                        if echo {
                            ws.send(Message::Binary(s2c_ping.clone())).await?
                        }
                        // Sending to others
                        let _ = session.subs_tx.send(s2c_ping);
                    },
                    C2SMessage::Sub(uuid) => {
                        tracing::debug!("[WebSocket] {} subscribes to {}", session.user.username, uuid);
                        
                        // Doesn't allow to subscribe to yourself
                        if session.user.uuid != uuid {
                            // Creates a channel to send pings to a subscriber if it can't find an existing one
                            let rx = match state.subscribes.get(&uuid) {
                                Some(tx) => tx.subscribe(),
                                None => {
                                    let (tx, rx) = broadcast::channel(32);
                                    state.subscribes.insert(uuid, tx);
                                    rx
                                },
                            };
                            let handle = tokio::spawn(sub_worker(session.own_tx.clone(), rx)).abort_handle();
                            session.sub_workers_aborthandles.insert(uuid, handle);
                        }
                    },
                    C2SMessage::Unsub(uuid) => {
                        tracing::debug!("[WebSocket] {} unsubscribes from {}", session.user.username, uuid);

                        match session.sub_workers_aborthandles.get(&uuid) {
                            Some(handle) => handle.abort(),
                            None => tracing::warn!("[WebSocket] {} was not subscribed.", session.user.username),
                        };
                    },
                }
            },
            internal_msg = session.own_rx.recv() => {
                let internal_msg = internal_msg.ok_or(anyhow::anyhow!("Unexpected error! Session channel broken!"))?;
                match internal_msg {
                    SessionMessage::Ping(msg) => {
                        ws.send(Message::Binary(msg)).await?
                    },
                    SessionMessage::Banned => {
                        let _ = ban_action(ws).await
                            .inspect_err(
                                |kind| tracing::warn!("[WebSocket] Didn't get the ban message due to {}", kind)
                            );
                        bail!("{} banned!", session.user.username)
                    },
                }
            }
        }
    }
}

async fn sub_worker(tx_main: mpsc::Sender<SessionMessage>, mut rx: broadcast::Receiver<Vec<u8>>) {
    loop {
        let msg = match rx.recv().await {
            Ok(m) => m,
            Err(kind) => {
                tracing::error!("[Subscribes_Worker] Broadcast error! {}", kind);
                return;
            },
        };
        match tx_main.send(SessionMessage::Ping(msg)).await {
            Ok(_) => (),
            Err(kind) => {
                tracing::error!("[Subscribes_Worker] Session error! {}", kind);
                return;
            },
        }
    }
}

async fn authenticate(socket: &mut WebSocket, state: &AppState) -> Result<Userinfo, AuthModeError> {
    match socket.recv_and_decode().await {
        Ok(msg) => {
            match msg {
                C2SMessage::Token(token) => {
                    let token = String::from_utf8(token.to_vec()).map_err(|_| AuthModeError::ConvertError)?;
                    match state.user_manager.get(&token) {
                        Some(user) => {
                            if socket.send(Message::Binary(S2CMessage::Auth.into())).await.is_err() {
                                Err(AuthModeError::SendError)
                            } else if !user.banned {
                                Ok(user.clone())
                            } else {
                                let _ = ban_action(socket).await
                                    .inspect_err(
                                        |kind| tracing::warn!("[WebSocket] Didn't get the ban message due to {}", kind)
                                    );
                                Err(AuthModeError::Banned(user.username.clone()))
                            }
                        },
                        None => {
                            if socket.send(
                                Message::Close(Some(axum::extract::ws::CloseFrame { code: 4000, reason: "Re-auth".into() }))
                            ).await.is_err() {
                                Err(AuthModeError::SendError)
                            } else {
                                Err(AuthModeError::AuthenticationFailure)
                            }
                        },
                    }
                },
                _ => {
                    Err(AuthModeError::UnauthorizedAction)
                }
            }
        },
        Err(err) => {
            Err(AuthModeError::RecvError(err))
        },
    }
}

async fn ban_action(ws: &mut WebSocket) -> anyhow::Result<()> {
    ws.send(Message::Binary(S2CMessage::Toast(2, "You're banned!".to_string(), None).into())).await?;
    tokio::time::sleep(std::time::Duration::from_secs(6)).await;
    ws.send(Message::Close(Some(axum::extract::ws::CloseFrame { code: 4001, reason: "You're banned!".into() }))).await?;

    Ok(())
}