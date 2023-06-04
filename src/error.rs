use bitcoin::consensus;
use core::fmt;
use std::str;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidMutlipartLengthError(usize),
    InvalidSequenceLengthError(usize),
    InvalidSequenceMessageLengthError(usize),
    InvalidSequenceMessageLabelError(u8),
    Invalid256BitHashLengthError(usize),
    InvalidTopicError(Vec<u8>),
    BitcoinDeserializationError(consensus::encode::Error),
    ZmqError(zmq::Error),
}

impl From<zmq::Error> for Error {
    fn from(value: zmq::Error) -> Self {
        Self::ZmqError(value)
    }
}

impl From<consensus::encode::Error> for Error {
    fn from(value: consensus::encode::Error) -> Self {
        Self::BitcoinDeserializationError(value)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidMutlipartLengthError(len) => {
                write!(f, "invalid multipart message length: {len} (expected 3)")
            }
            Error::InvalidSequenceLengthError(len) => {
                write!(f, "invalid sequence length: {len} (expected 4)")
            }
            Error::InvalidSequenceMessageLengthError(len) => {
                write!(f, "invalid message length {len} of message type 'sequence'")
            }
            Error::InvalidSequenceMessageLabelError(label) => {
                write!(
                    f,
                    "invalid label '{}' (0x{:02x}) of message type 'sequence'",
                    *label as char, label
                )
            }
            Error::Invalid256BitHashLengthError(len) => {
                write!(f, "invalid hash length: {len} (expected 32)")
            }
            Error::InvalidTopicError(topic) => {
                write!(f, "invalid message topic ")?;
                if let Ok(topic_str) = str::from_utf8(topic) {
                    write!(f, "'{topic_str}'")
                } else {
                    write!(f, "{topic:#04x?} (not utf-8)")
                }
            }
            Error::BitcoinDeserializationError(e) => {
                write!(f, "bitcoin consensus deserialization error: {e}")
            }
            Error::ZmqError(e) => write!(f, "ZMQ Error: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        Some(match self {
            Self::BitcoinDeserializationError(e) => e,
            Self::ZmqError(e) => e,
            Self::InvalidMutlipartLengthError(_)
            | Self::InvalidSequenceLengthError(_)
            | Self::InvalidSequenceMessageLengthError(_)
            | Self::InvalidSequenceMessageLabelError(_)
            | Self::Invalid256BitHashLengthError(_)
            | Self::InvalidTopicError(_) => return None,
        })
    }
}
