use bitcoin::{Address, BlockHash};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use core::fmt::Debug;
use std::{env, sync::mpsc::Receiver, time::Duration};

pub const RECV_TIMEOUT: Duration = Duration::from_secs(10);

pub fn setup_rpc() -> Client {
    Client::new(
        "http://localhost:18443",
        Auth::CookieFile(get_cookie_path().into()),
    )
    .expect("unable to connect to Bitcoin Core regtest RPC")
}

fn get_cookie_path() -> String {
    env::var("BITCOIN_CORE_COOKIE_PATH").expect(
        "env var BITCOIN_CORE_COOKIE_PATH probably not set,\
         make sure to run this with the 'test.sh' script to set Bitcoin Core up correctly",
    )
}

pub fn generate(
    rpc: &Client,
    block_num: u64,
) -> Result<(Vec<BlockHash>, Address), bitcoincore_rpc::Error> {
    let addr = rpc.get_new_address(None, None)?.assume_checked();
    Ok((rpc.generate_to_address(block_num, &addr)?, addr))
}

pub fn recv_timeout<T>(rx: &Receiver<Result<T, impl Debug>>) -> T {
    rx.recv_timeout(RECV_TIMEOUT)
        .expect("receiving failed")
        .expect("zmq message error")
}

pub fn recv_timeout_2<T>(rx: &Receiver<Result<T, impl Debug>>) -> (T, T) {
    (recv_timeout(rx), recv_timeout(rx))
}
