#!/bin/bash

runtime=$1;
chain="${runtime}-local"

# 1. Build ${runtime}-release which is added with "runtime-benchmarks" feature;
make "build-${runtime}-release-with-bench"
# 2. Filter the pallets of ${runtime} that should be executed benchmark;
target/release/bifrost benchmark --chain=$chain --list | sed -n '2,$p' | grep -Eio "^\w+" | uniq |
    while IFS= read -r line
    do
        pallet=$line;

        target/release/bifrost benchmark --chain=$chain \
        --steps=50 \
        --repeat=20 \
        --pallet=$pallet \
        --extrinsic="*" \
        --execution=wasm \
        --wasm-execution=compiled \
        --heap-pages=4096 \
        --header=./HEADER-GPL3 \
        --output="./runtime/${runtime}/src/weights/${pallet}.rs";
    done