mod raw;
mod sequence;
mod topic;

pub use raw::RawMessage;
pub use sequence::SequenceMessage;
pub use topic::{Topic, UnknownTopicError};

use crate::error::{Error, Result};

use core::fmt;

use bitcoin::consensus::{deserialize, serialize};
use bitcoin::hashes::Hash;
use bitcoin::{Block, BlockHash, Transaction, Txid};

/// Length of the sequence field in a message.
pub const SEQUENCE_LEN: usize = size_of::<u32>();

/// Content and topic of a message.
///
/// Parts of the documentation on the variants was taken from
/// <https://github.com/bitcoin/bitcoin/blob/master/doc/zmq.md#usage>.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MessageContent {
    /// Topic: [`HashBlock`][Topic::HashBlock].
    ///
    /// Notifies when the chain tip is updated.
    ///
    /// Like [`Block`][MessageContent::Block], but only contains the hash of the block.
    BlockHash(BlockHash),
    /// Topic: [`HashTx`][Topic::HashTx].
    ///
    /// Notifies about all transactions, both when they are added to mempool or when a new block
    /// arrives. This means a transaction could be published multiple times. First, when it enters
    /// the mempool and then again in each block that includes it.
    ///
    /// Like [`Tx`][MessageContent::Tx], but only contains the hash of the transaction.
    Txid(Txid),
    /// Topic: [`RawBlock`][Topic::RawBlock].
    ///
    /// Notifies when the chain tip is updated.
    ///
    /// Like [`BlockHash`][MessageContent::BlockHash], but contains the complete block.
    Block(Block),
    /// Topic: [`RawTx`][Topic::RawTx].
    ///
    /// Notifies about all transactions, both when they are added to mempool or when a new block
    /// arrives. This means a transaction could be published multiple times. First, when it enters
    /// the mempool and then again in each block that includes it.
    ///
    /// Like [`Txid`][MessageContent::Txid], but contains the complete transaction.
    Tx(Transaction),
    /// Topic: [`Sequence`][Topic::Sequence].
    Sequence(SequenceMessage),
}

impl MessageContent {
    /// Returns the topic of this [`Message`]. On the returned enum variant of [`Topic`]
    /// [`.as_str()`][Topic::as_str] and [`.as_bytes()`][Topic::as_bytes] can be used to get the
    /// topic as a string slice or byte slice, respectively, it can be used with the `match`
    /// keyword, or something else.
    pub fn topic(&self) -> Topic {
        match self {
            Self::BlockHash(..) => Topic::HashBlock,
            Self::Txid(..) => Topic::HashTx,
            Self::Block(..) => Topic::RawBlock,
            Self::Tx(..) => Topic::RawTx,
            Self::Sequence(..) => Topic::Sequence,
        }
    }

    /// Serializes the middle part of this [`Message`] (no topic and sequence).
    #[inline]
    pub fn serialize_data_to_vec(&self) -> Vec<u8> {
        match self {
            Self::BlockHash(blockhash) => {
                let mut arr = blockhash.to_byte_array();
                arr.reverse();
                arr.to_vec()
            }
            Self::Txid(txid) => {
                let mut arr = txid.to_byte_array();
                arr.reverse();
                arr.to_vec()
            }
            Self::Block(block) => serialize(&block),
            Self::Tx(tx) => serialize(&tx),
            Self::Sequence(sm) => sm.serialize_to_vec(),
        }
    }

