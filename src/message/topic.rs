use core::fmt;

/// Topic of a [`Message`][super::Message].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Topic {
    /// `"hashblock"`
    HashBlock,
    /// `"hashtx"`
    HashTx,
    /// `"rawblock"`
    RawBlock,
    /// `"rawtx"`
    RawTx,
    /// `"sequence"`
    Sequence,
}

impl Topic {
    /// Convert a topic string (as a byte slice) to a [`Topic`] enum variant.
    ///
    /// To convert back, use [`Topic::as_bytes`].
    ///
    /// Use [`try_from_bytes`][Self::try_from_bytes] in most cases as it will produce a readable
    /// error message. This function returns [`None`] for unknown topics to make this function
    /// usable in const-contexts.
    pub const fn try_from_bytes_const(bytes: &[u8]) -> Option<Self> {
        use Topic::*;

        Some(match bytes {
            b"hashblock" => HashBlock,
            b"hashtx" => HashTx,
            b"rawblock" => RawBlock,
            b"rawtx" => RawTx,
            b"sequence" => Sequence,
            _ => return None,
        })
    }

    /// Convert a topic string (as a byte slice) to a [`Topic`] enum variant.
    ///
    /// To convert back, use [`Topic::as_bytes`].
    ///
    /// See [`try_from_bytes_const`][Self::try_from_bytes_const] for a function that does the same
    /// but is usable in const-contexts.
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, UnknownTopicError> {
        Self::try_from_bytes_const(bytes).ok_or_else(|| UnknownTopicError::from_bytes(bytes.into()))
    }

    /// Convert a topic string to a [`Topic`] enum variant.
    ///
    /// To convert back, use [`Topic::as_str`].
    ///
    /// See [`try_from_bytes_const`][Self::try_from_bytes_const] for a function that does the same
    /// but is usable in const-contexts. Convert the string to a byte slice using
    /// [`as_bytes`][str::as_bytes].
    pub fn try_from_str(str: &str) -> Result<Self, UnknownTopicError> {
        Self::try_from_bytes(str.as_bytes())
    }

    /// Get the string representation of this topic as bytes. Convenience method for
    /// <code>topic.[as_str][Self::as_str]().[as_bytes][str::as_bytes]()</code>.
    ///
    /// To convert back, use [`Topic::try_from_bytes`].
    pub const fn as_bytes(self) -> &'static [u8] {
        self.as_str().as_bytes()
    }

    /// Get the string representation of this topic as bytes.
    ///
    /// To convert back, use [`Topic::try_from_str`].
    pub const fn as_str(self) -> &'static str {
        use Topic::*;

        match self {
            HashBlock => "hashblock",
            HashTx => "hashtx",
            RawBlock => "rawblock",
            RawTx => "rawtx",
            Sequence => "sequence",
        }
    }
}

impl TryFrom<&[u8]> for Topic {
    type Error = UnknownTopicError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_bytes(value)
    }
}

impl TryFrom<&str> for Topic {
    type Error = UnknownTopicError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from_str(value)
    }
}

impl From<Topic> for &'static [u8] {
    fn from(value: Topic) -> Self {
        value.as_bytes()
    }
}

impl From<Topic> for &'static str {
    fn from(value: Topic) -> Self {
        value.as_str()
    }
}

#[derive(Debug, Clone)]
pub struct UnknownTopicError {
    topic: Box<[u8]>,
}

impl UnknownTopicError {
    fn from_bytes(bytes: Box<[u8]>) -> Self {
        Self { topic: bytes }
    }

    pub fn invalid_topic_as_bytes(&self) -> &[u8] {
        &self.topic
    }

    pub fn into_inner(self) -> Box<[u8]> {
        self.topic
    }
}

impl fmt::Display for UnknownTopicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown topic \"{}\"",
            String::from_utf8_lossy(self.invalid_topic_as_bytes()),
        )
    }
}

impl std::error::Error for UnknownTopicError {}
