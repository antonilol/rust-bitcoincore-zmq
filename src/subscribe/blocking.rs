use super::{new_socket_internal, subscribe_internal};
use crate::{error::Result, message::Message};
use core::{convert::Infallible, ops::ControlFlow};
use std::{sync::mpsc::channel, thread};
use zmq::Context;

fn break_on_err(is_err: bool) -> ControlFlow<()> {
    if is_err {
        ControlFlow::Break(())
    } else {
        ControlFlow::Continue(())
    }
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
