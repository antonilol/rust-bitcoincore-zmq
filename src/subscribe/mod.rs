pub mod blocking;
pub mod receiver;
#[cfg(feature = "async")]
pub mod stream;

use crate::{
    error::Result,
    message::{Message, SEQUENCE_LEN, TOPIC_MAX_LEN},
    Error, DATA_MAX_LEN,
};
use core::{convert::Infallible, ops::ControlFlow};
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

pub(super) fn recv_internal_socket(
    socket: &Socket,
    tmp_buffer: &mut [u8; DATA_MAX_LEN],
) -> Result<Message> {
    let mut topic = [0u8; TOPIC_MAX_LEN];
    let mut sequence = [0u8; SEQUENCE_LEN];

    let topic_len = socket.recv_into(&mut topic, 0)?;
    let topic = topic
        .get(0..topic_len)
        .ok_or(Error::InvalidTopic(topic_len, topic))?;

    if !socket.get_rcvmore()? {
        return Err(Error::InvalidMutlipartLength(1));
    }

    let data_len = socket.recv_into(tmp_buffer, 0)?;
    let data = tmp_buffer
        .get(0..data_len)
        .ok_or(Error::InvalidDataLength(data_len))?;

    if !socket.get_rcvmore()? {
        return Err(Error::InvalidMutlipartLength(2));
    }

    let sequence_len = socket.recv_into(&mut sequence, 0)?;
    if sequence_len != SEQUENCE_LEN {
        return Err(Error::InvalidSequenceLength(sequence_len));
    }

    if !socket.get_rcvmore()? {
        return Message::from_parts(topic, data, sequence);
    }

    let mut len = 3;

    loop {
        socket.recv_into(&mut [], 0)?;

        len += 1;

        if !socket.get_rcvmore()? {
            return Err(Error::InvalidMutlipartLength(len));
        }
    }
}

#[cfg(feature = "async")] // only used with the async feature on
pub(super) fn message_from_multipart_zmq_message(messages: &[zmq::Message]) -> Result<Message> {
    // zmq::Message doesn't implement AsRef<[u8]>

    let [topic, data, sequence]: &[zmq::Message; 3] = messages
        .try_into()
        .map_err(|_| Error::InvalidMutlipartLength(messages.len()))?;

    Message::from_fixed_size_multipart::<&[u8]>(&[topic, data, sequence])
}

pub(super) fn subscribe_internal<F, B>(socket: Socket, callback: F) -> ControlFlow<B, Infallible>
where
    F: Fn(Result<Message>) -> ControlFlow<B>,
{
    let mut buf: Box<[u8; DATA_MAX_LEN]> =
        vec![0; DATA_MAX_LEN].into_boxed_slice().try_into().unwrap();

    loop {
        let msg = recv_internal_socket(&socket, &mut buf);

        callback(msg)?;
    }
}
