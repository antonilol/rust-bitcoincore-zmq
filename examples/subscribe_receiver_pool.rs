use bitcoincore_zmq::subscribe_multi;
use std::{
    sync::{Arc, Mutex},
    thread,
};

/// Use 4 threads to handle messages
const POOL_THREADS: usize = 4;

fn main() {
    let mut threads = Vec::new();

    let rx = Arc::new(Mutex::new(
        subscribe_multi(&["tcp://127.0.0.1:28332", "tcp://127.0.0.1:28333"]).unwrap(),
    ));

    for id in 0..POOL_THREADS {
        let rx = rx.clone();

        threads.push(thread::spawn(move || {
            while let Ok(msg) = { rx.lock().unwrap().recv() } {
                match msg {
                    Ok(msg) => println!("Thread {id}: Received message: {msg}"),
                    Err(err) => println!("Thread {id}: Error receiving message: {err}"),
                }
            }
        }));
    }

    for t in threads {
        t.join().unwrap();
    }
}
