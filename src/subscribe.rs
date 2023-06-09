use crate::{
    error::Result,
    message::{Message, SEQUENCE_LEN, TOPIC_MAX_LEN},
    Error, DATA_MAX_LEN,
};
use std::{
    sync::mpsc::{channel, Receiver},
    thread,
};
use zmq::{Context, Socket};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Continue,
    Stop,
}

impl Action {
    #[inline]
    pub fn stop_if(cond: bool) -> Self {
        if cond {
            Self::Stop
        } else {
            Self::Continue
        }
    }

    #[inline]
    pub fn continue_if(cond: bool) -> Self {
        Self::stop_if(!cond)
    }
}

impl From<()> for Action {
    fn from(_: ()) -> Self {
        Self::Continue
    }
}

/// Subscribes to a single ZMQ endpoint and returns a [`Receiver`]
#[inline]
pub fn subscribe_single(endpoint: &str) -> Result<Receiver<Result<Message>>> {
    let (tx, rx) = channel();
    let context = Context::new();

    let socket = new_socket_internal(&context, endpoint)?;

    thread::spawn(move || subscribe_internal(socket, |msg| Action::stop_if(tx.send(msg).is_err())));

    Ok(rx)
}

/// Subscribes to multiple ZMQ endpoints and returns a [`Receiver`]
#[inline]
pub fn subscribe_multi(endpoints: &[&str]) -> Result<Receiver<Result<Message>>> {
    let (tx, rx) = channel();
    let context = Context::new();

    for endpoint in endpoints {
        let tx = tx.clone();

        let socket = new_socket_internal(&context, endpoint)?;

        thread::spawn(move || {
            subscribe_internal(socket, |msg| Action::stop_if(tx.send(msg).is_err()))
        });
    }

    Ok(rx)
}

/// Subscribes to a single ZMQ endpoint and blocks the thread until [`Action::Stop`] is returned by the callback
#[inline]
pub fn subscribe_single_blocking<F: Fn(Result<Message>) -> T, T: Into<Action>>(
    endpoint: &str,
    callback: F,
) -> Result<()> {
    let context = Context::new();

    let socket = new_socket_internal(&context, endpoint)?;

    subscribe_internal(socket, callback);

    Ok(())
}

/// Subscribes to multiple ZMQ endpoints and blocks the thread until [`Action::Stop`] is returned by the callback
#[inline]
pub fn subscribe_multi_blocking<F: Fn(Result<Message>) -> T, T: Into<Action>>(
    endpoints: &[&str],
    callback: F,
) -> Result<()> {
    let (tx, rx) = channel();
    let context = Context::new();

    for endpoint in endpoints {
        let tx = tx.clone();

        let socket = new_socket_internal(&context, endpoint)?;

        thread::spawn(move || {
            subscribe_internal(socket, |msg| Action::stop_if(tx.send(msg).is_err()))
        });
    }

    for msg in rx {
        if callback(msg).into() == Action::Stop {
            break;
        }
    }

    Ok(())
}

#[inline]
fn new_socket_internal(context: &Context, endpoint: &str) -> Result<Socket> {
    let socket = context.socket(zmq::SUB)?;
    socket.connect(endpoint)?;
    socket.set_subscribe(b"")?;

    Ok(socket)
}

fn recv_internal(socket: &Socket, data: &mut [u8; DATA_MAX_LEN]) -> Result<Message> {
    let mut topic = [0u8; TOPIC_MAX_LEN];
    let mut sequence = [0u8; SEQUENCE_LEN];

    let topic_len = socket.recv_into(&mut topic, 0)?;
    if topic_len > TOPIC_MAX_LEN {
        return Err(Error::InvalidTopic(topic_len, topic));
    }

    if !socket.get_rcvmore()? {
        return Err(Error::InvalidMutlipartLength(1));
    }

    let data_len = socket.recv_into(data, 0)?;
    if data_len > DATA_MAX_LEN {
        return Err(Error::InvalidDataLength(data_len));
    }

    if !socket.get_rcvmore()? {
        return Err(Error::InvalidMutlipartLength(2));
    }

    let sequence_len = socket.recv_into(&mut sequence, 0)?;
    if sequence_len != SEQUENCE_LEN {
        return Err(Error::InvalidSequenceLength(sequence_len));
    }

    if !socket.get_rcvmore()? {
        return Message::from_parts(&topic[0..topic_len], &data[0..data_len], sequence);
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

#[inline]
fn subscribe_internal<F: Fn(Result<Message>) -> T, T: Into<Action>>(socket: Socket, callback: F) {
    let mut data: Box<[u8; DATA_MAX_LEN]> =
        vec![0; DATA_MAX_LEN].into_boxed_slice().try_into().unwrap();

    loop {
        let msg = recv_internal(&socket, &mut data);

        if callback(msg).into() == Action::Stop {
            break;
        }
    }
}
