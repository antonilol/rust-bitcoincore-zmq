use crate::{
    error::Result,
    message::{Message, SEQUENCE_LEN, TOPIC_MAX_LEN},
    Error, DATA_MAX_LEN,
};
use core::{cmp::min, convert::Infallible, ops::ControlFlow, slice};
use std::{
    sync::mpsc::{channel, Receiver},
    thread,
};
use zmq::{Context, Socket};

fn break_on_err(is_err: bool) -> ControlFlow<()> {
    if is_err {
        ControlFlow::Break(())
    } else {
        ControlFlow::Continue(())
    }
}

/// Subscribes to a single ZMQ endpoint and returns a [`Receiver`].
#[inline]
pub fn subscribe_single(endpoint: &str) -> Result<Receiver<Result<Message>>> {
    let (tx, rx) = channel();
    let context = Context::new();

    let socket = new_socket_internal(&context, endpoint)?;

    thread::spawn(move || subscribe_internal(socket, |msg| break_on_err(tx.send(msg).is_err())));

    Ok(rx)
}

/// Subscribes to multiple ZMQ endpoints and returns a [`Receiver`].
#[inline]
pub fn subscribe_multi(endpoints: &[&str]) -> Result<Receiver<Result<Message>>> {
    let (tx, rx) = channel();
    let context = Context::new();

    for endpoint in endpoints {
        let tx = tx.clone();

        let socket = new_socket_internal(&context, endpoint)?;

        thread::spawn(move || {
            subscribe_internal(socket, |msg| break_on_err(tx.send(msg).is_err()))
        });
    }

    Ok(rx)
}

/// Subscribes to a single ZMQ endpoint and blocks the thread until [`ControlFlow::Break`] is
/// returned by the callback.
#[inline]
pub fn subscribe_single_blocking<F, B>(
    endpoint: &str,
    callback: F,
) -> Result<ControlFlow<B, Infallible>>
where
    F: Fn(Result<Message>) -> ControlFlow<B>,
{
    let context = Context::new();

    let socket = new_socket_internal(&context, endpoint)?;

    Ok(subscribe_internal(socket, callback))
}

/// Subscribes to multiple ZMQ endpoints and blocks the thread until [`ControlFlow::Break`] is
/// returned by the callback.
#[inline]
pub fn subscribe_multi_blocking<F, B>(
    endpoints: &[&str],
    callback: F,
) -> Result<ControlFlow<B, Infallible>>
where
    F: Fn(Result<Message>) -> ControlFlow<B>,
{
    let (tx, rx) = channel();
    let context = Context::new();

    for endpoint in endpoints {
        let tx = tx.clone();

        let socket = new_socket_internal(&context, endpoint)?;

        thread::spawn(move || {
            subscribe_internal(socket, |msg| break_on_err(tx.send(msg).is_err()))
        });
    }

    Ok((|| {
        for msg in rx {
            callback(msg)?;
        }

        // `tx` is dropped at the end of this function
        unreachable!();
    })())
}

#[inline]
fn new_socket_internal(context: &Context, endpoint: &str) -> Result<Socket> {
    let socket = context.socket(zmq::SUB)?;
    socket.connect(endpoint)?;
    socket.set_subscribe(b"")?;

    Ok(socket)
}

pub(crate) trait ReceiveFrom {
    fn has_next(&self) -> Result<bool>;

    fn receive_into(&mut self, buf: &mut [u8]) -> Result<usize>;
}

impl ReceiveFrom for &Socket {
    fn has_next(&self) -> Result<bool> {
        Ok(self.get_rcvmore()?)
    }

    fn receive_into(&mut self, buf: &mut [u8]) -> Result<usize> {
        Ok(self.recv_into(buf, 0)?)
    }
}

impl ReceiveFrom for slice::Iter<'_, zmq::Message> {
    fn has_next(&self) -> Result<bool> {
        Ok(!self.as_slice().is_empty())
    }

    fn receive_into(&mut self, buf: &mut [u8]) -> Result<usize> {
        // TODO better way to handle None than unwrap
        let bytes = &**self.next().unwrap();
        let len = bytes.len();
        let copy_len = min(len, buf.len());
        buf[0..copy_len].copy_from_slice(&bytes[0..copy_len]);
        Ok(len)
    }
}

#[inline]
pub(crate) fn recv_internal<R: ReceiveFrom>(
    mut socket: R,
    data: &mut [u8; DATA_MAX_LEN],
) -> Result<Message> {
    let mut topic = [0u8; TOPIC_MAX_LEN];
    let mut sequence = [0u8; SEQUENCE_LEN];

    let topic_len = socket.receive_into(&mut topic)?;
    if topic_len > TOPIC_MAX_LEN {
        return Err(Error::InvalidTopic(topic_len, topic));
    }

    if !socket.has_next()? {
        return Err(Error::InvalidMutlipartLength(1));
    }

    let data_len = socket.receive_into(data)?;
    if data_len > DATA_MAX_LEN {
        return Err(Error::InvalidDataLength(data_len));
    }

    if !socket.has_next()? {
        return Err(Error::InvalidMutlipartLength(2));
    }

    let sequence_len = socket.receive_into(&mut sequence)?;
    if sequence_len != SEQUENCE_LEN {
        return Err(Error::InvalidSequenceLength(sequence_len));
    }

    if !socket.has_next()? {
        return Message::from_parts(&topic[0..topic_len], &data[0..data_len], sequence);
    }

    let mut len = 3;

    loop {
        socket.receive_into(&mut [])?;

        len += 1;

        if !socket.has_next()? {
            return Err(Error::InvalidMutlipartLength(len));
        }
    }
}

#[inline]
fn subscribe_internal<F, B>(socket: Socket, callback: F) -> ControlFlow<B, Infallible>
where
    F: Fn(Result<Message>) -> ControlFlow<B>,
{
    let mut data: Box<[u8; DATA_MAX_LEN]> =
        vec![0; DATA_MAX_LEN].into_boxed_slice().try_into().unwrap();

    loop {
        let msg = recv_internal(&socket, &mut data);

        callback(msg)?;
    }
}
