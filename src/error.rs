use bitcoin::{consensus, hashes};
use core::fmt;
use std::str;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidMutlipartLengthError,
    InvalidSequenceLengthError,
    InvalidSequenceMessageLengthError(usize),
    InvalidSequenceMessageLabelError(u8),
    InvalidHashLengthError(hashes::Error),
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

impl From<hashes::Error> for Error {
    fn from(value: hashes::Error) -> Self {
        Self::InvalidHashLengthError(value)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidMutlipartLengthError => write!(f, "invalid multipart message length"),
            Error::InvalidSequenceLengthError => write!(f, "invalid sequence length"),
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
            Error::InvalidHashLengthError(e) => write!(f, "invalid hash length: {e}"),
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
