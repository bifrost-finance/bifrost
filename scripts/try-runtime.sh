#!/bin/bash

chain_type=$1
chain_type="${chain_type:-bifrost}"
echo $chain_type

chain_url=$2
chain_url="${chain_url:-wss://bifrost-rpc.liebi.com:443/ws}"
echo $chain_url

block_hash=$3
block_hash="${block_hash:-0xccb8cff7e90bd9ecaa308e6d809bbe8c3241a51399ac063f405f11c16381da77}"

cargo run -p node-cli --locked --features with-$chain_type-runtime --features try-runtime -- try-runtime --chain="$chain_type-local" --wasm-execution=compiled on-runtime-upgrade live --uri=$chain_url --at=$block_hash