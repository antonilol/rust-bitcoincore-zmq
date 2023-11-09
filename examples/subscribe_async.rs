use bitcoincore_zmq::subscribe_async;
use futures::executor::block_on;
use futures_util::StreamExt;

fn main() {
    let mut stream = subscribe_async(&["tcp://127.0.0.1:28332"]).unwrap();

    // This is a small example to demonstrate subscribe_single_async, it is okay here to use
    // block_on, but not in production environments as this defeats the purpose of async.
    block_on(async {
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(msg) => println!("Received message: {msg}"),
                Err(err) => println!("Error receiving message: {err}"),
            }
        }
    });
}
