use crate::{
    message::{UnknownTopicError, SEQUENCE_LEN},
    monitor::MonitorMessageError,
};
use bitcoin::consensus;
use core::fmt;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidMutlipartLength(usize),
    UnknownTopic(UnknownTopicError),
    InvalidSequenceLength(usize),
    InvalidSequenceMessageLength(usize),
    InvalidSequenceMessageLabel(u8),
    Invalid256BitHashLength(usize),
    BitcoinDeserialization(consensus::encode::Error),
    Zmq(zmq::Error),
    MonitorMessage(MonitorMessageError),
}

impl From<zmq::Error> for Error {
    #[inline]
    fn from(value: zmq::Error) -> Self {
        Self::Zmq(value)
    }
}

#[cfg(feature = "async")]
impl From<async_zmq::SocketError> for Error {
    #[inline]
    fn from(value: async_zmq::SocketError) -> Self {
        Self::Zmq(value.into())
    }
}

#[cfg(feature = "async")]
impl From<async_zmq::SubscribeError> for Error {
    #[inline]
    fn from(value: async_zmq::SubscribeError) -> Self {
        Self::Zmq(value.into())
    }
}

#[cfg(feature = "async")]
impl From<async_zmq::RecvError> for Error {
    #[inline]
    fn from(value: async_zmq::RecvError) -> Self {
        Self::Zmq(value.into())
    }
}

impl From<consensus::encode::Error> for Error {
    #[inline]
    fn from(value: consensus::encode::Error) -> Self {
        Self::BitcoinDeserialization(value)
    }
}

impl From<MonitorMessageError> for Error {
    #[inline]
    fn from(value: MonitorMessageError) -> Self {
        Self::MonitorMessage(value)
    }
}

impl From<UnknownTopicError> for Error {
    #[inline]
    fn from(value: UnknownTopicError) -> Self {
        Self::UnknownTopic(value)
    }
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMutlipartLength(len) => {
                write!(f, "invalid multipart message length: {len} (expected 3)")
            }
            Self::UnknownTopic(e) => {
                write!(f, "{e}")
            }
            Self::InvalidSequenceLength(len) => {
                write!(
                    f,
                    "invalid sequence length: {len} (expected {SEQUENCE_LEN})"
                )
            }
            Self::InvalidSequenceMessageLength(len) => {
                write!(f, "invalid message length {len} of message type 'sequence'")
            }
            Self::InvalidSequenceMessageLabel(label) => {
                write!(
                    f,
                    "invalid label '{}' (0x{:02x}) of message type 'sequence'",
                    *label as char, label
                )
            }
            Self::Invalid256BitHashLength(len) => {
                write!(f, "invalid hash length: {len} (expected 32)")
            }

            Self::BitcoinDeserialization(e) => {
                write!(f, "bitcoin consensus deserialization error: {e}")
            }
            Self::Zmq(e) => write!(f, "ZMQ Error: {e}"),
            Self::MonitorMessage(err) => write!(f, "unable to parse monitor message: {err}"),
        }
    }
}

impl std::error::Error for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::UnknownTopic(e) => e,
            Self::BitcoinDeserialization(e) => e,
            Self::Zmq(e) => e,
            Self::MonitorMessage(e) => e,
            Self::InvalidMutlipartLength(_)
            | Self::InvalidSequenceLength(_)
            | Self::InvalidSequenceMessageLength(_)
            | Self::InvalidSequenceMessageLabel(_)
            | Self::Invalid256BitHashLength(_) => return None,
        })
    }
}
