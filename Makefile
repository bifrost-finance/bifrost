PARA_ID        := 2001
DOCKER_TAG     := latest
CHAIN		   := bifrost-local
SURI           := //Alice

.PHONY: init
init:
	git config core.hooksPath .githooks
	./scripts/init.sh

# Build Debug

.PHONY: build-asgard
build-asgard: copy-genesis-config
	cargo build -p node-cli --locked --features "with-asgard-runtime"

.PHONY: build-bifrost
build-bifrost: copy-genesis-config
	cargo build -p node-cli --locked --features "with-bifrost-runtime"

.PHONY: build-all
build-all: copy-genesis-config
	cargo build -p node-cli --locked --features "with-all-runtime"

# Build Release

.PHONY: build-asgard-release
build-asgard-release: copy-genesis-config-release
	cargo build -p node-cli --locked --features "with-asgard-runtime" --release

.PHONY: build-bifrost-release
build-bifrost-release: copy-genesis-config-release
	cargo build -p node-cli --locked --features "with-bifrost-runtime" --release

.PHONY: build-bifrost-polkadot-release
build-bifrost-polkadot-release: copy-genesis-config-release
	cargo build -p node-cli --locked --features "with-bifrost-polkadot-runtime" --release

.PHONY: build-all-release
build-all-release: copy-genesis-config-release
	cargo build -p node-cli --locked --features "with-all-runtime" --release

.PHONY: check-asgard
check-asgard:
	SKIP_WASM_BUILD= cargo check -p node-cli --locked --features "with-asgard-runtime"

.PHONY: check-bifrost
check-bifrost:
	SKIP_WASM_BUILD= cargo check -p node-cli --locked --features "with-bifrost-runtime"

.PHONY: check-all
check-all: format
	SKIP_WASM_BUILD= cargo check -p node-cli --locked --features "with-all-runtime"

.PHONY: check-tests
check-tests:
	cargo check --features "with-all-runtime" --tests

.PHONY: test-bifrost
test-bifrost:
	SKIP_WASM_BUILD= cargo test --features "with-bifrost-runtime"

.PHONY: test-asgard
test-asgard:
	SKIP_WASM_BUILD= cargo test --features "with-asgard-runtime"

.PHONY: test-all
test-all:
	SKIP_WASM_BUILD= cargo test --features "with-all-runtime"

.PHONY: integration-test
integration-test:
	SKIP_WASM_BUILD= cargo test -p runtime-integration-tests --features=with-asgard-runtime

.PHONY: clean
clean:
	cargo clean

.PHONY: copy-genesis-config
copy-genesis-config:
	mkdir -p "target/debug/res"
	cp -r node/service/res/genesis_config target/debug/res

.PHONY: copy-genesis-config-release
copy-genesis-config-release:
	mkdir -p "target/release/res"
	cp -r node/service/res/genesis_config target/release/res

.PHONY: format
format:
	rustup component add rustfmt
	cargo +nightly fmt --all -- --check

.PHONY: test-benchmarking
test-benchmarking:
	cargo test --features runtime-benchmarks --features with-all-runtime --features --all benchmarking

.PHONY: generate-bifrost-weights
generate-bifrost-weights:
	bash ./scripts/generate-weights.sh bifrost

.PHONY: generate-asgard-weights
generate-asgard-weights:
	bash ./scripts/generate-weights.sh asgard

.PHONY: generate-all-weights
generate-all-weights:
	bash ./scripts/generate-weights.sh asgard bifrost

.PHONY: build-asgard-release-with-bench
build-asgard-release-with-bench: copy-genesis-config-release
	cargo build -p node-cli --locked --features "with-asgard-runtime,runtime-benchmarks" --release

.PHONY: build-bifrost-release-with-bench
build-bifrost-release-with-bench: copy-genesis-config-release
	cargo build -p node-cli --locked --features "with-bifrost-runtime,runtime-benchmarks" --release

.PHONY: build-all-release-with-bench
build-all-release-with-bench: copy-genesis-config-release
	cargo build -p node-cli --locked --features "with-all-runtime,runtime-benchmarks" --release

# Deploy
.PHONY: deploy-asgard-local
deploy-asgard-local:
	pm2 start scripts/asgard-local-ecosystem.config.js

.PHONY: deploy-bifrost-live
deploy-bifrost-live:
	pm2 deploy scripts/bifrost-ecosystem.config.js production

# Run dev chain
.PHONY: run-dev-manual-seal
run-dev:
	RUST_LOG=debug CARGO_INCREMENTAL=0 cargo run -p node-cli --locked --features "with-asgard-runtime" -- --tmp --dev --sealing instant --rpc-cors all --unsafe-ws-external

# Build docker image
.PHONY: build-docker-image
build-docker-image:
	.maintain/build-image.sh

# Build wasm
.PHONY: build-bifrost-wasm
build-bifrost-wasm:
	.maintain/build-wasm.sh bifrost

.PHONY: build-asgard-wasm
build-asgard-wasm:
	.maintain/build-wasm.sh asgard

.PHONY: check-try-runtime
check-try-runtime:
	SKIP_WASM_BUILD= cargo check --features try-runtime --features with-bifrost-runtime

.PHONY: try-bifrost-runtime-upgrade
try-bifrost-runtime-upgrade:
	./scripts/try-runtime.sh bifrost

.PHONY: try-asgard-runtime-upgrade
try-asgard-runtime-upgrade:
	./scripts/try-runtime.sh asgard

.PHONY: resources
resources:
	./target/release/bifrost export-genesis-state --chain $(CHAIN) --parachain-id $(PARA_ID) > ./resources/para-$(PARA_ID)-genesis
	./target/release/bifrost export-genesis-wasm --chain $(CHAIN) > ./resources/para-$(PARA_ID).wasm
	./target/release/bifrost build-spec --chain $(CHAIN) --disable-default-bootnode --raw > ./resources/$(CHAIN)-raw.json

.PHONY: keystore
keystore:
	./target/release/bifrost key insert --chain $(CHAIN) --keystore-path ./resources/keystore --suri "$(SURI)" --key-type aura
	./target/release/bifrost key insert --chain $(CHAIN) --keystore-path ./resources/keystore --suri "$(SURI)" --key-type gran

.PHONY: production-release
production-release:
	.maintain/publish-release.sh
