#![cfg_attr(docsrs, feature(doc_auto_cfg))]

mod error;
mod message;
mod monitor;
mod sequence_message;
mod subscribe;

pub use crate::{
    error::Error,
    message::{Message, DATA_MAX_LEN, SEQUENCE_LEN, TOPIC_MAX_LEN},
    monitor::{
        event::{HandshakeFailure, SocketEvent},
        MonitorMessage,
    },
    sequence_message::SequenceMessage,
    subscribe::{blocking::subscribe_blocking, receiver::subscribe_receiver},
};

#[cfg(feature = "async")]
pub use crate::subscribe::stream::{
    subscribe_async, subscribe_async_monitor, subscribe_async_monitor_stream,
    subscribe_async_stream::{self, MessageStream},
    subscribe_async_wait_handshake, subscribe_async_wait_handshake_timeout, SocketMessage,
};

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
