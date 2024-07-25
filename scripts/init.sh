#!/usr/bin/env bash

set -e

echo "*** Initializing WASM build environment"

rustup default 1.77.0

rustup target add wasm32-unknown-unknown --toolchain 1.77.0
