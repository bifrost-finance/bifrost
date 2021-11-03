#!/bin/bash

chain_type=$1
chain_type="${chain_type:-bifrost}"
echo $chain_type

chain_url=$2
chain_url="${chain_url:-ws://localhost:18848}"
echo $chain_url

block_hash=$3
block_hash="${block_hash:-0x588bc30598fddd228f234b387cf68daa551262a2a17a5dda1199e770aeaffd77}"

cargo run -p node-cli --locked --features with-$chain_type-runtime --features try-runtime -- try-runtime --chain="$chain_type-local" --wasm-execution=compiled on-runtime-upgrade live --uri=$chain_url --at=$block_hash