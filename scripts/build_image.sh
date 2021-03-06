#!/usr/bin/env bash

VERSION=$1
NODE_NAME=bifrostnetwork/bifrost

if [[ -z "$1" ]] ; then
    VERSION=$(git rev-parse --short HEAD)
fi

eval $(ssh-agent)
ssh-add ~/.ssh/bifrost_id_rsa
DOCKER_BUILDKIT=1 docker build --ssh default -t "$NODE_NAME:$VERSION" .
docker push "$NODE_NAME:$VERSION"
docker push $NODE_NAME:latest
 .