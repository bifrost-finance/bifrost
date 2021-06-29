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
	cargo fmt --all -- --check