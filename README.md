[![Build and test](https://github.com/antonilol/rust-bitcoincore-zmq/actions/workflows/build_and_test.yml/badge.svg)](https://github.com/antonilol/rust-bitcoincore-zmq/actions/workflows/build_and_test.yml)
[![Integration tests](https://github.com/antonilol/rust-bitcoincore-zmq/actions/workflows/integration_tests.yml/badge.svg)](https://github.com/antonilol/rust-bitcoincore-zmq/actions/workflows/integration_tests.yml)
[![crates.io](https://img.shields.io/crates/v/bitcoincore-zmq.svg)](https://crates.io/crates/bitcoincore-zmq)
[![docs.rs](https://docs.rs/bitcoincore-zmq/badge.svg)](https://docs.rs/bitcoincore-zmq)

# Rust Bitcoin Core ZMQ Subscriber

### Usage example

```rust
fn main() {
    for msg in bitcoincore_zmq::subscribe_receiver(&["tcp://127.0.0.1:28359"]).unwrap() {
        match msg {
            Ok(msg) => println!("Received message: {msg}"),
            Err(err) => println!("Error receiving message: {err}"),
        }
    }
}
```

For more examples, have a look in the [examples directory](examples).

### Features

- Minimal dependencies: the 2 crates `bitcoin` and `zmq`, optionally 2 additional crates are needed for the async subscriber, `async_zmq` and `futures-util`.
- Handles all message types from Bitcoin Core: `hashblock`, `hashtx`, `block`, `tx` and `sequence`.
- Flexible: choose between blocking functions with a callback, reading from a [Receiver](https://doc.rust-lang.org/std/sync/mpsc/struct.Receiver.html) or reading from an asynchronous [Stream](https://docs.rs/futures-core/latest/futures_core/stream/trait.Stream.html) without locking to a specific async runtime.

### Testing

Tests run on every push and pull request.
Integration tests use the latest version of the 5 most recent major Bitcoin Core versions, see [integration_tests.yml](.github/workflows/integration_tests.yml#L19-L23).

### Useful resources

- [Bitcoin Core ZMQ documentation](https://github.com/bitcoin/bitcoin/blob/master/doc/zmq.md)

---

TODO:
- This README
- SequenceMessage itest
- Easy addEventListener like functionality with help of the `getzmqnotifications` rpc (bitcoincore-rpc PR: [#295](https://github.com/rust-bitcoin/rust-bitcoincore-rpc/pull/295))
- raw messages
- zmq publisher
- include source in message
