use crate::{
    error::{Error, Result},
    sequence_message::SequenceMessage,
};
use bitcoin::{
    consensus::{deserialize, serialize},
    constants::MAX_BLOCK_WEIGHT,
    hashes::Hash,
    Block, BlockHash, Transaction, Txid,
};
use core::fmt;

pub const TOPIC_MAX_LEN: usize = 9;
pub const DATA_MAX_LEN: usize = MAX_BLOCK_WEIGHT as usize;
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
            Self::Block(..) => "block",
            Self::Tx(..) => "tx",
            Self::Sequence(..) => "sequence",
        };

        debug_assert!(topic.len() <= TOPIC_MAX_LEN);

        topic
    }

    /// Serializes the middle part of this [`Message`] (no topic and sequence).
    #[inline]
    pub fn content(&self) -> Vec<u8> {
        match self {
            Self::HashBlock(_, _) | Self::HashTx(_, _) => {
                let mut arr = match self {
                    Self::HashBlock(blockhash, _) => blockhash.to_byte_array(),
                    Self::HashTx(txid, _) => txid.to_byte_array(),
                    _ => unreachable!(),
                };
                arr.reverse();
                arr.to_vec()
            }
            Self::Block(block, _) => serialize(&block),
            Self::Tx(tx, _) => serialize(&tx),
            Self::Sequence(sm, _) => sm.as_bytes(),
        }
    }

    /// Returns the sequence of this [`Message`], a number that starts at 0 and goes up every time
    /// Bitcoin Core sends a ZMQ message per publisher
    #[inline]
    pub fn sequence(&self) -> u32 {
        match self {
            Self::HashBlock(_, seq)
            | Self::HashTx(_, seq)
            | Self::Block(_, seq)
            | Self::Tx(_, seq)
            | Self::Sequence(_, seq) => *seq,
        }
    }

    pub fn from_multipart<T: AsRef<[u8]>>(mp: &[T]) -> Result<Self> {
        Self::from_fixed_size_multipart(
            mp.try_into()
                .map_err(|_| Error::InvalidMutlipartLengthError(mp.len()))?,
        )
    }

    pub fn from_fixed_size_multipart<T: AsRef<[u8]>>(mp: &[T; 3]) -> Result<Self> {
        let [topic, data, seq] = mp;

        let topic = topic.as_ref();
        let data = data.as_ref();
        let seq = seq.as_ref();

        let seq = seq
            .try_into()
            .map_err(|_| Error::InvalidSequenceLengthError(seq.len()))?;

        Self::from_parts(topic, data, seq)
    }

    pub fn from_parts(topic: &[u8], data: &[u8], seq: [u8; 4]) -> Result<Self> {
        let seq = u32::from_le_bytes(seq);

        Ok(match topic {
            b"hashblock" | b"hashtx" => {
                let mut data: [u8; 32] = data
                    .try_into()
                    .map_err(|_| Error::Invalid256BitHashLengthError(data.len()))?;
                data.reverse();

                match topic {
                    b"hashblock" => Self::HashBlock(BlockHash::from_byte_array(data), seq),
                    _ /* b"hashtx" */ => Self::HashTx(Txid::from_byte_array(data), seq),
                }
            }
            b"rawblock" => Self::Block(deserialize(data)?, seq),
            b"rawtx" => Self::Tx(deserialize(data)?, seq),
            b"sequence" => Self::Sequence(SequenceMessage::from_byte_slice(data)?, seq),
            _ => {
                let mut buf = [0; TOPIC_MAX_LEN];

                buf[0..topic.len()].copy_from_slice(topic);

                return Err(Error::InvalidTopicError(topic.len(), buf));
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

    fn try_from(value: [T; 3]) -> Result<Self> {
        Self::from_fixed_size_multipart(&value)
    }
}

impl From<Message> for [Vec<u8>; 3] {
    fn from(msg: Message) -> Self {
        [
            msg.topic().to_vec(),
            msg.content(),
            msg.sequence().to_le_bytes().to_vec(),
        ]
    }
}

impl From<Message> for Vec<Vec<u8>> {
    #[inline]
    fn from(msg: Message) -> Self {
        let arr: [Vec<u8>; 3] = msg.into();
        arr.to_vec()
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HashBlock(blockhash, seq) => write!(f, "HashBlock({blockhash}, sequence={seq})"),
            Self::HashTx(txid, seq) => write!(f, "HashTx({txid}, sequence={seq})"),
            Self::Block(block, seq) => write!(f, "Block({}, sequence={seq})", block.block_hash()),
            Self::Tx(tx, seq) => write!(f, "Tx({}, sequence={seq})", tx.txid()),
            Self::Sequence(sm, seq) => write!(f, "Sequence({sm}, sequence={seq})"),
        }
    }
}
