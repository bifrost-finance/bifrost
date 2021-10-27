#!/bin/bash

## EXAMPLE
##
## Generate the weightInfo files of `asgard-runtimes`;
# sh ./script/generate-weights.sh asgard
## Generate the weightInfo files of `bifrost-runtimes`;
# sh ./script/generate-weights.sh bifrost
## Generate the weightInfo files of `asgard-runtimes` and that of `asgard-runtime`;
# sh ./script/generate-weights.sh asgard bifrost

# 1. Build all-release which is added with "runtime-benchmarks" feature;
# make "build-all-release-with-bench"
# 2. Filter the pallets of ${runtime} that should be executed benchmark;
IFS=', ' read -r -a runtimes <<< $@;
for runtime in "${runtimes[@]}"
do
    chain="${runtime}-local"
    echo $chain
    # target/release/bifrost benchmark --chain=$chain --list | sed -n '2,$p' | grep -Eio "^\w+" | uniq |
    #     while IFS= read -r line
    #     do
    #         pallet=$line;

    #         target/release/bifrost benchmark --chain=$chain \
    #         --steps=50 \
    #         --repeat=20 \
    #         --pallet=$pallet \
    #         --extrinsic="*" \
    #         --execution=wasm \
    #         --wasm-execution=compiled \
    #         --heap-pages=4096 \
    #         --header=./HEADER-GPL3 \
    #         --output="./runtime/${runtime}/src/weights/${pallet}.rs";
    #     done
done