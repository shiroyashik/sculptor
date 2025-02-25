mod c2s;
mod s2c;
mod errors;
mod session;

use std::time::Instant;

pub use session::*;
pub use errors::*;
pub use c2s::*;
pub use s2c::*;

use axum::extract::ws::{Message, WebSocket};

use crate::{PINGS, PINGS_ERROR};

pub trait RecvAndDecode {
    async fn recv_and_decode(&mut self) -> Result<C2SMessage, RADError>;
}

impl RecvAndDecode for WebSocket {
    async fn recv_and_decode(&mut self) -> Result<C2SMessage, RADError> {
        let msg = self.recv().await.ok_or(RADError::StreamClosed)??;
        
        if let Message::Close(frame) = msg {
            return Err(RADError::Close(frame.map(|f| format!("code: {}, reason: {}", f.code, f.reason))));
        }

        let start = Instant::now();
        
        let data = msg.into_data();
        let msg = C2SMessage::try_from(data.as_ref())
            .map_err(|e| { PINGS_ERROR.inc(); RADError::DecodeError(e, faster_hex::hex_string(&data)) });
        
        let latency = start.elapsed().as_secs_f64();
        PINGS
            .with_label_values(&[msg.as_ref().map(|m| m.name()).unwrap_or("error")])
            .observe(latency);
        msg
    }
}