use super::{new_socket_internal, subscribe_internal};
use crate::{error::Result, message::Message};
use core::{convert::Infallible, ops::ControlFlow};

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
    subscribe_multi_blocking(&[endpoint], callback)
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
    let (_context, socket) = new_socket_internal(endpoints)?;

    Ok(subscribe_internal(socket, callback))
}