    #[inline]
    pub fn try_from_raw_message<Bytes: AsRef<[u8]>>(message: RawMessage<Bytes>) -> Result<Self> {
        let topic = message.topic();
        let data = message.data_as_bytes();

        Ok(match topic {
            Topic::HashBlock | Topic::HashTx => {
                let mut data: [u8; 32] = data
                    .try_into()
                    .map_err(|_| Error::Invalid256BitHashLength(data.len()))?;
                data.reverse();

                match topic {
                    Topic::HashBlock => Self::BlockHash(BlockHash::from_byte_array(data)),
                    _ /* Topic::HashTx */ => Self::Txid(Txid::from_byte_array(data)),
                }
            }
            Topic::RawBlock => Self::Block(deserialize(data)?),
            Topic::RawTx => Self::Tx(deserialize(data)?),
            Topic::Sequence => Self::Sequence(SequenceMessage::from_byte_slice(data)?),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub content: MessageContent,
    pub sequence: u32,
}

impl Message {
    /// See [`MessageContent::topic`].
    pub fn topic(&self) -> Topic {
        self.content.topic()
    }

    /// See [`MessageContent::serialize_data_to_vec`].
    pub fn serialize_data_to_vec(&self) -> Vec<u8> {
        self.content.serialize_data_to_vec()
    }

    pub fn serialize_to_raw_message(&self) -> RawMessage<Vec<u8>> {
        RawMessage::from_parts(self.topic(), self.serialize_data_to_vec(), self.sequence)
    }

    /// Serializes this [`Message`] to 3 [`Vec<u8>`]s.
    pub fn serialize_to_vecs(&self) -> [Vec<u8>; 3] {
        self.serialize_to_raw_message().to_vecs()
    }

    /// Attempts to deserialize a multipart (multiple byte slices) to a [`Message`].
    #[inline]
    pub fn from_multipart<T: AsRef<[u8]>>(multipart: &[T]) -> Result<Self> {
        Self::try_from_raw_message(RawMessage::try_from_multipart(multipart)?)
    }

    #[inline]
    pub fn from_fixed_size_multipart<T: AsRef<[u8]>>(mp: &[T; 3]) -> Result<Self> {
        let [topic, data, sequence] = mp;

        Self::try_from_raw_message(RawMessage::try_from_multipart_parts(topic, data, sequence)?)
    }

    pub fn try_from_raw_message<Bytes: AsRef<[u8]>>(message: RawMessage<Bytes>) -> Result<Self> {
        let sequence = message.sequence();
        MessageContent::try_from_raw_message(message).map(|content| Self { content, sequence })
    }
}

impl<T: AsRef<[u8]>> TryFrom<&[T]> for Message {
    type Error = Error;

    #[inline]
    fn try_from(value: &[T]) -> Result<Self> {
        Self::from_multipart(value)
    }
}

impl<T: AsRef<[u8]>> TryFrom<[T; 3]> for Message {
    type Error = Error;

    #[inline]
    fn try_from(value: [T; 3]) -> Result<Self> {
        Self::from_fixed_size_multipart(&value)
    }
}

impl From<Message> for [Vec<u8>; 3] {
    #[inline]
    fn from(msg: Message) -> Self {
        msg.serialize_to_vecs()
    }
}

impl From<Message> for Vec<Vec<u8>> {
    #[inline]
    fn from(msg: Message) -> Self {
        msg.serialize_to_vecs().to_vec()
    }
}

impl<Bytes: AsRef<[u8]>> TryFrom<&RawMessage<Bytes>> for Message {
    type Error = Error;

    fn try_from(value: &RawMessage<Bytes>) -> Result<Self> {
        Self::try_from_raw_message(value.as_ref())
    }
}

impl<Bytes: AsRef<[u8]>> TryFrom<RawMessage<Bytes>> for Message {
    type Error = Error;

    fn try_from(value: RawMessage<Bytes>) -> Result<Self> {
        Self::try_from_raw_message(value)
    }
}

impl fmt::Display for Message {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.content {
            MessageContent::BlockHash(blockhash) => {
                write!(f, "HashBlock({blockhash}")?;
            }
            MessageContent::Txid(txid) => {
                write!(f, "HashTx({txid}")?;
            }
            MessageContent::Block(block) => {
                write!(f, "Block({}", block.block_hash())?;
            }
            MessageContent::Tx(tx) => {
                write!(f, "Tx({}", tx.compute_txid())?;
            }
            MessageContent::Sequence(sm) => {
                write!(f, "Sequence({sm}")?;
            }
        }

        write!(f, ", sequence={})", self.sequence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::UnknownTopicError;

    use bitcoin::constants::genesis_block;
    use bitcoin::Network;

    #[test]
    fn test_deserialize_rawtx() {
        let genesis_block = genesis_block(Network::Bitcoin);

        let tx = &genesis_block.txdata[0];
        let tx_bytes = serialize(tx);
        let txid = tx.compute_txid();
        let mut txid_bytes = txid.to_byte_array();
        txid_bytes.reverse();

        let to_deserialize = [
            b"rawtx" as &[u8],
            &tx_bytes,
            &[0x03, 0x00, 0x00, 0x00],
            b"garbage",
        ];

        let msg = Message::from_multipart(&to_deserialize[..3]).unwrap();

        assert_eq!(
            msg,
            Message {
                content: MessageContent::Tx(tx.clone()),
                sequence: 3
            },
        );

        assert_eq!(msg.topic().as_str(), "rawtx");
        assert_eq!(msg.serialize_data_to_vec(), tx_bytes);
        assert_eq!(msg.sequence, 3);

        assert_eq!(msg.serialize_to_vecs(), to_deserialize[0..3]);

        assert!(matches!(
            Message::from_multipart(&to_deserialize[..0]),
            Err(Error::InvalidMutlipartLength(0)),
        ));
        assert!(matches!(
            Message::from_multipart(&to_deserialize[..1]),
            Err(Error::InvalidMutlipartLength(1)),
        ));
        assert!(matches!(
            Message::from_multipart(&to_deserialize[..2]),
            Err(Error::InvalidMutlipartLength(2)),
        ));
        assert!(matches!(
            Message::from_multipart(&to_deserialize[..4]),
            Err(Error::InvalidMutlipartLength(4)),
        ));
    }

    #[test]
    fn test_deserialize_hashtx() {
        let genesis_block = genesis_block(Network::Bitcoin);

        let txid = genesis_block.txdata[0].compute_txid();
        let mut txid_bytes = txid.to_byte_array();
        txid_bytes.reverse();

        let to_deserialize = [b"hashtx" as &[u8], &txid_bytes, &[0x04, 0x00, 0x00, 0x00]];

        let msg = Message::from_multipart(&to_deserialize).unwrap();

        assert_eq!(
            msg,
            Message {
                content: MessageContent::Txid(txid),
                sequence: 4
            },
        );

        assert_eq!(msg.topic().as_str(), "hashtx");
        assert_eq!(msg.serialize_data_to_vec(), txid_bytes);
        assert_eq!(msg.sequence, 4);

        assert_eq!(msg.serialize_to_vecs(), to_deserialize);
    }

    #[test]
    fn test_deserialization_error_mp_len() {
        let to_deserialize = [
            b"sequence" as &[u8],
            &[],
            &[0x05, 0x00, 0x00, 0x00],
            b"garbage",
        ];

        assert!(matches!(
            Message::from_multipart(&to_deserialize[..0]),
            Err(Error::InvalidMutlipartLength(0)),
        ));
        assert!(matches!(
            Message::from_multipart(&to_deserialize[..1]),
            Err(Error::InvalidMutlipartLength(1)),
        ));
        assert!(matches!(
            Message::from_multipart(&to_deserialize[..2]),
            Err(Error::InvalidMutlipartLength(2)),
        ));
        assert!(matches!(
            Message::from_multipart(&to_deserialize[..4]),
            Err(Error::InvalidMutlipartLength(4)),
        ));
    }

    #[test]
    fn test_deserialization_error_topic() {
        for invalid_topic in [
            b"" as &[u8],
            b"abc",
            b"hashblock!",
            b"very loooooooooong invalid topic",
        ] {
            let err = Message::from_multipart(&[invalid_topic, &[], &0u32.to_le_bytes()])
                .expect_err("expected invalid topic");

            let Error::UnknownTopic(unknown_topic_err) = &err else {
                unreachable!();
            };

            assert_eq!(unknown_topic_err.invalid_topic_as_bytes(), invalid_topic);

            let unknown_topic_err = std::error::Error::source(&err)
                .unwrap()
                .downcast_ref::<UnknownTopicError>()
                .unwrap();

            assert_eq!(unknown_topic_err.invalid_topic_as_bytes(), invalid_topic);
        }
    }

    #[test]
    fn test_deserialization_error_element_len() {
        assert!(matches!(
            Message::from_multipart(&[b"rawtx" as &[u8], &[], b"not 4 bytes"]),
            Err(Error::InvalidSequenceLength(11)),
        ));

        assert!(matches!(
            Message::from_multipart(&[b"hashtx" as &[u8], &[], &[0x0a, 0x00, 0x00, 0x00]]),
            Err(Error::Invalid256BitHashLength(0)),
        ));

        assert!(matches!(
            Message::from_multipart(&[b"hashblock" as &[u8], &[0; 20], &[0x0b, 0x00, 0x00, 0x00]]),
            Err(Error::Invalid256BitHashLength(20)),
        ));

        assert!(matches!(
            Message::from_multipart(&[b"sequence" as &[u8], &[0; 32], &[0x0c, 0x00, 0x00, 0x00]]),
            Err(Error::InvalidSequenceMessageLength(32)),
        ));
    }
}
