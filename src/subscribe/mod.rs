pub mod blocking;
pub mod receiver;
#[cfg(feature = "async")]
pub mod stream;

use crate::{
    error::Result,
    message::{Message, SEQUENCE_LEN, TOPIC_MAX_LEN},
    Error, DATA_MAX_LEN,
};
use core::{cmp::min, convert::Infallible, ops::ControlFlow, slice};
use zmq::{Context, Socket};

pub(super) fn new_socket_internal(endpoints: &[&str]) -> Result<(Context, Socket)> {
    let context = Context::new();

    let socket = context.socket(zmq::SUB)?;
    socket.set_subscribe(b"")?;

    for endpoint in endpoints {
        socket.connect(endpoint)?;
    }

    Ok((context, socket))
}

pub(super) trait ReceiveFrom {
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

pub(super) fn recv_internal<R: ReceiveFrom>(
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

pub(super) fn subscribe_internal<F, B>(socket: Socket, callback: F) -> ControlFlow<B, Infallible>
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
