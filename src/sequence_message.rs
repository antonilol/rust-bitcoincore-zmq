use crate::error::{Error, Result};
use bitcoin::{hashes::Hash, BlockHash, Txid};
use core::fmt;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SequenceMessage {
    BlockConnect { blockhash: BlockHash },
    BlockDisconnect { blockhash: BlockHash },
    MempoolAcceptance { txid: Txid, mempool_sequence: u64 },
    MempoolRemoval { txid: Txid, mempool_sequence: u64 },
}

impl SequenceMessage {
    /// Returns the length of this [`SequenceMessage`] when serialized.
    pub fn raw_length(&self) -> usize {
        match self {
            Self::BlockConnect { .. } | Self::BlockDisconnect { .. } => 33,
            Self::MempoolAcceptance { .. } | Self::MempoolRemoval { .. } => 41,
        }
    }

    /// Returns the label of this [`SequenceMessage`] as a [`char`].
    pub fn label_char(&self) -> char {
        self.label() as char
    }

    /// Returns the label of this [`SequenceMessage`] as a [`u8`].
    pub fn label(&self) -> u8 {
        match self {
            Self::BlockConnect { .. } => b'C',
            Self::BlockDisconnect { .. } => b'D',
            Self::MempoolAcceptance { .. } => b'A',
            Self::MempoolRemoval { .. } => b'R',
        }
    }

    /// Returns the contained hash (block hash or txid) of this [`SequenceMessage`].
    pub fn inner_hash_as_bytes(&self) -> [u8; 32] {
        let mut arr = match self {
            Self::BlockConnect { blockhash } | Self::BlockDisconnect { blockhash } => {
                blockhash.to_byte_array()
            }
            Self::MempoolAcceptance { txid, .. } | Self::MempoolRemoval { txid, .. } => {
                txid.to_byte_array()
            }
        };
        arr.reverse();
        arr
    }

    #[deprecated(
        since = "1.0.7",
        note = "this function was renamed to `inner_hash_as_bytes` and does not consume self anymore"
    )]
    pub fn into_inner(self) -> [u8; 32] {
        self.inner_hash_as_bytes()
    }

    /// Returns the mempool sequence of this [`SequenceMessage`] if it is [`MempoolAcceptance`] or
    /// [`MempoolRemoval`]. This is a number that starts at 1 and goes up every time Bitcoin Core
    /// adds or removes a transaction to the mempool.
    ///
    /// Note that transactions that got removed from the mempool because they were included in a
    /// block increment Bitcoin Core's mempool sequence, they do not produce a [`MempoolRemoval`]
    /// message.
    ///
    /// [`MempoolAcceptance`]: SequenceMessage::MempoolAcceptance
    /// [`MempoolRemoval`]: SequenceMessage::MempoolRemoval
    pub fn mempool_sequence(&self) -> Option<u64> {
        match self {
            Self::BlockConnect { .. } | Self::BlockDisconnect { .. } => None,
            Self::MempoolAcceptance {
                mempool_sequence, ..
            }
            | Self::MempoolRemoval {
                mempool_sequence, ..
            } => Some(*mempool_sequence),
        }
    }

    /// Deserializes bytes to a [`SequenceMessage`]
    pub fn from_byte_slice<T: AsRef<[u8]>>(bytes: T) -> Result<Self> {
        let bytes = bytes.as_ref();

        if bytes.len() < 33 {
            return Err(Error::InvalidSequenceMessageLengthError(bytes.len()));
        }

        let mut hash: [u8; 32] = bytes[0..32].try_into().unwrap();
        hash.reverse();

        let label = bytes[32];
        Ok(match label {
            b'C' | b'D' => {
                if bytes.len() != 33 {
                    return Err(Error::InvalidSequenceMessageLengthError(bytes.len()));
                }

                let blockhash = BlockHash::from_byte_array(hash);

                match label {
                    b'C' => Self::BlockConnect { blockhash },
                    _ /* b'D' */ => Self::BlockDisconnect { blockhash },
                }
            }
            b'A' | b'R' => {
                if bytes.len() != 41 {
                    return Err(Error::InvalidSequenceMessageLengthError(bytes.len()));
                }

                let txid = Txid::from_byte_array(hash);
                let mempool_sequence = u64::from_le_bytes(bytes[33..41].try_into().unwrap());

                match label {
                    b'A' => Self::MempoolAcceptance { txid, mempool_sequence },
                    _ /* b'R' */ => Self::MempoolRemoval { txid, mempool_sequence },
                }
            }
            _ => return Err(Error::InvalidSequenceMessageLabelError(label)),
        })
    }

    /// Serializes a [`SequenceMessage`] to bytes
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut ret = Vec::with_capacity(self.raw_length());

        // blockhash or txid
        ret.extend_from_slice(&self.inner_hash_as_bytes());

        // label
        ret.push(self.label());

        // optional mempool sequence
        if let Some(mempool_sequence) = self.mempool_sequence() {
            ret.extend_from_slice(&mempool_sequence.to_le_bytes());
        }

        ret
    }
}

