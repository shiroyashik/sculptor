use axum::extract::ws::{Message, WebSocket};

use super::{C2SMessage, RADError};

pub trait RecvAndDecode {
    async fn recv_and_decode(&mut self) -> Result<C2SMessage, RADError>;
}

impl RecvAndDecode for WebSocket {
    async fn recv_and_decode(&mut self) -> Result<C2SMessage, RADError> {
        if let Some(msg) = self.recv().await {
            match msg {
                Ok(msg) => {
                    match msg {
                        Message::Close(frame) => Err(RADError::Close(frame.map(|f| format!("code: {}, reason: {}", f.code, f.reason)))),
                        _ => {
                            match C2SMessage::try_from(msg.clone().into_data().as_slice()) {
                                Ok(decoded) => Ok(decoded),
                                Err(e) => {
                                    Err(RADError::DecodeError(e, hex::encode(msg.into_data())))
                                },
                            }
                        }
                    }
                },
                Err(e) => Err(RADError::WebSocketError(e)),
            }
        } else {
            Err(RADError::StreamClosed)
        }
    }
}