#![cfg_attr(docsrs, feature(doc_auto_cfg))]

mod error;
mod message;
mod sequence_message;
mod subscribe;

pub use crate::{
    error::Error,
    message::{Message, DATA_MAX_LEN, SEQUENCE_LEN, TOPIC_MAX_LEN},
    sequence_message::SequenceMessage,
    subscribe::{blocking::subscribe_blocking, receiver::subscribe_receiver},
};

#[cfg(feature = "async")]
pub use crate::subscribe::stream::{subscribe_async, MessageStream};

#[allow(deprecated)]
pub use crate::subscribe::{
    blocking::{subscribe_multi_blocking, subscribe_single_blocking},
    receiver::{subscribe_multi, subscribe_single},
};

#[cfg(feature = "async")]
#[allow(deprecated)]
pub use crate::subscribe::stream::{
    subscribe_multi_async, subscribe_single_async, MultiMessageStream,
};
