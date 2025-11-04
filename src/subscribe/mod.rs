pub mod blocking;
pub mod receiver;
#[cfg(feature = "async")]
pub mod stream;

use crate::error::{Error, Result};
use crate::message::{Message, RawMessage, Topic};

use core::convert::Infallible;
use core::ops::ControlFlow;

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

pub(super) fn recv_internal<'a>(
    socket: &Socket,
    data_buf: &'a mut zmq::Message,
    tmp_buf: &mut zmq::Message,
) -> Result<RawMessage<&'a [u8]>> {
    socket.recv(tmp_buf, 0)?;
    let topic = Topic::try_from_bytes(tmp_buf.as_ref())?;

    if !socket.get_rcvmore()? {
        return Err(Error::InvalidMutlipartLength(1));
    }

    socket.recv(data_buf, 0)?;
    let data = data_buf.as_ref();

    if !socket.get_rcvmore()? {
        return Err(Error::InvalidMutlipartLength(2));
    }

    socket.recv(tmp_buf, 0)?;
    let sequence = u32::from_le_bytes(
        tmp_buf
            .as_ref()
            .try_into()
            .map_err(|_| Error::InvalidSequenceLength(tmp_buf.as_ref().len()))?,
    );

    if socket.get_rcvmore()? {
        socket.recv(tmp_buf, 0)?;

        let mut len = 4;
        while socket.get_rcvmore()? {
            socket.recv(tmp_buf, 0)?;
            len += 1;
        }

        return Err(Error::InvalidMutlipartLength(len));
    }

    Ok(RawMessage::from_parts(topic, data, sequence))
}

#[cfg(feature = "async")] // only used with the async feature on
pub(super) fn message_from_multipart_zmq_message(messages: &[zmq::Message]) -> Result<Message> {
    // zmq::Message doesn't implement AsRef<[u8]>, Deref is used here

    let [topic, data, sequence]: &[zmq::Message; 3] = messages
        .try_into()
        .map_err(|_| Error::InvalidMutlipartLength(messages.len()))?;

    Message::from_fixed_size_multipart::<&[u8]>(&[topic, data, sequence])
}

pub(super) fn subscribe_internal<F, B>(socket: Socket, callback: F) -> ControlFlow<B, Infallible>
where
    F: Fn(Result<Message>) -> ControlFlow<B>,
{
    let mut data_buf = zmq::Message::new();
    let mut tmp_buf = zmq::Message::new();

    loop {
        let msg = recv_internal(&socket, &mut data_buf, &mut tmp_buf)
            .and_then(Message::try_from_raw_message);

        callback(msg)?;
    }
}
