pub mod event;

use self::event::SocketEvent;

use core::fmt;

/// A [`SocketEvent`] combined with its source (the url used when connecting).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorMessage {
    pub event: SocketEvent,
    pub source_url: String,
}

impl MonitorMessage {
    pub fn parse_from(msg: &[zmq::Message]) -> Result<Self, MonitorMessageError> {
        let [event_message, url_message] = msg else {
            return Err(MonitorMessageError::InvalidMutlipartLength(msg.len()));
        };

        Ok(Self {
            event: SocketEvent::parse_from(event_message)?,
            source_url: String::from_utf8_lossy(url_message).into(),
        })
    }
}

#[derive(Debug)]
// currently all variants have the same prefix: `Invalid`, which is correct and intended
#[allow(clippy::enum_variant_names)]
pub enum MonitorMessageError {
    InvalidMutlipartLength(usize),
    InvalidEventFrameLength(usize),
    InvalidEventData(u16, u32),
}

impl fmt::Display for MonitorMessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMutlipartLength(len) => {
                write!(f, "invalid multipart message length: {len} (expected 2)")
            }
            Self::InvalidEventFrameLength(len) => {
                write!(f, "invalid event frame length: {len} (expected 6)")
            }
            Self::InvalidEventData(event_type, event_data) => {
                write!(f, "invalid event data {event_data} for event {event_type}")
            }
        }
    }
}

impl std::error::Error for MonitorMessageError {}
