use crate::{
    error::{Error, Result},
    sequence_message::SequenceMessage,
};
use bitcoin::{
    consensus::{deserialize, serialize},
    hashes::Hash,
    Block, BlockHash, Transaction, Txid,
};
use core::fmt;

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
        match self {
            Self::HashBlock(..) => "hashblock",
            Self::HashTx(..) => "hashtx",
            Self::Block(..) => "block",
            Self::Tx(..) => "tx",
            Self::Sequence(..) => "sequence",
        }
    }

    /// Serializes the middle part of this [`Message`] (no topic and sequence).
    #[inline]
    pub fn content(&self) -> Vec<u8> {
        let mut arr = match self {
            Self::HashBlock(blockhash, _) => blockhash.to_byte_array(),
            Self::HashTx(txid, _) => txid.to_byte_array(),
            Self::Block(block, _) => return serialize(&block),
            Self::Tx(tx, _) => return serialize(&tx),
            Self::Sequence(sm, _) => return (*sm).into(),
        };
        arr.reverse();
        arr.to_vec()
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
}

impl TryFrom<[Vec<u8>; 3]> for Message {
    type Error = Error;

    fn try_from(value: [Vec<u8>; 3]) -> Result<Self> {
        let [topic, content, seq] = value;

        if seq.len() != 4 {
            return Err(Error::InvalidSequenceLengthError(seq.len()));
        }
        let seq = u32::from_le_bytes(seq.try_into().unwrap());

        Ok(match &topic[..] {
            b"hashblock" | b"hashtx" => {
                if content.len() != 32 {
                    return Err(Error::Invalid256BitHashLengthError(content.len()));
                }
                let mut content: [u8; 32] = content.try_into().unwrap();
                content.reverse();

                match &topic[..] {
                    b"hashblock" => Self::HashBlock(BlockHash::from_byte_array(content), seq),
                    _ /* b"hashtx" */ => Self::HashTx(Txid::from_byte_array(content), seq),
                }
            }
            b"rawblock" => Self::Block(deserialize(&content)?, seq),
            b"rawtx" => Self::Tx(deserialize(&content)?, seq),
            b"sequence" => Self::Sequence(content.try_into()?, seq),
            _ => return Err(Error::InvalidTopicError(topic)),
        })
    }
}

impl TryFrom<Vec<Vec<u8>>> for Message {
    type Error = Error;

    #[inline]
    fn try_from(value: Vec<Vec<u8>>) -> Result<Self> {
        if value.len() != 3 {
            return Err(Error::InvalidMutlipartLengthError(value.len()));
        }
        let arr: [Vec<u8>; 3] = value.try_into().unwrap();
        arr.try_into()
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
