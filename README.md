# Rust Bitcoin Core ZMQ

### Usage example

```rs
fn main() {
    for msg in bitcoincore_zmq::sub_zmq(&["tcp://127.0.0.1:28359"]).unwrap() {
        match msg {
            Ok(msg) => println!("Received message: {msg}"),
            Err(err) => println!("Error receiving message: {err}"),
        }
    }
}
```

---

TODO:
- This README
- Message test
- SequenceMessage itest
