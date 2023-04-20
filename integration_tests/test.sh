#!/bin/bash

# exit on error
set -e

datadir=$(mktemp -d)

shopt -s expand_aliases
alias bitcoin-cli="bitcoin-cli -regtest -datadir=$datadir"

bitcoind --version

echo
echo "datadir=$datadir"
echo

bitcoind -daemonwait -regtest -datadir=$datadir -conf="$PWD"/bitcoin.conf

echo
echo "Setting up Bitcoin Core..."

bitcoin-cli createwallet default_wallet
bitcoin-cli -generate 200 > /dev/null && echo "generated 200 blocks"

echo
echo "Running tests"

BITCOIN_CORE_COOKIE_PATH="$datadir/regtest/.cookie" cargo run

echo
echo "Stopping Bitcoin Core"

bitcoin-cli stop
tail --pid=$(cat "$datadir"/regtest/bitcoind.pid) -f /dev/null
