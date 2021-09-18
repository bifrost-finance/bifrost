#!/usr/bin/env bash

VERSION=$1
NODE_NAME=bifrostnetwork/bifrost

if [[ -z "$1" ]] ; then
    VERSION=$(git rev-parse --short HEAD)
fi

docker push "$NODE_NAME:$VERSION"
docker push $NODE_NAME:latest