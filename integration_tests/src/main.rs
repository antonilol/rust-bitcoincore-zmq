mod endpoints;
mod util;

use self::util::{generate, recv_timeout_2, setup_rpc, sleep, static_ref_heap, RECV_TIMEOUT};

use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use bitcoincore_rpc::Client;
use bitcoincore_zmq::{
    subscribe_async, subscribe_async_monitor, subscribe_async_wait_handshake,
    subscribe_async_wait_handshake_timeout, subscribe_blocking, subscribe_receiver, Message,
    MessageContent, MonitorMessage, SocketEvent, SocketMessage,
};
use futures::executor::block_on;
use futures::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime;
use tokio::sync::mpsc::unbounded_channel;

macro_rules! test {
    ($($function:ident,)*) => {
        let rpc = static_ref_heap(setup_rpc());
        $(
            println!(concat!("Running ", stringify!($function), "..."));
            $function(rpc);
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
        test_subscribe_timeout_tokio,
        test_subscribe_timeout_inefficient,
        test_disconnect,
    }
}

fn test_hashblock(rpc: &Client) {
    let receiver = subscribe_receiver(&[endpoints::HASHBLOCK, endpoints::RAWBLOCK])
        .expect("failed to subscribe to Bitcoin Core's ZMQ publisher");

    let rpc_hash = generate(rpc, 1).expect("rpc call failed").0[0];

    let (a, b) = recv_timeout_2(&receiver);
    match (&a.content, &b.content) {
        (MessageContent::Block(block), MessageContent::BlockHash(blockhash))
        | (MessageContent::BlockHash(blockhash), MessageContent::Block(block)) => {
            assert_eq!(rpc_hash, block.block_hash());
            assert_eq!(rpc_hash, *blockhash);
        }
        _ => {
            panic!("invalid messages received: ({a}, {b})");
        }
    }
}

fn test_hashtx(rpc: &Client) {
    let receiver = subscribe_receiver(&[endpoints::HASHTX, endpoints::RAWTX])
        .expect("failed to subscribe to Bitcoin Core's ZMQ publisher");

    generate(rpc, 1).expect("rpc call failed");

    let (a, b) = recv_timeout_2(&receiver);
    match (&a.content, &b.content) {
        (MessageContent::Tx(tx), MessageContent::Txid(txid))
        | (MessageContent::Txid(txid), MessageContent::Tx(tx)) => {
            assert_eq!(tx.compute_txid(), *txid);
        }
        _ => {
            panic!("invalid messages received: ({a}, {b})");
        }
    }
}

fn test_sub_blocking(rpc: &Client) {
    sleep(1000);

    let (tx, rx) = mpsc::channel();

    let h = thread::spawn(move || {
        let ControlFlow::Break(()) = subscribe_blocking(&[endpoints::HASHBLOCK], |msg| {
            let msg = msg.expect("zmq message error");

            match &msg.content {
                MessageContent::BlockHash(hash) => {
                    tx.send(*hash).unwrap();
                }
                _ => {
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
        });
    });

    let (a, b) = recv_timeout_2(&rx);
    match (&a.content, &b.content) {
        (MessageContent::Block(block), MessageContent::BlockHash(blockhash))
        | (MessageContent::BlockHash(blockhash), MessageContent::Block(block)) => {
            assert_eq!(rpc_hash, block.block_hash());
            assert_eq!(rpc_hash, *blockhash);
        }
        _ => {
            panic!("invalid messages received: ({a}, {b})");
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
                SocketMessage::Message(_msg) => {
                    break;
                }
                SocketMessage::Event(MonitorMessage { event, .. }) => {
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

fn test_subscribe_timeout_tokio(_rpc: &Client) {
    const TIMEOUT: Duration = Duration::from_millis(500);

    runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let _ = tokio::time::timeout(
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

fn test_subscribe_timeout_inefficient(_rpc: &Client) {
    const TIMEOUT: Duration = Duration::from_millis(500);

    block_on(async {
        let _ = subscribe_async_wait_handshake_timeout(&[endpoints::HASHBLOCK], TIMEOUT)
            .await
            .unwrap()
            .unwrap();

        subscribe_async_wait_handshake_timeout(&["tcp://localhost:18443"], TIMEOUT)
            .await
            .map(|_| ())
            .expect_err("an http server will not make a zmtp handshake");

        subscribe_async_wait_handshake_timeout(
            &[endpoints::HASHBLOCK, "tcp://localhost:18443"],
            TIMEOUT,
        )
        .await
        .map(|_| ())
        .expect_err("an http server will not make a zmtp handshake");
    });
}

fn test_disconnect(rpc: &'static Client) {
    runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (tx, mut rx) = unbounded_channel();

            let h = tokio::spawn(async move {
                let mut stream = tokio::time::timeout(
                    Duration::from_millis(2000),
                    subscribe_async_wait_handshake(&["tcp://127.0.0.1:29999"]),
                )
                .await
                .unwrap()
                .unwrap();

                tokio::time::sleep(Duration::from_millis(500)).await;

                let rpc_hash = generate(rpc, 1).expect("rpc call failed").0[0];

                loop {
                    match stream.next().await {
                        Some(Ok(SocketMessage::Message(Message {
                            content: MessageContent::BlockHash(zmq_hash),
                            ..
                        }))) if rpc_hash == zmq_hash => {
                            break;
                        }
                        Some(Ok(SocketMessage::Event(_))) => {
                            // ignore events
                        }
                        other => panic!("unexpected response: {other:?}"),
                    }
                }

                // send the signal to close the proxy
                tx.send(()).unwrap();

                loop {
                    match stream.next().await {
                        Some(Ok(SocketMessage::Event(MonitorMessage {
                            event: SocketEvent::Disconnected { .. },
                            source_url,
                        }))) if source_url == "tcp://127.0.0.1:29999" => {
                            break;
                        }
                        Some(Ok(SocketMessage::Event(_))) => {
                            // ignore other events
                        }
                        other => panic!("unexpected response: {other:?}"),
                    }
                }
            });

            // proxy endpoints::HASHBLOCK to 127.0.0.1:29999 to simulate a disconnect
            // stopping bitcoin core is not a good idea as other tests may follow this one
            // taken from https://github.com/tokio-rs/tokio/discussions/3173, it is not perfect but ok for this test
            let ss = TcpListener::bind("127.0.0.1:29999".parse::<SocketAddr>().unwrap())
                .await
                .unwrap();
            let (cs, _) = ss.accept().await.unwrap();
            // [6..] splits off "tcp://"
            let g = TcpStream::connect(endpoints::HASHBLOCK[6..].parse::<SocketAddr>().unwrap())
                .await
                .unwrap();
            let (mut gr, mut gw) = g.into_split();
            let (mut csr, mut csw) = cs.into_split();
            let h1 = tokio::spawn(async move {
                let _ = tokio::io::copy(&mut gr, &mut csw).await;
                let _ = csw.shutdown().await;
            });
            let h2 = tokio::spawn(async move {
                let _ = tokio::io::copy(&mut csr, &mut gw).await;
                let _ = gw.shutdown().await;
            });

            // wait for the signal
            rx.recv().await.unwrap();

            // close the proxy
            h1.abort();
            h2.abort();

            // wait on other spawned tasks
            h.await.unwrap();
        });
}
