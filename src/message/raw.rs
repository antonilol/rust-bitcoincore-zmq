use super::{Message, Topic};
use crate::error::{Error, Result};

/// A raw message. Raw messages can be parsed to [`Message`]s and serialized to bytes.
///
/// This type can hold bytes in any type that implements [`AsRef<[u8]>`][AsRef]. It defaults to
/// using [`Vec<u8>`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawMessage<Bytes = Vec<u8>> {
    topic: Topic,
    data: Bytes,
    sequence: u32,
}

impl<Bytes: AsRef<[u8]>> RawMessage<Bytes> {
    pub fn from_parts(topic: Topic, data: Bytes, sequence: u32) -> Self {
        Self {
            topic,
            data,
            sequence,
        }
    }

    pub fn into_parts(self) -> (Topic, Bytes, u32) {
        (self.topic, self.data, self.sequence)
    }

    pub fn as_ref(&self) -> RawMessage<&[u8]> {
        RawMessage::from_parts(self.topic, self.data_as_bytes(), self.sequence)
    }

    pub fn try_from_multipart(multipart: impl IntoIterator<Item = Bytes>) -> Result<Self> {
        let mut mp_iter = multipart.into_iter();

        let topic = mp_iter.next().ok_or(Error::InvalidMutlipartLength(0))?;
        let data = mp_iter.next().ok_or(Error::InvalidMutlipartLength(1))?;
        let sequence = mp_iter.next().ok_or(Error::InvalidMutlipartLength(2))?;

        let parts = mp_iter.count() + 3;
        if parts != 3 {
            return Err(Error::InvalidMutlipartLength(parts));
        }

        Self::try_from_multipart_parts(topic, data, sequence)
    }

    pub fn try_from_multipart_parts(
        topic: impl AsRef<[u8]>,
        data: Bytes,
        sequence: impl AsRef<[u8]>,
    ) -> Result<Self> {
        let topic = Topic::try_from_bytes(topic.as_ref())?;

        let sequence = sequence.as_ref();
        let sequence = u32::from_le_bytes(
            sequence
                .try_into()
                .map_err(|_| Error::InvalidSequenceLength(sequence.len()))?,
        );

        Ok(Self::from_parts(topic, data, sequence))
    }

    pub fn to_vecs(&self) -> [Vec<u8>; 3] {
        [
            self.topic_as_bytes().to_vec(),
            self.data_as_bytes().to_vec(),
            self.sequence_as_bytes().to_vec(),
        ]
    }

    pub fn topic(&self) -> Topic {
        self.topic
    }

    pub fn topic_as_bytes(&self) -> &'static [u8] {
        self.topic.as_str().as_bytes()
    }

    pub fn data(&self) -> &Bytes {
        &self.data
    }

    pub fn data_as_bytes(&self) -> &[u8] {
        self.data.as_ref()
    }

    pub fn sequence(&self) -> u32 {
        self.sequence
    }

    pub fn sequence_as_bytes(&self) -> [u8; 4] {
        self.sequence.to_le_bytes()
    }
}

impl From<&Message> for RawMessage<Vec<u8>> {
    fn from(value: &Message) -> Self {
        value.serialize_to_raw_message()
    }
}

impl From<Message> for RawMessage<Vec<u8>> {
    fn from(value: Message) -> Self {
        value.serialize_to_raw_message()
    }
}
