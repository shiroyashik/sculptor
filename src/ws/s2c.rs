use super::MessageLoadError;
use std::convert::{TryFrom, TryInto};

#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum S2CMessage<'a> {
    Auth = 0,
    Ping(u128, u32, bool, &'a [u8]) = 1,
    Event(u128) = 2,
    Toast(u8, &'a str, Option<&'a str>) = 3,
    Chat(&'a str) = 4,
    Notice(u8) = 5,
}
impl<'a> TryFrom<&'a [u8]> for S2CMessage<'a> {
    type Error = MessageLoadError;
    fn try_from(buf: &'a [u8]) -> Result<Self, <Self as TryFrom<&'a [u8]>>::Error> {
        if buf.len() == 0 {
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
                            u128::from_be_bytes((&buf[1..17]).try_into().unwrap()),
                            u32::from_be_bytes((&buf[17..21]).try_into().unwrap()),
                            buf[21] != 0,
                            &buf[22..],
                        ))
                    } else {
                        Err(BadLength("S2CMessage::Ping", 22, false, buf.len()))
                    }
                }
                2 => {
                    if buf.len() == 17 {
                        Ok(Event(u128::from_be_bytes(
                            (&buf[1..17]).try_into().unwrap(),
                        )))
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
impl<'a> Into<Box<[u8]>> for S2CMessage<'a> {
    fn into(self) -> Box<[u8]> {
        use std::iter::once;
        use S2CMessage::*;
        match self {
            Auth => Box::new([0]),
            Ping(u, i, s, d) => once(1)
                .chain(u.to_be_bytes().iter().copied())
                .chain(i.to_be_bytes().iter().copied())
                .chain(once(if s { 1 } else { 0 }))
                .chain(d.into_iter().copied())
                .collect(),
            Event(u) => once(2).chain(u.to_be_bytes().iter().copied()).collect(),
            Toast(t, h, d) => once(3)
                .chain(once(t))
                .chain(h.as_bytes().into_iter().copied())
                .chain(
                    d.into_iter()
                        .map(|s| once(0).chain(s.as_bytes().into_iter().copied()))
                        .flatten(),
                )
                .collect(),
            Chat(c) => once(4).chain(c.as_bytes().iter().copied()).collect(),
            Notice(t) => Box::new([5, t]),
        }
    }
}