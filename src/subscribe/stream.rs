use super::{new_socket_internal, recv_internal};
use crate::{error::Result, message::Message, DATA_MAX_LEN};
use async_zmq::{Stream, StreamExt, Subscribe};
use core::{
    pin::Pin,
    slice,
    task::{Context as AsyncContext, Poll},
};
use futures_util::stream::FusedStream;

/// Stream that asynchronously produces [`Message`]s using a ZMQ subscriber.
pub struct MessageStream {
    zmq_stream: Subscribe,
    data_cache: Box<[u8; DATA_MAX_LEN]>,
}

impl MessageStream {
    fn new(zmq_stream: Subscribe) -> Self {
        Self {
            zmq_stream,
            data_cache: vec![0; DATA_MAX_LEN].into_boxed_slice().try_into().unwrap(),
        }
    }

    /// Returns a reference to the ZMQ socket used by this stream. To get the [`zmq::Socket`], use
    /// [`as_raw_socket`] on the result. This is useful to set socket options or use other
    /// functions provided by [`zmq`] or [`async_zmq`].
    ///
    /// [`as_raw_socket`]: Subscribe::as_raw_socket
    pub fn as_zmq_socket(&self) -> &Subscribe {
        &self.zmq_stream
    }
}

impl Stream for MessageStream {
    type Item = Result<Message>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut AsyncContext<'_>) -> Poll<Option<Self::Item>> {
        self.zmq_stream.poll_next_unpin(cx).map(|opt| {
            opt.map(|res| match res {
                Ok(mp) => recv_internal(mp.iter(), &mut self.data_cache),
                Err(err) => Err(err.into()),
            })
        })
    }
}

impl FusedStream for MessageStream {
    fn is_terminated(&self) -> bool {
        false
    }
}

/// Stream that asynchronously produces [`Message`]s using multiple ZMQ subscribers. The ZMQ
/// sockets are polled in a round-robin fashion.
#[deprecated(
    since = "1.3.2",
    note = "This struct is only used by deprecated functions."
)]
pub struct MultiMessageStream(pub MessageStream);

#[allow(deprecated)]
impl MultiMessageStream {
    /// Returns a reference to the separate [`MessageStream`]s this [`MultiMessageStream`] is made
    /// of. This is useful to set socket options or use other functions provided by [`zmq`] or
    /// [`async_zmq`]. (See [`MessageStream::as_zmq_socket`])
    pub fn as_streams(&self) -> &[MessageStream] {
        slice::from_ref(&self.0)
    }

    /// Returns the separate [`MessageStream`]s this [`MultiMessageStream`] is made of. This is
    /// useful to set socket options or use other functions provided by [`zmq`] or [`async_zmq`].
    /// (See [`MessageStream::as_zmq_socket`])
    pub fn into_streams(self) -> Vec<MessageStream> {
        vec![self.0]
    }
}

#[allow(deprecated)]
impl Stream for MultiMessageStream {
    type Item = Result<Message>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut AsyncContext<'_>) -> Poll<Option<Self::Item>> {
        self.0.poll_next_unpin(cx)
    }
}

#[allow(deprecated)]
impl FusedStream for MultiMessageStream {
    fn is_terminated(&self) -> bool {
        false
    }
}

/// Subscribes to multiple ZMQ endpoints and returns a [`MultiMessageStream`].
#[deprecated(
    since = "1.3.2",
    note = "Use subscribe_async. This function has no performance benefit over subscribe_single_async anymore."
)]
#[allow(deprecated)]
pub fn subscribe_multi_async(endpoints: &[&str]) -> Result<MultiMessageStream> {
    subscribe_async(endpoints).map(MultiMessageStream)
}

/// Subscribes to a single ZMQ endpoint and returns a [`MessageStream`].
#[deprecated(
    since = "1.3.2",
    note = "Use subscribe_async. The name changed because there is no distinction made anymore between subscribing to 1 or more endpoints."
)]
pub fn subscribe_single_async(endpoint: &str) -> Result<MessageStream> {
    subscribe_async(&[endpoint])
}

/// Subscribes to multiple ZMQ endpoints and returns a [`MessageStream`].
pub fn subscribe_async(endpoints: &[&str]) -> Result<MessageStream> {
    let (_context, socket) = new_socket_internal(endpoints)?;

    Ok(MessageStream::new(socket.into()))
}
