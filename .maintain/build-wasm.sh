#!/bin/sh

set -xe

RUSTC_VERSION="1.53.0"
EXTRA_ARGS='--json'
BIN_PATH=$(dirname $(readlink -f $0))
WORK_PATH=${BIN_PATH}/..
eval $(ssh-agent)
ssh-add ~/.ssh/github_actions

RUNTIME=$1

docker run --rm -it \
  -e PACKAGE=$RUNTIME-runtime \
  -e VERBOSE=1 \
  -e CARGO_TERM_COLOR=always \
  -e SSH_AUTH_SOCK=/ssh-agent \
  -v $(readlink -f $SSH_AUTH_SOCK):/ssh-agent \
  -v ${TMPDIR}/cargo:/cargo-home \
  -v ${WORK_PATH}:/build \
  paritytech/srtool:${RUSTC_VERSION} build ${EXTRA_ARGS}

mkdir -p ${WORK_PATH}/deploy/wasm
cp ${WORK_PATH}/runtime/$RUNTIME/target/srtool/release/wbuild/$RUNTIME-runtime/${RUNTIME}_runtime.compact.wasm \
${WORK_PATH}/deploy/wasm