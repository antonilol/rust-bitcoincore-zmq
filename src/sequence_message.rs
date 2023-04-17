use crate::error::{Error, Result};
use bitcoin::{hashes::Hash, BlockHash, Txid};
use core::fmt;

#[derive(Debug)]
pub enum SequenceMessage {
    BlockConnect { blockhash: BlockHash },
    BlockDisconnect { blockhash: BlockHash },
    MempoolAcceptance { txid: Txid, mempool_sequence: u64 },
    MempoolRemoval { txid: Txid, mempool_sequence: u64 },
}

impl TryFrom<Vec<u8>> for SequenceMessage {
    type Error = Error;

    fn try_from(value: Vec<u8>) -> Result<Self> {
        if value.len() < 33 {
            return Err(Error::InvalidSequenceMessageLengthError(value.len()));
        }

        let label = value[32];
        Ok(match label {
            b'C' | b'D' => {
                if value.len() != 33 {
                    return Err(Error::InvalidSequenceMessageLengthError(value.len()));
                }

                let mut blockhash: [u8; 32] = value[0..32].try_into().expect(
                    "a 32-byte slice should always convert to a 32-byte array successfully",
                );
                blockhash.reverse();
                let blockhash = BlockHash::from_byte_array(blockhash);

                match label {
                    b'C' => Self::BlockConnect { blockhash },
                    _ /* b'D' */ => Self::BlockDisconnect { blockhash },
                }
            }
            b'A' | b'R' => {
                if value.len() != 41 {
                    return Err(Error::InvalidSequenceMessageLengthError(value.len()));
                }

                let mut txid: [u8; 32] = value[0..32].try_into().expect(
                    "a 32-byte slice should always convert to a 32-byte array successfully",
                );
                txid.reverse();
                let txid = Txid::from_byte_array(txid);
                let mempool_sequence = u64::from_le_bytes(value[33..41].try_into().expect(
                    "an 8-byte slice should always convert to an 8-byte array successfully",
                ));

                match label {
                    b'A' => Self::MempoolAcceptance { txid, mempool_sequence },
                    _ /* b'R' */ => Self::MempoolRemoval { txid, mempool_sequence },
                }
            }
            _ => return Err(Error::InvalidSequenceMessageLabelError(label)),
        })
    }
}

impl fmt::Display for SequenceMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SequenceMessage::BlockConnect { blockhash } => write!(f, "BlockConnect({blockhash})"),
            SequenceMessage::BlockDisconnect { blockhash } => {
                write!(f, "BlockDisconnect({blockhash})")
            }
            SequenceMessage::MempoolAcceptance {
                txid,
                mempool_sequence,
            } => write!(
                f,
                "MempoolAcceptance({txid}, mempool_sequence={mempool_sequence})"
            ),
            SequenceMessage::MempoolRemoval {
                txid,
                mempool_sequence,
            } => write!(
                f,
                "MempoolRemoval({txid}, mempool_sequence={mempool_sequence})"
            ),
        }
    }
}
