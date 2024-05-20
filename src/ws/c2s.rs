use uuid::Uuid;

use super::MessageLoadError;
use std::convert::{TryFrom, TryInto};

#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum C2SMessage<'a> {
    Token(&'a [u8]) = 0,
    Ping(u32, bool, &'a [u8]) = 1,
    Sub(Uuid) = 2, // owo
    Unsub(Uuid) = 3,
}
// 6 - 6
impl<'a> TryFrom<&'a [u8]> for C2SMessage<'a> {
    type Error = MessageLoadError;
    fn try_from(buf: &'a [u8]) -> Result<Self, <Self as TryFrom<&'a [u8]>>::Error> {
        if buf.len() == 0 {
            Err(MessageLoadError::BadLength("C2SMessage", 1, false, 0))
        } else {
            match buf[0] {
                0 => Ok(C2SMessage::Token(&buf[1..])),
                1 => {
                    if buf.len() >= 6 {
                        Ok(C2SMessage::Ping(
                            u32::from_be_bytes((&buf[1..5]).try_into().unwrap()),
                            buf[5] != 0,
                            &buf[6..],
                        ))
                    } else {
                        Err(MessageLoadError::BadLength(
                            "C2SMessage::Ping",
                            6,
                            false,
                            buf.len(),
                        ))
                    }
                }
                2 => {
                    if buf.len() == 17 {
                        Ok(C2SMessage::Sub(Uuid::from_bytes(
                            (&buf[1..]).try_into().unwrap(),
                        )))
                    } else {
                        Err(MessageLoadError::BadLength(
                            "C2SMessage::Sub",
                            17,
                            true,
                            buf.len(),
                        ))
                    }
                }
                3 => {
                    if buf.len() == 17 {
                        Ok(C2SMessage::Unsub(Uuid::from_bytes(
                            (&buf[1..]).try_into().unwrap(),
                        )))
                    } else {
                        Err(MessageLoadError::BadLength(
                            "C2SMessage::Unsub",
                            17,
                            true,
                            buf.len(),
                        ))
                    }
                }
                a => Err(MessageLoadError::BadEnum(
                    "C2SMessage.type",
                    0..=3,
                    a.into(),
                )),
            }
        }
    }
}
impl<'a> Into<Box<[u8]>> for C2SMessage<'a> {
    fn into(self) -> Box<[u8]> {
        use std::iter;
        let a: Box<[u8]> = match self {
            C2SMessage::Token(t) => iter::once(0).chain(t.into_iter().copied()).collect(),
            C2SMessage::Ping(p, s, d) => iter::once(1)
                .chain(p.to_be_bytes())
                .chain(iter::once(s.into()))
                .chain(d.into_iter().copied())
                .collect(),
            C2SMessage::Sub(s) => iter::once(2).chain(s.into_bytes()).collect(),
            C2SMessage::Unsub(s) => iter::once(3).chain(s.into_bytes()).collect(),
        };
        a
    }
}