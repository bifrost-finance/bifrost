.PHONY: init
init:
	./scripts/init.sh

# Build Debug

.PHONY: build-asgard
build-asgard:
	cargo build -p node-cli --locked --no-default-features --features "cli with-asgard-runtime"

.PHONY: build-bifrost
build-bifrost: copy-genesis-config
	cargo build -p node-cli --locked --no-default-features --features "cli with-bifrost-runtime"

.PHONY: build-all
build-all:
	cargo build -p node-cli --locked --no-default-features --features "cli with-all-runtime"

# Build Release

.PHONY: build-asgard-release
build-asgard-release:
	cargo build -p node-cli --locked --no-default-features --features "cli with-asgard-runtime" --release

.PHONY: build-bifrost-release
build-bifrost-release: copy-genesis-config-release
	cargo build -p node-cli --locked --no-default-features --features "cli with-bifrost-runtime" --release

.PHONY: build-all-release
build-all-release:
	cargo build -p node-cli --locked --no-default-features --features "cli with-all-runtime" --release

.PHONY: check-asgard
check-asgard:
	cargo check -p node-cli --locked --no-default-features --features "cli with-asgard-runtime"

.PHONY: check-bifrost
check-bifrost:
	cargo check -p node-cli --locked --no-default-features --features "cli with-bifrost-runtime"

.PHONY: check-all
check-all:
	cargo check -p node-cli --locked --no-default-features --features "cli with-all-runtime"

.PHONY: check-tests
check-tests:
	cargo check --no-default-features --features "with-all-runtime" --tests

.PHONY: clean
clean:
	cargo clean

.PHONY: copy-genesis-config
copy-genesis-config:
	mkdir -p "target/debug"
	cp -r bin/node/service/res/genesis_config/ target/debug/

.PHONY: copy-genesis-config-release
copy-genesis-config-release:
	mkdir -p "target/release"
	cp -r bin/node/service/res/genesis_config/ target/release/