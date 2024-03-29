#!/usr/bin/env bash

## EXAMPLE
##
## Generate the weightInfo files of `bifrost-runtimes`;
# sh ./scripts/generate-weights.sh bifrost

# 1. Build all-release which is added with "runtime-benchmarks" feature;
make build-all-release-with-bench
# 2. Filter the pallets of ${runtime} that should be executed benchmark;
IFS=', ' read -r -a runtimes <<< $@;
for runtime in "${runtimes[@]}"
do
    chain="${runtime}-local"
    echo $chain
    target/release/bifrost benchmark pallet --chain=$chain --list | sed -n '2,$p' | grep -Eio "^\w+" | uniq |
        while IFS= read -r line
        do
		  	pallet=$line
		  	temp=${pallet/bifrost_/}
		  	pallet_dir=${temp//_/-}
			echo "benchmark pallet ${pallet}"
			echo "benchmark runtime ${runtime}"
			target/release/bifrost benchmark pallet --chain=$chain \
			--steps=50 \
			--repeat=20 \
			--pallet=$pallet \
			--extrinsic="*" \
			--execution=wasm \
			--wasm-execution=compiled \
			--heap-pages=4096 \
			--output="./pallets/${pallet_dir}/src/weights.rs" \
			--template="./weight-template/pallet-weight-template.hbs";
        done
done


IFS=', ' read -r -a runtimes <<< $@;
for runtime in "${runtimes[@]}"
do
    chain="${runtime}-local"
    echo $chain
    target/release/bifrost benchmark pallet --chain=$chain --list | sed -n '2,$p' | grep -Eio "^\w+" | uniq |
        while IFS= read -r line
        do
		  	pallet=$line
		  	temp=${pallet/bifrost_/}
		  	pallet_dir=${temp//_/-}
			echo "benchmark pallet ${pallet}"
			echo "benchmark runtime ${runtime}"
			target/release/bifrost benchmark pallet --chain=$chain \
			--steps=50 \
			--repeat=20 \
			--pallet=$pallet \
			--extrinsic="*" \
			--execution=wasm \
			--wasm-execution=compiled \
			--heap-pages=4096 \
			--output="./runtime/${runtime}/src/weights/${pallet}.rs" \
			--template="./weight-template/runtime-weight-template.hbs";
        done
done
