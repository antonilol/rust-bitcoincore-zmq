mod error;
mod message;
mod sequence_message;
mod subscribe;

pub use crate::{
    error::Error,
    message::{Message, DATA_MAX_LEN, SEQUENCE_LEN, TOPIC_MAX_LEN},
    sequence_message::SequenceMessage,
    subscribe::{
        blocking::{subscribe_multi_blocking, subscribe_single_blocking},
        receiver::{subscribe_multi, subscribe_single},
    },
};

#[cfg(feature = "async")]
pub use crate::subscribe::stream::{
    subscribe_async, subscribe_multi_async, MessageStream, MultiMessageStream,
};
