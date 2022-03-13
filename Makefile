PARA_ID        := 2001
DOCKER_TAG     := latest
CHAIN		   := bifrost-genesis
SURI           := //Alice

.PHONY: init
init:
	git config core.hooksPath .githooks
	./scripts/init.sh

# Build Release

.PHONY: build-bifrost-kusama-release
build-bifrost-kusama-release: copy-genesis-config-release
	cargo build -p node-cli --locked --features "with-bifrost-kusama-runtime" --release

.PHONY: build-bifrost-polkadot-release
build-bifrost-polkadot-release: copy-genesis-config-release
	cargo build -p node-cli --locked --features "with-bifrost-polkadot-runtime" --release

.PHONY: build-all-release
build-all-release: copy-genesis-config-release
	cargo build -p node-cli --locked --features "with-all-runtime" --release

.PHONY: build-bifrost-kusama-fast-release
build-bifrost-kusama-fast-release:
	cargo build -p node-cli --locked --features "with-bifrost-kusama-runtime,fast-runtime" --release


.PHONY: check-all
check-all: format
	SKIP_WASM_BUILD= cargo check -p node-cli --locked --features "with-all-runtime"

.PHONY: test-all
test-all:
	SKIP_WASM_BUILD= cargo test --features "with-all-runtime"

.PHONY: integration-test
integration-test:
	SKIP_WASM_BUILD= cargo test -p runtime-integration-tests --features=with-bifrost-kusama-runtime

.PHONY: clean
clean:
	cargo clean

.PHONY: copy-genesis-config-release
copy-genesis-config-release:
	mkdir -p "target/release/res"
	cp -r node/service/res/genesis_config target/release/res

.PHONY: format
format:
	cargo +nightly fmt --all -- --check

.PHONY: test-benchmarking
test-benchmarking:
	cargo test --features runtime-benchmarks --features with-bifrost-kusama-runtime --features --all benchmarking

.PHONY: generate-all-weights
generate-all-weights:
	bash ./scripts/generate-weights.sh bifrost

.PHONY: build-all-release-with-bench
build-all-release-with-bench: copy-genesis-config-release
	cargo build -p node-cli --locked --features "with-all-runtime,runtime-benchmarks" --release

# Deploy
.PHONY: deploy-bifrost-live
deploy-bifrost-live:
	pm2 deploy scripts/bifrost-ecosystem.config.js production

# Run dev chain
.PHONY: run-dev-manual-seal
run-dev:
	cargo run -p node-cli --locked --features "with-bifrost-kusama-runtime" -- --tmp --dev --sealing instant --rpc-cors all --unsafe-ws-external

# Build docker image
.PHONY: build-docker-image
build-docker-image:
	.maintain/build-image.sh

# Build wasm
.PHONY: build-bifrost-kusama-wasm
build-bifrost-kusama-wasm:
	.maintain/build-wasm.sh bifrost-kusama

.PHONY: build-bifrost-polkadot-wasm
build-bifrost-polkadot-wasm:
	.maintain/build-wasm.sh bifrost-polkadot

.PHONY: check-try-runtime
check-try-runtime:
	SKIP_WASM_BUILD= cargo check --features try-runtime --features with-bifrost-runtime

.PHONY: try-bifrost-runtime-upgrade
try-bifrost-runtime-upgrade:
	./scripts/try-runtime.sh bifrost

.PHONY: resources
resources:
	./target/release/bifrost export-genesis-state --chain $(CHAIN) > ./resources/para-$(PARA_ID)-genesis
	./target/release/bifrost export-genesis-wasm --chain $(CHAIN) > ./resources/para-$(PARA_ID).wasm
	./target/release/bifrost build-spec --chain $(CHAIN) --disable-default-bootnode --raw > ./resources/$(CHAIN)-raw.json

.PHONY: generate-session-key
generate-session-key:
	./target/release/bifrost key generate --scheme Sr25519

.PHONY: insert-session-key
insert-session-key:
	./target/release/bifrost key insert --chain $(CHAIN) --keystore-path ./resources/keystore --suri "$(SURI)" --scheme Sr25519 --key-type aura

.PHONY: generate-node-key
generate-node-key:
	subkey generate-node-key --file ./resources/node-key

.PHONY: view-key
view-key:
	subkey inspect $(SURI) -n bifrost

.PHONY: copy-genesis-config-production
copy-genesis-config-production:
	mkdir -p "target/production/res"
	cp -r node/service/res/genesis_config target/production/res

.PHONY: production-release
production-release:
	cargo build -p node-cli --locked --features "with-bifrost-runtime" --profile production
