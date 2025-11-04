use super::{new_socket_internal, subscribe_internal};
use crate::{error::Result, message::Message};
use core::{convert::Infallible, ops::ControlFlow};

/// Subscribes to multiple ZMQ endpoints and blocks the thread until [`ControlFlow::Break`] is
/// returned by the callback.
#[inline]
pub fn subscribe_blocking<F, B>(
    endpoints: &[&str],
    callback: F,
) -> Result<ControlFlow<B, Infallible>>
where
    F: Fn(Result<Message>) -> ControlFlow<B>,
{
    let (_context, socket) = new_socket_internal(endpoints)?;

    Ok(subscribe_internal(socket, callback))
}
