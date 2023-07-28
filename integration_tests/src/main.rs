mod endpoints;
mod util;

use bitcoincore_rpc::Client;
use bitcoincore_zmq::{subscribe_multi, subscribe_single_blocking, Message};
use std::{assert_eq, ops::ControlFlow, sync::mpsc, thread};
use util::{generate, recv_timeout_2, setup_rpc, sleep, RECV_TIMEOUT};

fn main() {
    let rpc = setup_rpc();

    test_hashblock(&rpc);
    test_hashtx(&rpc);
    test_sub_blocking(&rpc);
}

fn test_hashblock(rpc: &Client) {
    let receiver = subscribe_multi(&[endpoints::HASHBLOCK, endpoints::RAWBLOCK])
        .expect("failed to subscribe to Bitcoin Core's ZMQ subscriber");

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
        .expect("failed to subscribe to Bitcoin Core's ZMQ subscriber");

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
        .expect("failed to subscribe to Bitcoin Core's ZMQ subscriber");
    });

    sleep(1000);

    let rpc_hash = generate(rpc, 1).expect("rpc call failed").0[0];

    let zmq_hash = rx.recv_timeout(RECV_TIMEOUT).unwrap();

    assert_eq!(rpc_hash, zmq_hash);
}
