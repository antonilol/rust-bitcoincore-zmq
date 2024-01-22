use bitcoincore_zmq::{subscribe_async_wait_handshake, SocketEvent, SocketMessage};
use core::time::Duration;
use futures_util::StreamExt;
use tokio::time::timeout;

#[tokio::main]
async fn main() {
    // In this example I use match instead of unwrap to clearly show where errors are produced.
    // `timeout` here returns an `impl Future<Output = Result<Result<impl Stream ...>>>`. The outer
    // Result is created by tokio's timeout function, and wraps the inner Result created by the
    // subscribe function.
    let mut stream = match timeout(
        Duration::from_millis(2000),
        subscribe_async_wait_handshake(&["tcp://127.0.0.1:28332"]),
    )
    .await
    {
        Ok(Ok(stream)) => {
            // Ok(Ok(_)), ok from both functions.
            stream
        }
        Ok(Err(err)) => {
            // Ok(Err(_)), ok from `timeout` but an error from the subscribe function.
            panic!("subscribe error: {err}");
        }
        Err(_) => {
            // Err(_), err from `timeout` means that it timed out.
            panic!("subscribe_async_wait_handshake timed out");
        }
    };

    // Like in other examples, we have a stream we can get messages from, this one also produces
    // events, for a detailed description on the events, see
    // https://linux.die.net/man/3/zmq_socket_monitor.
    while let Some(msg) = stream.next().await {
        match msg {
            Ok(SocketMessage::Message(msg)) => println!("Received message: {msg}"),
            Ok(SocketMessage::Event(event)) => {
                println!("Received socket event: {event:?}");
                match event.event {
                    SocketEvent::Disconnected { .. } => {
                        println!(
                            "disconnected from {}, ZMQ automatically tries to reconnect",
                            event.source_url
                        );
                    }
                    SocketEvent::HandshakeSucceeded => {
                        // We can say "reconnected" because subscribe_async_wait_handshake waits on
                        // the first connections of each endpoint before returning.
                        println!("reconnected to {}", event.source_url);
                    }
                    _ => {
                        // ignore other events
                    }
                }
            }
            Err(err) => println!("Error receiving message: {err}"),
        }
    }
}
