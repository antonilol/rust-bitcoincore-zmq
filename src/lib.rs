mod error;
mod message;
mod sequence_message;
mod subscribe;
#[cfg(feature = "async")]
mod subscribe_async;

pub use crate::{
    error::Error,
    message::{Message, DATA_MAX_LEN, SEQUENCE_LEN, TOPIC_MAX_LEN},
    sequence_message::SequenceMessage,
    subscribe::{
        subscribe_multi, subscribe_multi_blocking, subscribe_single, subscribe_single_blocking,
    },
};

#[cfg(feature = "async")]
pub use crate::subscribe_async::{
    subscribe_async, subscribe_multi_async, MessageStream, MultiMessageStream,
};
