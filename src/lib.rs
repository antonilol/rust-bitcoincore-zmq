#![cfg_attr(docsrs, feature(doc_cfg))]

mod error;
mod message;
mod monitor;
mod subscribe;

pub use crate::error::{Error, Result};
pub use crate::message::{
    Message, MessageContent, RawMessage, SequenceMessage, Topic, UnknownTopicError, SEQUENCE_LEN,
};
pub use crate::monitor::event::{HandshakeFailure, SocketEvent};
pub use crate::monitor::MonitorMessage;
pub use crate::subscribe::{blocking::subscribe_blocking, receiver::subscribe_receiver};

#[cfg(feature = "async")]
pub use crate::subscribe::stream::{
    subscribe_async, subscribe_async_monitor, subscribe_async_monitor_stream,
    subscribe_async_stream, subscribe_async_wait_handshake, subscribe_async_wait_handshake_timeout,
    SocketMessage,
};
