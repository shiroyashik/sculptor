use super::MessageLoadError;
use std::convert::{TryFrom, TryInto};

use uuid::Uuid;

#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum S2CMessage {
    Auth = 0,
    Ping(Uuid, u32, bool, Vec<u8>) = 1,
    Event(Uuid) = 2, // Updates avatar for other players
    Toast(u8, String, Option<String>) = 3,
    Chat(String) = 4,
    Notice(u8) = 5,
}
impl TryFrom<&[u8]> for S2CMessage {

    type Error = MessageLoadError;

    fn try_from(buf: &[u8]) -> Result<Self, Self::Error> {
        if buf.is_empty() {
            Err(MessageLoadError::BadLength("S2CMessage", 1, false, 0))
        } else {
            use MessageLoadError::*;
            use S2CMessage::*;
            match buf[0] {
                0 => {
                    if buf.len() == 1 {
                        Ok(Auth)
                    } else {
                        Err(BadLength("S2CMessage::Auth", 1, true, buf.len()))
                    }
                }
                1 => {
                    if buf.len() >= 22 {
                        Ok(Ping(
                            Uuid::from_bytes((&buf[1..17]).try_into().unwrap()),
                            u32::from_be_bytes((&buf[17..21]).try_into().unwrap()),
                            buf[21] != 0,
                            buf[22..].to_vec(),
                        ))
                    } else {
                        Err(BadLength("S2CMessage::Ping", 22, false, buf.len()))
                    }
                }
                2 => {
                    if buf.len() == 17 {
                        Ok(Event(Uuid::from_bytes((&buf[1..17]).try_into().unwrap())))
                    } else {
                        Err(BadLength("S2CMessage::Event", 17, true, buf.len()))
                    }
                }
                3 => todo!(),
                4 => todo!(),
                5 => todo!(),
                a => Err(BadEnum("S2CMessage.type", 0..=5, a.into())),
            }
        }
    }
}

impl From<S2CMessage> for Vec<u8> {
    fn from(val: S2CMessage) -> Self {
        use std::iter::once;
        use S2CMessage::*;
        match val {
            Auth => vec![0],
            Ping(u, i, s, d) => once(1)
                .chain(u.into_bytes().iter().copied())
                .chain(i.to_be_bytes().iter().copied())
                .chain(once(if s { 1 } else { 0 }))
                .chain(d.iter().copied())
                .collect(),
            Event(u) => once(2).chain(u.into_bytes().iter().copied()).collect(),
            Toast(t, h, d) => once(3)
                .chain(once(t))
                .chain(h.as_bytes().iter().copied())
                .chain(
                    d.into_iter()
                        .flat_map(|s| once(0).chain(s.as_bytes().iter().copied()).collect::<Vec<_>>()), // FIXME: Try find other solution
                )
                .collect(),
            Chat(c) => once(4).chain(c.as_bytes().iter().copied()).collect(),
            Notice(t) => vec![5, t],
        }
    }
}
impl S2CMessage {
    pub fn name(&self) -> &'static str {
        match self {
            S2CMessage::Auth => "s2c>auth",
            S2CMessage::Ping(_, _, _, _) => "s2c>ping",
            S2CMessage::Event(_) => "s2c>event",
            S2CMessage::Toast(_, _, _) => "s2c>toast",
            S2CMessage::Chat(_) => "s2c>chat",
            S2CMessage::Notice(_) => "s2c>notice",
        }
    }
}

// impl<'a> S2CMessage<'a> {
//     pub fn to_array(&self) -> Box<[u8]> {
//         <S2CMessage as Into<Box<[u8]>>>::into(self.clone())
//     }
//     pub fn to_vec(&self) -> Vec<u8> {
//         self.to_array().to_vec()
//     }
// }
