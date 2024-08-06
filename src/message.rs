use crate::{
    error::{Error, Result},
    sequence_message::SequenceMessage,
};
use bitcoin::{
    consensus::{deserialize, serialize},
    hashes::Hash,
    Block, BlockHash, Transaction, Txid, Weight,
};
use core::{cmp::min, fmt};

pub const TOPIC_MAX_LEN: usize = 9;
pub const DATA_MAX_LEN: usize = Weight::MAX_BLOCK.to_wu() as usize;
pub const SEQUENCE_LEN: usize = 4;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Message {
    HashBlock(BlockHash, u32),
    HashTx(Txid, u32),
    Block(Block, u32),
    Tx(Transaction, u32),
    Sequence(SequenceMessage, u32),
}

impl Message {
    /// Returns the topic of this [`Message`] as a byte slice.
    #[inline]
    pub fn topic(&self) -> &'static [u8] {
        self.topic_str().as_bytes()
    }

    /// Returns the topic of this [`Message`] as a string slice.
    #[inline]
    pub fn topic_str(&self) -> &'static str {
        let topic = match self {
            Self::HashBlock(..) => "hashblock",
            Self::HashTx(..) => "hashtx",
            Self::Block(..) => "rawblock",
            Self::Tx(..) => "rawtx",
            Self::Sequence(..) => "sequence",
        };

        debug_assert!(topic.len() <= TOPIC_MAX_LEN);

        topic
    }

    /// Serializes the middle part of this [`Message`] (no topic and sequence).
    #[inline]
    pub fn serialize_data_to_vec(&self) -> Vec<u8> {
        match self {
            Self::HashBlock(blockhash, _) => {
                let mut arr = blockhash.to_byte_array();
                arr.reverse();
                arr.to_vec()
            }
            Self::HashTx(txid, _) => {
                let mut arr = txid.to_byte_array();
                arr.reverse();
                arr.to_vec()
            }
            Self::Block(block, _) => serialize(&block),
            Self::Tx(tx, _) => serialize(&tx),
            Self::Sequence(sm, _) => sm.serialize_to_vec(),
        }
    }

    /// Serializes this [`Message`] to 3 [`Vec<u8>`]s.
    #[inline]
    pub fn serialize_to_vecs(&self) -> [Vec<u8>; 3] {
        [
            self.topic().to_vec(),
            self.serialize_data_to_vec(),
            self.sequence().to_le_bytes().to_vec(),
        ]
    }

    /// Returns the sequence of this [`Message`], a number that starts at 0 and goes up every time
    /// Bitcoin Core sends a ZMQ message per publisher.
    #[inline]
    pub const fn sequence(&self) -> u32 {
        match self {
            Self::HashBlock(_, sequence)
            | Self::HashTx(_, sequence)
            | Self::Block(_, sequence)
            | Self::Tx(_, sequence)
            | Self::Sequence(_, sequence) => *sequence,
        }
    }

    /// Attempts to deserialize a multipart (multiple byte slices) to a [`Message`].
    #[inline]
    pub fn from_multipart<T: AsRef<[u8]>>(mp: &[T]) -> Result<Self> {
        Self::from_fixed_size_multipart(
            mp.try_into()
                .map_err(|_| Error::InvalidMutlipartLength(mp.len()))?,
        )
    }

    #[inline]
    pub fn from_fixed_size_multipart<T: AsRef<[u8]>>(mp: &[T; 3]) -> Result<Self> {
        let [topic, data, sequence] = mp;

        let topic = topic.as_ref();
        let data = data.as_ref();
        let sequence = sequence.as_ref();

        let sequence = sequence
            .try_into()
            .map_err(|_| Error::InvalidSequenceLength(sequence.len()))?;

        Self::from_parts(topic, data, sequence)
    }

    #[inline]
    pub fn from_parts(topic: &[u8], data: &[u8], sequence: [u8; 4]) -> Result<Self> {
        let sequence = u32::from_le_bytes(sequence);

        Ok(match topic {
            b"hashblock" | b"hashtx" => {
                let mut data: [u8; 32] = data
                    .try_into()
                    .map_err(|_| Error::Invalid256BitHashLength(data.len()))?;
                data.reverse();

                match topic {
                    b"hashblock" => Self::HashBlock(BlockHash::from_byte_array(data), sequence),
                    _ /* b"hashtx" */ => Self::HashTx(Txid::from_byte_array(data), sequence),
                }
            }
            b"rawblock" => Self::Block(deserialize(data)?, sequence),
            b"rawtx" => Self::Tx(deserialize(data)?, sequence),
            b"sequence" => Self::Sequence(SequenceMessage::from_byte_slice(data)?, sequence),
            _ => {
                let mut buf = [0; TOPIC_MAX_LEN];

                buf[..min(TOPIC_MAX_LEN, topic.len())]
                    .copy_from_slice(&topic[..min(TOPIC_MAX_LEN, topic.len())]);

                return Err(Error::InvalidTopic(topic.len(), buf));
            }
        })
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

impl fmt::Display for Message {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HashBlock(blockhash, sequence) => {
                write!(f, "HashBlock({blockhash}, sequence={sequence})")
            }
            Self::HashTx(txid, sequence) => write!(f, "HashTx({txid}, sequence={sequence})"),
            Self::Block(block, sequence) => {
                write!(f, "Block({}, sequence={sequence})", block.block_hash())
            }
            Self::Tx(tx, sequence) => write!(f, "Tx({}, sequence={sequence})", tx.compute_txid()),
            Self::Sequence(sm, sequence) => write!(f, "Sequence({sm}, sequence={sequence})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Error, Message};
    use bitcoin::{consensus::serialize, constants::genesis_block, hashes::Hash, Network};

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

        assert_eq!(msg, Message::Tx(tx.clone(), 3));

        assert_eq!(msg.topic_str(), "rawtx");
        assert_eq!(msg.serialize_data_to_vec(), tx_bytes);
        assert_eq!(msg.sequence(), 3);

        assert_eq!(msg.serialize_to_vecs(), to_deserialize[0..3]);

        assert!(matches!(
            Message::from_multipart(&to_deserialize[..0]),
            Err(Error::InvalidMutlipartLength(0))
        ));
        assert!(matches!(
            Message::from_multipart(&to_deserialize[..1]),
            Err(Error::InvalidMutlipartLength(1))
        ));
        assert!(matches!(
            Message::from_multipart(&to_deserialize[..2]),
            Err(Error::InvalidMutlipartLength(2))
        ));
        assert!(matches!(
            Message::from_multipart(&to_deserialize[..4]),
            Err(Error::InvalidMutlipartLength(4))
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

        assert_eq!(msg, Message::HashTx(txid, 4));

        assert_eq!(msg.topic_str(), "hashtx");
        assert_eq!(msg.serialize_data_to_vec(), txid_bytes);
        assert_eq!(msg.sequence(), 4);

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
            Err(Error::InvalidMutlipartLength(0))
        ));
        assert!(matches!(
            Message::from_multipart(&to_deserialize[..1]),
            Err(Error::InvalidMutlipartLength(1))
        ));
        assert!(matches!(
            Message::from_multipart(&to_deserialize[..2]),
            Err(Error::InvalidMutlipartLength(2))
        ));
        assert!(matches!(
            Message::from_multipart(&to_deserialize[..4]),
            Err(Error::InvalidMutlipartLength(4))
        ));
    }

    #[test]
    fn test_deserialization_error_topic() {
        assert_eq!(
            Message::from_multipart(&[b"" as &[u8], &[], &[0x06, 0x00, 0x00, 0x00]])
                .expect_err("expected invalid topic")
                .invalid_topic_data(),
            Some((b"" as &[u8], 0))
        );

        assert_eq!(
            Message::from_multipart(&[b"abc" as &[u8], &[], &[0x07, 0x00, 0x00, 0x00]])
                .expect_err("expected invalid topic")
                .invalid_topic_data(),
            Some((b"abc" as &[u8], 3))
        );

        assert_eq!(
            Message::from_multipart(&[b"hashblock!" as &[u8], &[], &[0x08, 0x00, 0x00, 0x00]])
                .expect_err("expected invalid topic")
                .invalid_topic_data(),
            Some((b"hashblock" as &[u8], 10))
        );

        assert_eq!(
            Message::from_multipart(&[
                b"too long so gets truncated" as &[u8],
                &[],
                &[0x09, 0x00, 0x00, 0x00]
            ])
            .expect_err("expected invalid topic")
            .invalid_topic_data(),
            Some((b"too long " as &[u8], 26))
        );
    }

    #[test]
    fn test_deserialization_error_element_len() {
        assert!(matches!(
            Message::from_multipart(&[b"something" as &[u8], &[], b"not 4 bytes"]),
            Err(Error::InvalidSequenceLength(11))
        ));

        assert!(matches!(
            Message::from_multipart(&[b"hashtx" as &[u8], &[], &[0x0a, 0x00, 0x00, 0x00]]),
            Err(Error::Invalid256BitHashLength(0))
        ));

        assert!(matches!(
            Message::from_multipart(&[b"hashblock" as &[u8], &[0; 20], &[0x0b, 0x00, 0x00, 0x00]]),
            Err(Error::Invalid256BitHashLength(20))
        ));

        assert!(matches!(
            Message::from_multipart(&[b"sequence" as &[u8], &[0; 32], &[0x0c, 0x00, 0x00, 0x00]]),
            Err(Error::InvalidSequenceMessageLength(32))
        ));
    }
}
