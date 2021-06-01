.PHONY: init
init:
	./scripts/init.sh

# Build Debug

.PHONY: build-asgard
build-asgard:
	cargo build -p node-cli --locked --features "with-asgard-runtime"

.PHONY: build-bifrost
build-bifrost:
	cargo build -p node-cli --locked --features "with-bifrost-runtime"

.PHONY: build-all
build-all:
	cargo build -p node-cli --locked --features "with-all-runtime"

# Build Release

.PHONY: build-asgard-release
build-asgard-release:
	cargo build -p node-cli --locked --features "with-asgard-runtime" --release

.PHONY: build-bifrost-release
build-bifrost-release:
	cargo build -p node-cli --locked --features "with-bifrost-runtime" --release

.PHONY: build-all-release
build-all-release:
	cargo build -p node-cli --locked --features "with-all-runtime" --release

.PHONY: check-asgard
check-asgard:
	cargo check -p node-cli --locked --features "with-asgard-runtime"

.PHONY: check-bifrost
check-bifrost:
	cargo check -p node-cli --locked --features "with-bifrost-runtime"

.PHONY: check-all
check-all:
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

# TODO: Copy genesis_config JSON files to target directory