use super::new_socket_internal;
use crate::{
    error::Result,
    message::Message,
    monitor::{event::SocketEvent, MonitorMessage, MonitorMessageError},
};
use core::{
    fmt,
    future::Future,
    mem,
    pin::{pin, Pin},
    slice,
    task::{Context as AsyncContext, Poll, Waker},
    time::Duration,
};
use futures_util::{
    future::{select, Either},
    stream::{FusedStream, Stream, StreamExt},
};
use std::{
    sync::{Arc, Mutex},
    thread,
};

/// A [`Message`] or a [`MonitorMessage`].
#[derive(Debug, Clone)]
pub enum SocketMessage {
    Message(Message),
    Event(MonitorMessage),
}

/// Stream that asynchronously produces [`Message`]s using multiple ZMQ subscribers. The ZMQ
/// sockets are polled in a round-robin fashion.
#[deprecated(
    since = "1.3.2",
    note = "This struct is only used by deprecated functions."
)]
pub struct MultiMessageStream(pub subscribe_async_stream::MessageStream);

#[allow(deprecated)]
impl MultiMessageStream {
    /// Returns a reference to the separate `MessageStream`s this [`MultiMessageStream`] is made
    /// of. This is useful to set socket options or use other functions provided by [`zmq`] or
    /// [`async_zmq`]. (See `MessageStream::as_zmq_socket`)
    pub fn as_streams(&self) -> &[subscribe_async_stream::MessageStream] {
        slice::from_ref(&self.0)
    }

    /// Returns the separate `MessageStream`s this [`MultiMessageStream`] is made of. This is
    /// useful to set socket options or use other functions provided by [`zmq`] or [`async_zmq`].
    /// (See `MessageStream::as_zmq_socket`)
    pub fn into_streams(self) -> Vec<subscribe_async_stream::MessageStream> {
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

/// Subscribes to a single ZMQ endpoint and returns a `MessageStream`.
#[deprecated(
    since = "1.3.2",
    note = "Use subscribe_async. The name changed because there is no distinction made anymore between subscribing to 1 or more endpoints."
)]
pub fn subscribe_single_async(endpoint: &str) -> Result<subscribe_async_stream::MessageStream> {
    subscribe_async(&[endpoint])
}

pub mod subscribe_async_stream {
    use crate::{error::Result, message::Message, subscribe::message_from_multipart_zmq_message};
    use async_zmq::Subscribe;
    use core::{
        pin::Pin,
        task::{Context as AsyncContext, Poll},
    };
    use futures_util::{
        stream::{FusedStream, StreamExt},
        Stream,
    };

    /// Stream returned by [`subscribe_async`][super::subscribe_async].
    pub struct MessageStream {
        zmq_stream: Subscribe,
    }

