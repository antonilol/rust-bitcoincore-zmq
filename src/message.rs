use crate::{
    error::{Error, Result},
    sequence_message::SequenceMessage,
};
use bitcoin::{consensus::deserialize, hashes::Hash, Block, BlockHash, Transaction, Txid};
use core::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum Message {
    HashBlock(BlockHash, u32),
    HashTx(Txid, u32),
    Block(Block, u32),
    Tx(Transaction, u32),
    Sequence(SequenceMessage, u32),
}

impl TryFrom<Vec<Vec<u8>>> for Message {
    type Error = Error;

    fn try_from(mut value: Vec<Vec<u8>>) -> Result<Self> {
        if value.len() != 3 {
            return Err(Error::InvalidMutlipartLengthError);
        }

        let seq = value.pop().unwrap();
        let mut content = value.pop().unwrap();
        let topic = value.pop().unwrap();

        let seq = u32::from_le_bytes(
            seq.try_into()
                .map_err(|_| Error::InvalidSequenceLengthError)?,
        );

        // hashblock, hashtx
        if topic[0] == b'h' {
            content.reverse();
        }

        Ok(match &topic[..] {
            b"hashblock" => Self::HashBlock(BlockHash::from_slice(&content)?, seq),
            b"hashtx" => Self::HashTx(Txid::from_slice(&content)?, seq),
            b"rawblock" => Self::Block(deserialize(&content)?, seq),
            b"rawtx" => Self::Tx(deserialize(&content)?, seq),
            b"sequence" => Self::Sequence(content.try_into()?, seq),
            _ => return Err(Error::InvalidTopicError(topic)),
        })
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
