use std::env;
use std::fmt;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;

use bitcoin::{Address, BlockHash};
use bitcoincore_rpc::{Auth, Client, RpcApi};

pub const RECV_TIMEOUT: Duration = Duration::from_secs(10);

pub fn setup_rpc() -> Client {
    Client::new(
        "http://localhost:18443",
        Auth::CookieFile(get_cookie_path().into()),
    )
    .expect("unable to connect to Bitcoin Core regtest RPC")
}

pub fn static_ref_heap<T>(val: T) -> &'static T {
    Box::leak(Box::new(val))
}

fn get_cookie_path() -> String {
    const ENV_VAR: &str = "BITCOIN_CORE_COOKIE_PATH";

    match env::var(ENV_VAR) {
        Ok(value) => value,
        Err(err) => {
            if matches!(err, env::VarError::NotPresent) {
                eprintln!("Environment variable \"{ENV_VAR}\" not set.");
                eprintln!(
                    "Make sure to run this with the \"test.sh\" \
                     script to set Bitcoin Core up correctly."
                );
            }

            panic!("Failed to read environment variable \"{ENV_VAR}\": {err}");
        }
    }
}

pub fn generate(
    rpc: &Client,
    block_num: u64,
) -> Result<(Vec<BlockHash>, Address), bitcoincore_rpc::Error> {
    let addr = rpc.get_new_address(None, None)?.assume_checked();
    Ok((rpc.generate_to_address(block_num, &addr)?, addr))
}

pub fn recv_timeout<T>(rx: &Receiver<Result<T, impl fmt::Debug>>) -> T {
    rx.recv_timeout(RECV_TIMEOUT)
        .expect("receiving failed")
        .expect("zmq message error")
}

pub fn recv_timeout_2<T>(rx: &Receiver<Result<T, impl fmt::Debug>>) -> (T, T) {
    (recv_timeout(rx), recv_timeout(rx))
}

pub fn sleep(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}
