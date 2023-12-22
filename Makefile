.PHONY: all
all:help

PARA_ID        := 2001
DOCKER_TAG     := latest
CHAIN		   := bifrost-genesis
SURI           := //Alice

.PHONY: init # init config
init:
	git config core.hooksPath .githooks
	./scripts/init.sh

# Build Release

.PHONY: build-bifrost-kusama-release # build bifrost kusama release
build-bifrost-kusama-release: copy-genesis-config-release
	cargo build -p bifrost-cli --locked --features "with-bifrost-kusama-runtime" --release

.PHONY: build-bifrost-polkadot-release # build bifrost polkadot release
build-bifrost-polkadot-release:
	cargo build -p bifrost-cli --locked --features "with-bifrost-polkadot-runtime" --release

.PHONY: build-all-release # build all runtime release
build-all-release: copy-genesis-config-release
	cargo build -p bifrost-cli --locked --features "with-all-runtime" --release

.PHONY: check-all # cargo check all runtime
check-all: format
	SKIP_WASM_BUILD= cargo check -p bifrost-cli --locked --features "with-all-runtime,runtime-benchmarks,try-runtime"

.PHONY: test-all # cargo test all
test-all: test-runtimes test-benchmarks


.PHONY: test-runtimes
test-runtimes:
	cargo test --features "with-all-runtime" --lib

.PHONY: test-benchmarks
test-benchmarks:
	cargo test --all benchmarking  --features="runtime-benchmarks, polkadot" --exclude "*integration*" --exclude "bifrost-cli" --exclude "bifrost-rpc" --exclude "bifrost-service" --exclude "*-runtime"

.PHONY: integration-test # integration test
integration-test:
	cargo test  -p *-integration-tests

.PHONY: kusama-integration-test # integration test
kusama-integration-test:
	cargo test -p bifrost-kusama-integration-tests

.PHONY: polkadot-integration-test # integration test
polkadot-integration-test:
	cargo test -p bifrost-polkadot-integration-tests

.PHONY: clean # cargo clean
clean:
	cargo clean

.PHONY: copy-genesis-config-release # copy genesis config to release directory
copy-genesis-config-release:
	mkdir -p "target/release/res"
	cp -r node/service/res/genesis_config target/release/res

.PHONY: format # cargo fmt
format:
	cargo fmt --all -- --check

.PHONY: clippy # cargo clippy
clippy:
	SKIP_WASM_BUILD= cargo clippy --all --all-targets --features=with-all-runtime -- -D warnings

.PHONY: benchmarking-staking # benchmarking staking pallet
benchmarking-staking:
	cargo run -p bifrost-cli --locked --features "with-bifrost-kusama-runtime,runtime-benchmarks" --release \
			-- benchmark --chain=bifrost-local --steps=50 \
			--repeat=20 \
            --pallet=bifrost_parachain_staking \
            --extrinsic="*" \
            --execution=wasm \
            --wasm-execution=compiled \
            --heap-pages=4096 \
            --header=./HEADER-GPL3 \
			--output="./runtime/bifrost-kusama/src/weights/bifrost_parachain_staking.rs"

.PHONY: generate-bifrost-kusama-weights # generate bifrost-kusama weights
generate-bifrost-kusama-weights:
	bash ./scripts/generate-weights.sh bifrost-kusama

.PHONY: generate-bifrost-polkadot-weights # generate bifrost-polkadot weights
generate-bifrost-polkadot-weights:
	bash ./scripts/generate-weights.sh bifrost-polkadot

.PHONY: generate-all-weights # generate all weights
generate-all-weights: generate-bifrost-kusama-weights generate-bifrost-polkadot-weights

.PHONY: build-all-release-with-bench # build all release with benchmarking
build-all-release-with-bench: copy-genesis-config-release
	cargo build -p bifrost-cli --locked --features "with-all-runtime,runtime-benchmarks" --release

# Build docker image
.PHONY: build-docker-image # build docker image
build-docker-image:
	.maintain/build-image.sh

# Build wasm
.PHONY: build-bifrost-kusama-wasm # build bifrost kusama wasm
build-bifrost-kusama-wasm:
	.maintain/build-wasm.sh bifrost-kusama

.PHONY: build-bifrost-polkadot-wasm # build bifrost polkadot wasm
build-bifrost-polkadot-wasm:
	.maintain/build-wasm.sh bifrost-polkadot

.PHONY: build-bifrost-rococo-fast-wasm # build bifrost rococo fast wasm
build-bifrost-rococo-fast-wasm:
	.maintain/build-wasm.sh bifrost-kusama fast

.PHONY: build-try-runtime # build bifrost rococo wasm
build-try-runtime:
	cargo build -p bifrost-cli --locked --features "with-all-runtime,try-runtime" --release

.PHONY: try-kusama-runtime-upgrade # try kusama runtime upgrade
try-kusama-runtime-upgrade:build-try-runtime
	try-runtime \
		--runtime \
			target/release/wbuild/bifrost-kusama-runtime/bifrost_kusama_runtime.compact.compressed.wasm \
		on-runtime-upgrade \
		--checks none \
		--no-weight-warnings \
		live \
		--uri wss://hk.bifrost-rpc.liebi.com:443/ws 

.PHONY: try-polkadot-runtime-upgrade # try polkadot runtime upgrade
try-polkadot-runtime-upgrade:build-try-runtime
	try-runtime \
		--runtime \
		target/release/wbuild/bifrost-polkadot-runtime/bifrost_polkadot_runtime.compact.compressed.wasm \
		on-runtime-upgrade \
		--checks none \
		--no-weight-warnings \
		live \
		--uri wss://hk.p.bifrost-rpc.liebi.com:443/ws

.PHONY: resources # export genesis resources
resources:
	./target/release/bifrost export-genesis-state --chain $(CHAIN) > ./resources/para-$(PARA_ID)-genesis
	./target/release/bifrost export-genesis-wasm --chain $(CHAIN) > ./resources/para-$(PARA_ID).wasm
	./target/release/bifrost build-spec --chain $(CHAIN) --disable-default-bootnode --raw > ./resources/$(CHAIN)-raw.json

.PHONY: generate-session-key # generate session key
generate-session-key:
	./target/release/bifrost key generate --scheme Sr25519

.PHONY: insert-session-key # insert session key
insert-session-key:
	./target/release/bifrost key insert --chain $(CHAIN) --keystore-path ./resources/keystore --suri "$(SURI)" --scheme Sr25519 --key-type aura

.PHONY: generate-node-key # generate node key
generate-node-key:
	subkey generate-node-key --file ./resources/node-key

.PHONY: view-key # view keys
view-key:
	subkey inspect $(SURI) -n bifrost

.PHONY: copy-genesis-config-production # copy genesis config to resources
copy-genesis-config-production:
	mkdir -p "target/production/res"
	cp -r node/service/res/genesis_config target/production/res

.PHONY: production-release # build release for production
production-release:
	cargo build -p bifrost-cli --locked --features "with-all-runtime" --profile production

.PHONY: help # generate list of targets with descriptions
help:
	@grep '^.PHONY: .* #' Makefile | sort | sed 's/\.PHONY: \(.*\) # \(.*\)/\1	\2/' | expand -t35
