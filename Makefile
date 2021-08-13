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

.PHONY: build-all-release
build-all-release: copy-genesis-config-release
	cargo build -p node-cli --locked --features "with-all-runtime" --release

.PHONY: check-asgard
check-asgard:
	cargo check -p node-cli --locked --features "with-asgard-runtime"

.PHONY: check-bifrost
check-bifrost:
	cargo check -p node-cli --locked --features "with-bifrost-runtime"

.PHONY: check-all
check-all: format
	cargo check -p node-cli --locked --features "with-all-runtime"

.PHONY: check-tests
check-tests:
	cargo check --features "with-all-runtime" --tests

.PHONY: test-bifrost
test-bifrost:
	cargo test --features "with-bifrost-runtime"

.PHONY: test-asgard
test-asgard:
	cargo test --features "with-asgard-runtime"

.PHONY: test-all
test-all:
	cargo test --features "with-all-runtime"

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

.PHONY: run-benchmarking
run-benchmarking:
	./scripts/run_all_benches.sh

# Deploy
.PHONY: deploy-asgard-local
deploy-asgard-local:
	pm2 start scripts/asgard-local-ecosystem.config.js

.PHONY: deploy-bifrost-live
deploy-bifrost-live:
	pm2 deploy scripts/bifrost-ecosystem.config.js production

# Run dev chain
.PHONY: run-dev
run-dev:
	RUST_LOG=debug cargo run -p node-cli --locked --features "with-dev-runtime" -- --tmp --dev

.PHONY: run-dev-manual-seal
run-dev-manual-seal:
	RUST_LOG=debug cargo run -p node-cli --locked --features "with-dev-runtime" -- --tmp --dev --sealing instant

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

.PHONY: try-bifrost-runtime-upgrade
try-bifrost-runtime-upgrade:
	./scripts/try-runtime.sh bifrost

.PHONY: try-asgard-runtime-upgrade
try-asgard-runtime-upgrade:
	./scripts/try-runtime.sh asgard
