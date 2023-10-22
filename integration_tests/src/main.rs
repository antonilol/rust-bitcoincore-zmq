mod endpoints;
mod util;

use bitcoincore_rpc::Client;
use bitcoincore_zmq::{subscribe_multi, subscribe_multi_async, subscribe_single_blocking, Message};
use core::{assert_eq, ops::ControlFlow};
use futures::{StreamExt, executor::block_on};
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
    }
}

fn test_hashblock(rpc: &Client) {
    let receiver = subscribe_multi(&[endpoints::HASHBLOCK, endpoints::RAWBLOCK])
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
    let receiver = subscribe_multi(&[endpoints::HASHTX, endpoints::RAWTX])
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

    thread::spawn(move || {
        subscribe_single_blocking(endpoints::HASHBLOCK, |msg| {
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

    assert_eq!(rpc_hash, zmq_hash);
}

fn test_hashblock_async(rpc: &Client) {
    let mut stream = subscribe_multi_async(&[endpoints::HASHBLOCK, endpoints::RAWBLOCK])
        .expect("failed to subscribe to Bitcoin Core's ZMQ subscriber");

    let rpc_hash = generate(rpc, 1).expect("rpc call failed").0[0];

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        block_on(async {
            while let Some(msg) = stream.next().await {
                tx.send(msg).unwrap();
            }
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
}
