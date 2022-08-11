#!/usr/bin/env bash

set -xe

RUSTC_VERSION="1.62.0"
EXTRA_ARGS='--json'
BIN_PATH=$(dirname $(readlink -f $0))
WORK_PATH=${BIN_PATH}/..
BUILD_OPTS_PARAMS="--features on-chain-release-build"

RUNTIME=$1
BUILD_TYPE=$2

 if [ "$BUILD_TYPE" == "fast" ]; then
   BUILD_OPTS_PARAMS="--features on-chain-release-build,fast-runtime"
 fi

cd runtime && ln -fsn $RUNTIME bifrost
docker run --rm -i \
  -e PACKAGE=$RUNTIME-runtime \
  -e BUILD_OPTS="$BUILD_OPTS_PARAMS" \
  -e VERBOSE=1 \
  -e CARGO_TERM_COLOR=always \
  -v ${TMPDIR}/cargo:/cargo-home \
  -v ${WORK_PATH}:/build \
  paritytech/srtool:${RUSTC_VERSION} build ${EXTRA_ARGS}

mkdir -p ${WORK_PATH}/deploy/wasm
cp ${WORK_PATH}/runtime/$RUNTIME/target/srtool/release/wbuild/$RUNTIME-runtime/${RUNTIME/-/_}_runtime.compact.compressed.wasm \
${WORK_PATH}/deploy/wasm
