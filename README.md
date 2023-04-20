[![Build and test](https://github.com/antonilol/rust-bitcoincore-zmq/actions/workflows/build_and_test.yml/badge.svg)](https://github.com/antonilol/rust-bitcoincore-zmq/actions/workflows/build_and_test.yml)
[![Integration tests](https://github.com/antonilol/rust-bitcoincore-zmq/actions/workflows/integration_tests.yml/badge.svg)](https://github.com/antonilol/rust-bitcoincore-zmq/actions/workflows/integration_tests.yml)
[![crates.io](https://img.shields.io/crates/v/bitcoincore-zmq.svg)](https://crates.io/crates/bitcoincore-zmq)

# Rust Bitcoin Core ZMQ

### Usage example

```rust
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
