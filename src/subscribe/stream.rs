use super::{new_socket_internal, recv_internal};
use crate::{error::Result, message::Message, DATA_MAX_LEN};
use async_zmq::{Stream, StreamExt, Subscribe};
use core::{
    pin::Pin,
    task::{Context as AsyncContext, Poll},
};
use futures_util::stream::{Fuse, FusedStream};
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
}

impl Stream for MessageStream {
    type Item = Result<Message>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut AsyncContext<'_>) -> Poll<Option<Self::Item>> {
        self.as_mut().zmq_stream.poll_next_unpin(cx).map(|opt| {
            opt.map(|res| match res {
                Ok(mp) => recv_internal(mp.iter(), &mut self.data_cache),
                Err(err) => Err(err.into()),
            })
        })
    }
}

/// Stream that asynchronously produces [`Message`]s using multiple ZMQ subscriber.
pub struct MultiMessageStream {
    streams: Vec<Fuse<MessageStream>>,
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
        // Not sure if fuse is needed, but has to prevent use of closed streams.
        self.streams.push(MessageStream::new(stream).fuse());
    }
}

impl Stream for MultiMessageStream {
    type Item = Result<Message>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut AsyncContext<'_>) -> Poll<Option<Self::Item>> {
        let mut any_pending = false;

        let mut index_iter = (self.next..self.streams.len()).chain(0..self.next);
        while let Some(i) = index_iter.next() {
            match self.as_mut().streams[i].poll_next_unpin(cx) {
                msg @ Poll::Ready(Some(_)) => {
                    if let Some(next) = index_iter.next() {
                        self.next = next;
                    }
                    return msg;
                }
                Poll::Ready(None) => {
                    // continue
                }
                Poll::Pending => {
                    any_pending = true;
                }
            }
        }

        if any_pending {
            Poll::Pending
        } else {
            Poll::Ready(None)
        }
    }
}

impl FusedStream for MultiMessageStream {
    fn is_terminated(&self) -> bool {
        self.streams.iter().all(|stream| stream.is_terminated())
    }
}

pub fn subscribe_multi_async(endpoints: &[&str]) -> Result<MultiMessageStream> {
    let context = ZmqContext::new();
    let mut res = MultiMessageStream::new(endpoints.len());

    for endpoint in endpoints {
        let socket = new_socket_internal(&context, endpoint)?.into();
        res.push(socket);
    }

    Ok(res)
}

pub fn subscribe_single_async(endpoint: &str) -> Result<MessageStream> {
    Ok(MessageStream::new(
        new_socket_internal(&ZmqContext::new(), endpoint)?.into(),
    ))
}
