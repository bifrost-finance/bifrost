#!/bin/bash

set -xe

RUSTC_VERSION="1.53.0"
EXTRA_ARGS='--json'
RUNTIME=bifrost
BIN_PATH=$(dirname $(readlink -f $0))
WORK_PATH=${BIN_PATH}/../

###### build binary
make build-bifrost-release

cp target/release/bifrost ${WORK_PATH}/resources
chmod +x ${WORK_PATH}/resources/bifrost
tar cjSf ${WORK_PATH}/resources/bifrost-x86_64-linux-gnu.tar.bz2 ${WORK_PATH}/resources/bifrost


###### build wasm
docker run --rm -it \
  -e PACKAGE=$RUNTIME-runtime \
  -e VERBOSE=1 \
  -e CARGO_TERM_COLOR=always \
  -v ${TMPDIR}/cargo:/cargo-home \
  -v ${WORK_PATH}:/build \
  paritytech/srtool:${RUSTC_VERSION} build ${EXTRA_ARGS}

cp ${WORK_PATH}/runtime/$RUNTIME/target/srtool/release/wbuild/$RUNTIME-runtime/${RUNTIME}_runtime.compact.wasm \
${WORK_PATH}/resources
tar cjSf ${WORK_PATH}/resources/bifrost-wasm.tar.bz2 ${WORK_PATH}/resources/${RUNTIME}_runtime.compact.wasm