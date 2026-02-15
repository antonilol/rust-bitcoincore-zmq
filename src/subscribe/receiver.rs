use super::{new_socket_internal, subscribe_internal};
use crate::{error::Result, message::Message};

use core::ops::ControlFlow;
use std::sync::mpsc::{channel, Receiver};
use std::thread;

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
