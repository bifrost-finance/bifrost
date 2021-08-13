#!/bin/bash

chain_type=$1
chain_type="${chain_type:-bifrost}"

chain_url=$2
chain_url="${chain_url:-ws://localhost:9944}"

block_hash=$3
block_hash="${block_hash:-0xd854e0d78b2bcd9cc56b6b4897f13d937d8ee166f1cda3b6c748ce775c56a87d}"

cargo run -p node-cli --locked --features with-$chain_type-runtime --features try-runtime -- try-runtime --chain="$chain_type-genesis" --wasm-execution=compiled --url="$chain_url" --block-at="$block_hash" on-runtime-upgrade live -s snapshot.bin