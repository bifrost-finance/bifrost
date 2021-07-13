#!/bin/bash

# Runs all benchmarks for all pallets, for each of the runtimes specified below
# Should be run on a reference machine to gain accurate benchmarks(currently run on bifrost builder)

runtimes=(
  asgard
)

# cargo build --locked --release
for runtime in "${runtimes[@]}"; do
  cargo run --release --features="runtime-benchmarks,with-${runtime}-runtime" --locked benchmark --chain "${runtime}-dev" --execution=wasm --wasm-execution=compiled --pallet "*" --extrinsic "*" --repeat 0 | sed -r -e 's/Pallet: "([a-z_:]+)".*/\1/' | uniq | grep -v frame_system > "${runtime}_pallets"
  while read -r line; do
    pallet="$(echo "$line" | cut -d' ' -f1)";
    echo "Runtime: $runtime. Pallet: $pallet";
    cargo run --release --features="runtime-benchmarks,with-${runtime}-runtime" -- benchmark --chain="${runtime}-dev" --steps=50 --repeat=20 --pallet="$pallet" --extrinsic="*" --execution=wasm --wasm-execution=compiled --heap-pages=4096 --header=./HEADER-GPL3 --output="./runtime/${runtime}/src/weights/${pallet/::/_}.rs"
  done < "${runtime}_pallets"
  rm "${runtime}_pallets"
done