mod util;

use bitcoincore_rpc::Client;
use bitcoincore_zmq::{sub_zmq, Message};
use util::{generate, recv_timeout_2, setup_rpc};

fn main() {
    let rpc = setup_rpc();

    test_hashblock(&rpc);
    test_hashtx(&rpc);
}

fn test_hashblock(rpc: &Client) {
    let receiver = sub_zmq(&["tcp://127.0.0.1:28370", "tcp://127.0.0.1:28372"])
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
    let receiver = sub_zmq(&["tcp://127.0.0.1:28371", "tcp://127.0.0.1:28373"])
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
