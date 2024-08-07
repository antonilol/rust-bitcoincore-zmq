use super::{new_socket_internal, subscribe_internal};
use crate::{error::Result, message::Message};
use core::ops::ControlFlow;
use std::{
    sync::mpsc::{channel, Receiver},
    thread,
};

/// Subscribes to a single ZMQ endpoint and returns a [`Receiver`].
#[inline]
#[deprecated(
    since = "1.3.2",
    note = "Use subscribe_receiver. This function has no performance benefit over subscribe_multi anymore."
)]
pub fn subscribe_single(endpoint: &str) -> Result<Receiver<Result<Message>>> {
    subscribe_receiver(&[endpoint])
}

/// Subscribes to multiple ZMQ endpoints and returns a [`Receiver`].
#[inline]
#[deprecated(
    since = "1.3.2",
    note = "Use subscribe_receiver. The name changed because there is no distinction made anymore between subscribing to 1 or more endpoints."
)]
pub fn subscribe_multi(endpoints: &[&str]) -> Result<Receiver<Result<Message>>> {
    subscribe_receiver(endpoints)
}

/// Subscribes to multiple ZMQ endpoints and returns a [`Receiver`].
#[inline]
pub fn subscribe_receiver(endpoints: &[&str]) -> Result<Receiver<Result<Message>>> {
    let (tx, rx) = channel();

    let (_context, socket) = new_socket_internal(endpoints)?;

    thread::spawn(move || {
        subscribe_internal(socket, |msg| match tx.send(msg) {
            Err(_) => ControlFlow::Break(()),
            Ok(()) => ControlFlow::Continue(()),
        })
    });

    Ok(rx)
}
