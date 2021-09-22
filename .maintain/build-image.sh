#!/usr/bin/env bash

VERSION=$1
NODE_NAME=bifrostnetwork/bifrost

if [[ -z "$1" ]] ; then
    VERSION=$(git rev-parse --short HEAD)
fi

DOCKER_BUILDKIT=1 docker build -t "$NODE_NAME:$VERSION" .