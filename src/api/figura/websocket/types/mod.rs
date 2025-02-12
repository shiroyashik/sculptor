mod c2s;
mod s2c;
mod errors;
mod session;

pub use session::*;
pub use errors::*;
pub use c2s::*;
pub use s2c::*;

use axum::extract::ws::{Message, WebSocket};

pub trait RecvAndDecode {
    async fn recv_and_decode(&mut self) -> Result<C2SMessage, RADError>;
}

impl RecvAndDecode for WebSocket {
    async fn recv_and_decode(&mut self) -> Result<C2SMessage, RADError> {
        let msg = self.recv().await.ok_or(RADError::StreamClosed)??;
        
        if let Message::Close(frame) = msg {
            return Err(RADError::Close(frame.map(|f| format!("code: {}, reason: {}", f.code, f.reason))));
        }
        
        let data = msg.into_data();
        C2SMessage::try_from(data.as_ref())
            .map_err(|e| RADError::DecodeError(e, faster_hex::hex_string(&data)))
    }
}