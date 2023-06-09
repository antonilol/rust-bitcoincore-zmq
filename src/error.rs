use crate::message::{DATA_MAX_LEN, SEQUENCE_LEN, TOPIC_MAX_LEN};
use bitcoin::consensus;
use core::fmt;

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
}

impl From<zmq::Error> for Error {
    #[inline]
    fn from(value: zmq::Error) -> Self {
        Self::Zmq(value)
    }
}

impl From<consensus::encode::Error> for Error {
    #[inline]
    fn from(value: consensus::encode::Error) -> Self {
        Self::BitcoinDeserialization(value)
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
                    String::from_utf8_lossy(&topic[0..*len]),
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
        }
    }
}

impl std::error::Error for Error {
    #[inline]
    fn cause(&self) -> Option<&dyn std::error::Error> {
        Some(match self {
            Self::BitcoinDeserialization(e) => e,
            Self::Zmq(e) => e,
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