    impl MessageStream {
        pub(super) fn new(zmq_stream: Subscribe) -> Self {
            Self { zmq_stream }
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

        fn poll_next(
            mut self: Pin<&mut Self>,
            cx: &mut AsyncContext<'_>,
        ) -> Poll<Option<Self::Item>> {
            self.zmq_stream.poll_next_unpin(cx).map(|opt| {
                Some(match opt.unwrap() {
                    Ok(mp) => message_from_multipart_zmq_message(&mp),
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
}

/// Subscribes to multiple ZMQ endpoints and returns a stream that produces [`Message`]s.
pub fn subscribe_async(endpoints: &[&str]) -> Result<subscribe_async_stream::MessageStream> {
    let (_context, socket) = new_socket_internal(endpoints)?;

    Ok(subscribe_async_stream::MessageStream::new(socket.into()))
}

pub mod subscribe_async_monitor_stream {
    use super::{subscribe_async_stream, SocketMessage};
    use crate::{error::Result, monitor::MonitorMessage};
    use async_zmq::Subscribe;
    use core::{
        pin::Pin,
        task::{Context as AsyncContext, Poll},
    };
    use futures_util::{
        stream::{FusedStream, StreamExt},
        Stream,
    };
    use zmq::Socket;

    pub(super) enum Empty {}

    impl Iterator for Empty {
        type Item = Empty;

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    impl From<Empty> for async_zmq::Message {
        fn from(val: Empty) -> Self {
            match val {}
        }
    }

    // The generic type params don't matter as this will only be used for receiving
    // Better to use an empty type to not waste precious bytes
    pub(super) type RecvOnlyPair = async_zmq::Pair<Empty, Empty>;

    /// Stream returned by [`subscribe_async_monitor`][super::subscribe_async_monitor].
    pub struct MessageStream {
        messages: subscribe_async_stream::MessageStream,
        pub(super) monitor: RecvOnlyPair,
    }

    impl MessageStream {
        pub(super) fn new(
            messages: subscribe_async_stream::MessageStream,
            monitor: RecvOnlyPair,
        ) -> Self {
            Self { messages, monitor }
        }

        /// Returns a reference to the ZMQ socket used by this stream. To get the [`zmq::Socket`], use
        /// [`as_raw_socket`] on the result. This is useful to set socket options or use other
        /// functions provided by [`zmq`] or [`async_zmq`].
        ///
        /// [`as_raw_socket`]: Subscribe::as_raw_socket
        pub fn as_zmq_socket(&self) -> &Subscribe {
            self.messages.as_zmq_socket()
        }

        /// Returns a reference to the ZMQ monitor socket used by this stream. This is useful to
        /// set socket options or use other functions provided by [`zmq`].
        pub fn as_zmq_monitor_socket(&self) -> &Socket {
            self.monitor.as_raw_socket()
        }
    }

    impl Stream for MessageStream {
        type Item = Result<SocketMessage>;

        fn poll_next(
            mut self: Pin<&mut Self>,
            cx: &mut AsyncContext<'_>,
        ) -> Poll<Option<Self::Item>> {
            match self.monitor.poll_next_unpin(cx) {
                Poll::Ready(msg) => {
                    return Poll::Ready(Some(Ok(SocketMessage::Event(
                        MonitorMessage::parse_from(&msg.unwrap()?)?,
                    ))));
                }
                Poll::Pending => {}
            }

            self.messages
                .poll_next_unpin(cx)
                .map(|opt| Some(opt.unwrap().map(SocketMessage::Message)))
        }
    }

    impl FusedStream for MessageStream {
        fn is_terminated(&self) -> bool {
            false
        }
    }
}

/// Subscribes to multiple ZMQ endpoints and returns a stream that yields [`Message`]s and events
/// (see [`MonitorMessage`]).
pub fn subscribe_async_monitor(
    endpoints: &[&str],
) -> Result<subscribe_async_monitor_stream::MessageStream> {
    let (context, socket) = new_socket_internal(endpoints)?;

    socket.monitor("inproc://monitor", zmq::SocketEvent::ALL as i32)?;

    let monitor = context.socket(zmq::PAIR)?;
    monitor.connect("inproc://monitor")?;

    Ok(subscribe_async_monitor_stream::MessageStream::new(
        subscribe_async_stream::MessageStream::new(socket.into()),
        monitor.into(),
    ))
}

// TODO have some way to extract connecting to which endpoints failed, now just a (unit) error is returned (by tokio::time::timeout)

/// Subscribes to multiple ZMQ endpoints and returns a stream that yields [`Message`]s and events
/// (see [`MonitorMessage`]). This method will wait until a connection has been established to all
/// endpoints.
///
/// See examples/subscribe_async_timeout.rs for a usage example.
///
/// **NOTE:** This method will wait indefinitely until a connection has been established, but this is
/// often undesirable. This method should therefore be used in combination with your async
/// runtime's timeout function. Currently, with the state of async Rust in December of 2023, it is
/// not yet possible do this without creating an extra thread per timeout or depending on specific
/// runtimes.
pub async fn subscribe_async_wait_handshake(
    endpoints: &[&str],
) -> Result<subscribe_async_monitor_stream::MessageStream> {
    let mut stream = subscribe_async_monitor(endpoints)?;
    let mut connecting = endpoints.len();

    if connecting == 0 {
        return Ok(stream);
    }

    loop {
        let msg: &[zmq::Message] = &stream.monitor.next().await.unwrap()?;
        let [event_message, _] = msg else {
            return Err(MonitorMessageError::InvalidMutlipartLength(msg.len()).into());
        };
        match SocketEvent::parse_from(event_message)? {
            SocketEvent::HandshakeSucceeded => {
                connecting -= 1;
            }
            SocketEvent::Disconnected { .. } => {
                connecting += 1;
            }
            _ => {
                continue;
            }
        }
        if connecting == 0 {
            return Ok(stream);
        }
    }
}

/// See [`subscribe_async_wait_handshake`]. This method implements the inefficient, but runtime
/// independent approach.
pub async fn subscribe_async_wait_handshake_timeout(
    endpoints: &[&str],
    timeout: Duration,
) -> core::result::Result<Result<subscribe_async_monitor_stream::MessageStream>, Timeout> {
    let subscribe = subscribe_async_wait_handshake(endpoints);
    let timeout = sleep(timeout);

    match select(pin!(subscribe), timeout).await {
        Either::Left((res, _)) => Ok(res),
        Either::Right(_) => Err(Timeout(())),
    }
}

/// Error returned by [`subscribe_async_wait_handshake_timeout`] when the connection times out.
/// Contains no information, but does have a Error, Debug and Display impl.
#[derive(Debug)]
pub struct Timeout(());

impl fmt::Display for Timeout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "connection timed out")
    }
}

impl std::error::Error for Timeout {}

fn sleep(dur: Duration) -> Sleep {
    let state = Arc::new(Mutex::new(SleepReadyState::Pending));
    {
        let state = state.clone();
        thread::spawn(move || {
            thread::sleep(dur);
            let state = {
                let mut g = state.lock().unwrap();
                mem::replace(&mut *g, SleepReadyState::Done)
            };
            if let SleepReadyState::PendingPolled(waker) = state {
                waker.wake();
            }
        });
    }

    Sleep(state)
}

enum SleepReadyState {
    Pending,
    PendingPolled(Waker),
    Done,
}

struct Sleep(Arc<Mutex<SleepReadyState>>);

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut AsyncContext<'_>) -> Poll<Self::Output> {
        let mut g = self.0.lock().unwrap();
        if let SleepReadyState::Done = *g {
            Poll::Ready(())
        } else {
            *g = SleepReadyState::PendingPolled(cx.waker().clone());
            Poll::Pending
        }
    }
}
