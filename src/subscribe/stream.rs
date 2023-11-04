use super::{new_socket_internal, recv_internal};
use crate::{error::Result, message::Message, DATA_MAX_LEN};
use async_zmq::{Stream, StreamExt, Subscribe};
use core::{
    pin::Pin,
    task::{Context as AsyncContext, Poll},
};
use futures_util::stream::FusedStream;
use zmq::Context as ZmqContext;

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
pub struct MultiMessageStream {
    streams: Vec<MessageStream>,
    next: usize,
}

impl MultiMessageStream {
    fn new(buf_capacity: usize) -> Self {
        Self {
            streams: Vec::with_capacity(buf_capacity),
            next: 0,
        }
    }

    fn push(&mut self, stream: Subscribe) {
        self.streams.push(MessageStream::new(stream));
    }

    /// Returns a reference to the separate [`MessageStream`]s this [`MultiMessageStream`] is made
    /// of. This is useful to set socket options or use other functions provided by [`zmq`] or
    /// [`async_zmq`]. (See [`MessageStream::as_zmq_socket`])
    pub fn as_streams(&self) -> &[MessageStream] {
        &self.streams
    }

    /// Returns the separate [`MessageStream`]s this [`MultiMessageStream`] is made of. This is
    /// useful to set socket options or use other functions provided by [`zmq`] or [`async_zmq`].
    /// (See [`MessageStream::as_zmq_socket`])
    pub fn into_streams(self) -> Vec<MessageStream> {
        self.streams
    }
}

impl Stream for MultiMessageStream {
    type Item = Result<Message>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut AsyncContext<'_>) -> Poll<Option<Self::Item>> {
        let mut index_iter = (self.next..self.streams.len()).chain(0..self.next);
        while let Some(i) = index_iter.next() {
            match self.streams[i].poll_next_unpin(cx) {
                msg @ Poll::Ready(Some(_)) => {
                    if let Some(next) = index_iter.next() {
                        self.next = next;
                    }
                    return msg;
                }
                Poll::Ready(None) => {
                    // should never be returned by async_zmq
                }
                Poll::Pending => {
                    // continue, poll others and eventually return Poll::Pending
                }
            }
        }

        Poll::Pending
    }
}

impl FusedStream for MultiMessageStream {
    fn is_terminated(&self) -> bool {
        false
    }
}

/// Subscribes to multiple ZMQ endpoints and returns a [`MultiMessageStream`].
pub fn subscribe_multi_async(endpoints: &[&str]) -> Result<MultiMessageStream> {
    let context = ZmqContext::new();
    let mut res = MultiMessageStream::new(endpoints.len());

    for endpoint in endpoints {
        let socket = new_socket_internal(&context, endpoint)?.into();
        res.push(socket);
    }

    Ok(res)
}

/// Subscribes to a single ZMQ endpoint and returns a [`MessageStream`].
pub fn subscribe_single_async(endpoint: &str) -> Result<MessageStream> {
    Ok(MessageStream::new(
        new_socket_internal(&ZmqContext::new(), endpoint)?.into(),
    ))
}
