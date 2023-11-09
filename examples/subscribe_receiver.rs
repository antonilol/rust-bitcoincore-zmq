use bitcoincore_zmq::subscribe_receiver;

fn main() {
    let rx = subscribe_receiver(&["tcp://127.0.0.1:28332", "tcp://127.0.0.1:28333"]).unwrap();

    for msg in rx {
        match msg {
            Ok(msg) => println!("Received message: {msg}"),
            Err(err) => println!("Error receiving message: {err}"),
        }
    }
}
