#!/bin/bash

set -xe

RUSTC_VERSION="1.60.0"
EXTRA_ARGS='--json'
RUNTIME=bifrost
BIN_PATH=$(dirname $(readlink -f $0))
WORK_PATH=${BIN_PATH}/../

###### build binary
make production-release

cp target/release/bifrost ${WORK_PATH}/resources
chmod +x ${WORK_PATH}/resources/bifrost
tar cjSf ${WORK_PATH}/resources/bifrost-x86_64-linux-gnu.tar.bz2 ${WORK_PATH}/resources/bifrost