impl TryFrom<Vec<u8>> for SequenceMessage {
    type Error = Error;

    fn try_from(value: Vec<u8>) -> Result<Self> {
        Self::from_byte_slice(value)
    }
}

impl TryFrom<&[u8]> for SequenceMessage {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        Self::from_byte_slice(value)
    }
}

impl From<SequenceMessage> for Vec<u8> {
    fn from(sm: SequenceMessage) -> Self {
        sm.as_bytes()
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

#[cfg(test)]
mod tests {
    use crate::SequenceMessage;
    use bitcoin::{constants::genesis_block, hashes::Hash, Network};

    #[test]
    fn serialization() {
        let genesis_block = genesis_block(Network::Bitcoin);

        let blockhash = genesis_block.block_hash();
        let mut blockhash_bytes = blockhash.to_byte_array();
        blockhash_bytes.reverse();

        let txid = genesis_block.txdata[0].txid();
        let mut txid_bytes = txid.to_byte_array();
        txid_bytes.reverse();

        let connect_message = SequenceMessage::BlockConnect { blockhash };
        let connect_bytes = connect_message.as_bytes();
        assert_eq!(connect_message.raw_length(), connect_bytes.len());
        assert_eq!(connect_message.raw_length(), 32 + 1);
        assert_eq!(connect_message, connect_bytes.try_into().unwrap());

        assert_eq!(connect_message.label_char(), 'C');
        assert_eq!(connect_message.inner_hash_as_bytes(), blockhash_bytes);
        assert_eq!(connect_message.mempool_sequence(), None);

        let disconnect_message = SequenceMessage::BlockDisconnect { blockhash };
        let disconnect_bytes = disconnect_message.as_bytes();
        assert_eq!(disconnect_message.raw_length(), disconnect_bytes.len());
        assert_eq!(disconnect_message.raw_length(), 32 + 1);
        assert_eq!(disconnect_message, disconnect_bytes.try_into().unwrap());

        assert_eq!(disconnect_message.label_char(), 'D');
        assert_eq!(disconnect_message.inner_hash_as_bytes(), blockhash_bytes);
        assert_eq!(disconnect_message.mempool_sequence(), None);

        let accept_message = SequenceMessage::MempoolAcceptance {
            txid,
            mempool_sequence: 1,
        };
        let accept_bytes = accept_message.as_bytes();
        assert_eq!(accept_message.raw_length(), accept_bytes.len());
        assert_eq!(accept_message.raw_length(), 32 + 1 + 8);
        assert_eq!(accept_message, accept_bytes.try_into().unwrap());

        assert_eq!(accept_message.label_char(), 'A');
        assert_eq!(accept_message.inner_hash_as_bytes(), txid_bytes);
        assert_eq!(accept_message.mempool_sequence(), Some(1));

        let remove_message = SequenceMessage::MempoolRemoval {
            txid,
            mempool_sequence: 2,
        };
        let remove_bytes = remove_message.as_bytes();
        assert_eq!(remove_message.raw_length(), remove_bytes.len());
        assert_eq!(remove_message.raw_length(), 32 + 1 + 8);
        assert_eq!(remove_message, remove_bytes.try_into().unwrap());

        assert_eq!(remove_message.label_char(), 'R');
        assert_eq!(remove_message.inner_hash_as_bytes(), txid_bytes);
        assert_eq!(remove_message.mempool_sequence(), Some(2));
    }
}
