use super::{new_socket_internal, subscribe_internal};
use crate::{error::Result, message::Message};
use core::ops::ControlFlow;
use std::{
    sync::mpsc::{channel, Receiver},
    thread,
};
use zmq::Context;

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
