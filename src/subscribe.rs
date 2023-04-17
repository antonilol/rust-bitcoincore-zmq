use crate::{error::Result, message::Message};
use std::{
    sync::mpsc::{channel, Receiver},
    thread,
};
use zmq::Context;

#[inline]
pub fn sub_zmq(endpoints: &[&str]) -> Result<Receiver<Result<Message>>> {
    let (tx, rx) = channel();
    let context = Context::new();

    for endpoint in endpoints {
        let tx = tx.clone();

        let subscriber = context.socket(zmq::SUB)?;
        subscriber.connect(endpoint)?;
        subscriber.set_subscribe(b"")?;

        thread::spawn(move || loop {
            let msg = subscriber
                .recv_multipart(0)
                .map_err(|err| err.into())
                .and_then(|mp| mp.try_into());
            if tx.send(msg).is_err() {
                break;
            }
        });
    }

    Ok(rx)
}
