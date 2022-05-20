#!/bin/bash

set -xe

RUSTC_VERSION="1.60.0"
BIN_PATH=$(dirname $(readlink -f $0))
WORK_PATH=${BIN_PATH}/..

# build binary
make production-release

# pack artifacts
mkdir -p ${WORK_PATH}/artifacts
mv ${WORK_PATH}/target/production/bifrost ${WORK_PATH}/artifacts/
pushd artifacts
sha256sum bifrost | tee bifrost.sha256
shasum -c bifrost.sha256
popd
