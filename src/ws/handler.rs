use axum::{extract::{ws::{Message, WebSocket}, WebSocketUpgrade}, response::Response};
use log::{error, info, warn};

use crate::ws::C2SMessage;

pub async fn handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        info!("{msg:?}");
        let mut msg = if let Ok(msg) = msg {
            msg
        } else {
            // if reached here - client disconnected
            warn!("ws disconnected!");
            return;
        };
        // Work with code here
        let msg_array = msg.clone().into_data();
        let msg_array = msg_array.as_slice();
        
        let newmsg = match C2SMessage::try_from(msg_array) {
            Ok(data) => data,
            Err(e) => {
                error!("MessageLoadError: {e:?}");
                return;
            },
        };

        match newmsg {
            C2SMessage::Token(token) => {
                // TODO: Authenticated check
                msg = Message::Binary(vec![0])
            },
            // C2SMessage::Ping(_, _, _) => todo!(),
            // C2SMessage::Sub(_) => todo!(),
            // C2SMessage::Unsub(_) => todo!(),
            _ => ()
        }

        info!("{newmsg:?}");

        if socket.send(msg).await.is_err() {
            // if reached here - client disconnected
            warn!("ws disconnected!");
            return;
        }
    }
}