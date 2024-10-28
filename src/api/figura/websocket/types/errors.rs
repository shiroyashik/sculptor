use std::fmt::*;
use std::ops::RangeInclusive;

use thiserror::Error;

#[derive(Debug)]
pub enum MessageLoadError {
    BadEnum(&'static str, RangeInclusive<usize>, usize),
    BadLength(&'static str, usize, bool, usize),
}
impl Display for MessageLoadError {
    fn fmt(&self, fmt: &mut Formatter) -> Result {
        match self {
            Self::BadEnum(f, r, c) => write!(
                fmt,
                "invalid value of {f}: must be {} to {} inclusive, got {c}",
                r.start(),
                r.end()
            ),
            Self::BadLength(f, n, e, c) => write!(
                fmt,
                "buffer wrong size for {f}: must be {} {n} bytes, got {c}",
                if *e { "exactly" } else { "at least" }
            ),
        }
    }
}

#[derive(Error, Debug)]
pub enum RADError {
    #[error("message decode error due: {0}, invalid data: {1}")]
    DecodeError(MessageLoadError, String),
    #[error("close, frame: {0:?}")]
    Close(Option<String>),
    #[error(transparent)]
    WebSocketError(#[from] axum::Error),
    #[error("stream closed")]
    StreamClosed,
}

#[derive(Error, Debug)]
pub enum AuthModeError {
    #[error("token recieve error due {0}")]
    RecvError(RADError),
    #[error("action attempt without authentication")]
    UnauthorizedAction,
    #[error("convert error, bytes into string")]
    ConvertError,
    #[error("can't send, websocket broken")]
    SendError,
    #[error("authentication failure, sending re-auth...")]
    AuthenticationFailure,
    #[error("{0} banned")]
    Banned(String),
}

#[cfg(test)]
#[test]
fn message_load_error_display() {
    use MessageLoadError::*;
    assert_eq!(
        BadEnum("foo", 3..=5, 7).to_string(),
        "invalid value of foo: must be 3 to 5 inclusive, got 7"
    );
    assert_eq!(
        BadLength("bar", 17, false, 12).to_string(),
        "buffer wrong size for bar: must be at least 17 bytes, got 12"
    );
    assert_eq!(
        BadLength("bar", 17, true, 19).to_string(),
        "buffer wrong size for bar: must be exactly 17 bytes, got 19"
    );
}
