#!/bin/sh

set -xe

RUSTC_VERSION="1.56.1"
EXTRA_ARGS='--json'
BIN_PATH=$(dirname $(readlink -f $0))
WORK_PATH=${BIN_PATH}/..

RUNTIME=$1

docker run --rm -it \
  -e PACKAGE=$RUNTIME-runtime \
  -e VERBOSE=1 \
  -e CARGO_TERM_COLOR=always \
  -v ${TMPDIR}/cargo:/cargo-home \
  -v ${WORK_PATH}:/build \
  paritytech/srtool:${RUSTC_VERSION} build ${EXTRA_ARGS}

mkdir -p ${WORK_PATH}/deploy/wasm
cp ${WORK_PATH}/runtime/$RUNTIME/target/srtool/release/wbuild/$RUNTIME-runtime/${RUNTIME}_runtime.compact.compressed.wasm \
${WORK_PATH}/deploy/wasm