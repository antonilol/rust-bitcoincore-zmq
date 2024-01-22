use crate::{
    message::{DATA_MAX_LEN, SEQUENCE_LEN, TOPIC_MAX_LEN},
    monitor::MonitorMessageError,
};
use bitcoin::consensus;
use core::{cmp::min, fmt};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidMutlipartLength(usize),
    InvalidTopic(usize, [u8; TOPIC_MAX_LEN]),
    InvalidDataLength(usize),
    InvalidSequenceLength(usize),
    InvalidSequenceMessageLength(usize),
    InvalidSequenceMessageLabel(u8),
    Invalid256BitHashLength(usize),
    BitcoinDeserialization(consensus::encode::Error),
    Zmq(zmq::Error),
    MonitorMessage(MonitorMessageError),
}

impl Error {
    /// Returns the (invalid) topic as a byte slice (as this might not always be valid UTF-8). If
    /// this error is not an [`Error::InvalidTopic`], [`None`] is returned. The real length is also
    /// returned, if this is higher that the length of the slice, the data was truncated to fit
    /// directly in the object, instead of with a heap allocation.
    pub fn invalid_topic_data(&self) -> Option<(&[u8], usize)> {
        if let Self::InvalidTopic(len, buf) = self {
            Some((&buf[..min(*len, buf.len())], *len))
        } else {
            None
        }
    }
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

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMutlipartLength(len) => {
                write!(f, "invalid multipart message length: {len} (expected 3)")
            }
            Self::InvalidTopic(len, topic) => {
                write!(
                    f,
                    "invalid message topic '{}'{}",
                    String::from_utf8_lossy(&topic[..min(*len, topic.len())]),
                    if *len > TOPIC_MAX_LEN {
                        " (truncated)"
                    } else {
                        ""
                    }
                )
            }
            Self::InvalidDataLength(len) => {
                write!(f, "data too long ({len} > {DATA_MAX_LEN})")
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
            Self::BitcoinDeserialization(e) => e,
            Self::Zmq(e) => e,
            Self::MonitorMessage(e) => e,
            Self::InvalidMutlipartLength(_)
            | Self::InvalidTopic(_, _)
            | Self::InvalidDataLength(_)
            | Self::InvalidSequenceLength(_)
            | Self::InvalidSequenceMessageLength(_)
            | Self::InvalidSequenceMessageLabel(_)
            | Self::Invalid256BitHashLength(_) => return None,
        })
    }
}
