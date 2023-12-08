mod endpoints;
mod util;

use bitcoincore_rpc::Client;
use bitcoincore_zmq::{
    subscribe_async, subscribe_async_monitor, subscribe_async_wait_handshake, subscribe_blocking,
    subscribe_receiver, Message, MonitorMessage, SocketEvent, SocketMessage,
};
use core::{assert_eq, ops::ControlFlow, time::Duration};
use futures::{executor::block_on, StreamExt};
use std::{sync::mpsc, thread};
use util::{generate, recv_timeout_2, setup_rpc, sleep, RECV_TIMEOUT};

macro_rules! test {
    ($($function:ident,)*) => {
        let rpc = setup_rpc();
        $(
            println!(concat!("Running ", stringify!($function), "..."));
            $function(&rpc);
            println!("ok");
        )*
    };
}

fn main() {
    test! {
        test_hashblock,
        test_hashtx,
        test_sub_blocking,
        test_hashblock_async,
        test_monitor,
        test_subscribe_timeout,
    }
}

fn test_hashblock(rpc: &Client) {
    let receiver = subscribe_receiver(&[endpoints::HASHBLOCK, endpoints::RAWBLOCK])
        .expect("failed to subscribe to Bitcoin Core's ZMQ publisher");

    let rpc_hash = generate(rpc, 1).expect("rpc call failed").0[0];

    match recv_timeout_2(&receiver) {
        (Message::Block(block, _), Message::HashBlock(blockhash, _))
        | (Message::HashBlock(blockhash, _), Message::Block(block, _)) => {
            assert_eq!(rpc_hash, block.block_hash());
            assert_eq!(rpc_hash, blockhash);
        }
        (msg1, msg2) => {
            panic!("invalid messages received: ({msg1}, {msg2})");
        }
    }
}

fn test_hashtx(rpc: &Client) {
    let receiver = subscribe_receiver(&[endpoints::HASHTX, endpoints::RAWTX])
        .expect("failed to subscribe to Bitcoin Core's ZMQ publisher");

    generate(rpc, 1).expect("rpc call failed");

    match recv_timeout_2(&receiver) {
        (Message::Tx(tx, _), Message::HashTx(txid, _))
        | (Message::HashTx(txid, _), Message::Tx(tx, _)) => {
            assert_eq!(tx.txid(), txid);
        }
        (msg1, msg2) => {
            panic!("invalid messages received: ({msg1}, {msg2})");
        }
    }
}

fn test_sub_blocking(rpc: &Client) {
    sleep(1000);

    let (tx, rx) = mpsc::channel();

    let h = thread::spawn(move || {
        subscribe_blocking(&[endpoints::HASHBLOCK], |msg| {
            let msg = msg.expect("zmq message error");

            match msg {
                Message::HashBlock(hash, _) => {
                    tx.send(hash).unwrap();
                }
                msg => {
                    panic!("invalid message received: {msg}");
                }
            }

            // Stop after 1 message
            ControlFlow::Break(())
        })
        .expect("failed to subscribe to Bitcoin Core's ZMQ publisher");
    });

    sleep(1000);

    let rpc_hash = generate(rpc, 1).expect("rpc call failed").0[0];

    let zmq_hash = rx.recv_timeout(RECV_TIMEOUT).unwrap();

    h.join().unwrap();

    assert_eq!(rpc_hash, zmq_hash);
}

fn test_hashblock_async(rpc: &Client) {
    let mut stream = subscribe_async(&[endpoints::HASHBLOCK, endpoints::RAWBLOCK])
        .expect("failed to subscribe to Bitcoin Core's ZMQ subscriber");

    let rpc_hash = generate(rpc, 1).expect("rpc call failed").0[0];

    let (tx, rx) = mpsc::channel();

    let h = thread::spawn(move || {
        block_on(async {
            tx.send(stream.next().await.unwrap()).unwrap();
            tx.send(stream.next().await.unwrap()).unwrap();
        })
    });

    match recv_timeout_2(&rx) {
        (Message::Block(block, _), Message::HashBlock(blockhash, _))
        | (Message::HashBlock(blockhash, _), Message::Block(block, _)) => {
            assert_eq!(rpc_hash, block.block_hash());
            assert_eq!(rpc_hash, blockhash);
        }
        (msg1, msg2) => {
            panic!("invalid messages received: ({msg1}, {msg2})");
        }
    }

    h.join().unwrap();
}

fn test_monitor(rpc: &Client) {
    let mut stream = subscribe_async_monitor(&[endpoints::HASHBLOCK])
        .expect("failed to subscribe to Bitcoin Core's ZMQ publisher");

    block_on(async {
        while let Some(msg) = stream.next().await {
            let msg = msg.unwrap();
            match msg {
                SocketMessage::Message(msg) => {
                    // TODO remove debug printlns before merging
                    println!("received real message: {msg}");
                    break;
                }
                SocketMessage::Event(MonitorMessage { event, source_url }) => {
                    // TODO remove debug printlns before merging
                    println!("received monitor message: {event:?} from {source_url}");
                    if event == SocketEvent::HandshakeSucceeded {
                        // there is a zmq publisher on the other side!
                        // generate a block to generate a message
                        generate(rpc, 1).expect("rpc call failed");
                    }
                }
            }
        }
    });
}

fn test_subscribe_timeout(_rpc: &Client) {
    const TIMEOUT: Duration = Duration::from_millis(2000);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            tokio::time::timeout(
                TIMEOUT,
                subscribe_async_wait_handshake(&[endpoints::HASHBLOCK]),
            )
            .await
            .unwrap()
            .unwrap();

            tokio::time::timeout(
                TIMEOUT,
                subscribe_async_wait_handshake(&["tcp://localhost:18443"]),
            )
            .await
            .map(|_| ())
            .expect_err("an http server will not make a zmtp handshake");

            tokio::time::timeout(
                TIMEOUT,
                subscribe_async_wait_handshake(&[endpoints::HASHBLOCK, "tcp://localhost:18443"]),
            )
            .await
            .map(|_| ())
            .expect_err("an http server will not make a zmtp handshake");
        });
}
