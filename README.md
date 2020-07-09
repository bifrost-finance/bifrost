<img align="right" width="100" src="./docs/bifrost-black-logo.svg" alt="Bifrost Logo"/>

<h1 align="left"><a href="https://bifrost.finance">Bifrost</a></h1>

<h4>📦 A parachain designed for staking's liquidity.</h4>

<p align="left">
  <a href="https://web3.foundation/grants"><img src="./docs/web3-foundation-grant.svg" width="200" alt="Web3 Foundation Grants"></a>
  <a href="https://www.substrate.io/builders-program"><img src="./docs/substrate-builder.svg" width="200" alt="Substrate Builders Program"></a>
  <a href="https://bootcamp.web3.foundation/"><img src="./docs/web3-bootcamp.svg" width="200" alt="Web3 Bootcamp"></a>
</p>

[![jamie-dev-build](https://github.com/bifrost-finance/bifrost/workflows/jamie-dev-build/badge.svg)](https://github.com/bifrost-finance/bifrost/actions)
[![Codacy Badge](https://app.codacy.com/project/badge/Grade/acec53276777415593c2b02b2200f62e)](https://www.codacy.com/gh/bifrost-finance/bifrost?utm_source=github.com&amp;utm_medium=referral&amp;utm_content=bifrost-finance/bifrost&amp;utm_campaign=Badge_Grade)
[![Substrate Version](https://img.shields.io/badge/Substrate-2.0.0-brightgreen?logo=Parity%20Substrate)](https://github.com/paritytech/substrate)
[![Docker](https://img.shields.io/badge/Docker-v0.3.2-brightgreen?logo=Docker)](https://hub.docker.com/repository/docker/bifrostnetwork/bifrost)
[![License](https://img.shields.io/github/license/bifrost-finance/bifrost?color=blue)](https://github.com/bifrost-finance/bifrost/blob/master/LICENSE)
[![Faucet](https://img.shields.io/badge/-Faucet-5c5c5c?logo=Telegram)](https://t.me/bifrost_faucet)
[![Twitter](https://img.shields.io/badge/-Twitter-5c5c5c?logo=Twitter)](https://twitter.com/bifrost_network)
[![Medium](https://img.shields.io/badge/-Medium-5c5c5c?logo=Medium)](https://medium.com/bifrost-network)

# Bifrost Node

A parachain focused on building bridges of chains which based on PoS consensus.

# Building

Install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh
```

Install required tools:

```bash
./scripts/init.sh
```

Build all native code:

```bash
cargo build
```

# Run

### Normal way
You can start a development chain with:

```bash
cargo run -- --dev
```

Detailed logs may be shown by running the node with the following environment variables set: `RUST_LOG=debug RUST_BACKTRACE=1 cargo run -- --dev`.

If you want to see the multi-node consensus algorithm in action locally, then you can create a local testnet with two validator nodes for Alice and Bob, who are the initial authorities of the genesis chain that have been endowed with testnet units. Give each node a name and expose them so they are listed on the Polkadot [telemetry site](https://telemetry.polkadot.io/#/Local%20Testnet). You'll need two terminal windows open.

We'll start Alice's bifrost node first on default TCP port 30333 with her chain database stored locally at `/tmp/alice`. The bootnode ID of her node is `QmRpheLN4JWdAnY7HGJfWFNbfkQCb6tFf4vvA6hgjMZKrR`, which is generated from the `--node-key` value that we specify below:

```bash
cargo run -- \
  --base-path /tmp/alice \
  --chain=local \
  --alice \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

In the second terminal, we'll start Bob's bifrost node on a different TCP port of 30334, and with his chain database stored locally at `/tmp/bob`. We'll specify a value for the `--bootnodes` option that will connect his node to Alice's bootnode ID on TCP port 30333:

```bash
cargo run -- \
  --base-path /tmp/bob \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/QmRpheLN4JWdAnY7HGJfWFNbfkQCb6tFf4vvA6hgjMZKrR \
  --chain=local \
  --bob \
  --port 30334 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

Additional CLI usage options are available and may be shown by running `cargo run -- --help`.

### Easy way
You can use docker to run Bifrost chain, and you don't need to install rust.

If docker isn't installed on your machine, just check here to install it: [Docker Installation](https://docs.docker.com/install/).

After installation, pull the docker image by the following command:
```bash
docker pull linux6/bifrost:0.1
```

#### Start a single chain
Run the chain quickly:
```bash
docker run -p 9944:9944 --name=bifrost linux6/bifrost:0.1
```

#### Start multi-nodes
If you want to run multi-nodes like the way in **Normal way**, 

Start a container named alice on tcp port 9944.
```bash
docker run -p 9944:9944 --name=alice linux6/bifrost:0.1 bifrost-node --base-path /tmp/alice \
  --chain=local \
  --alice \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

Start another container named bob on tcp port 9933.
```bash
docker run -p 9933:9933 --name=bob linux6/bifrost:0.1 bifrost-node --base-path /tmp/bob \
  --bootnodes /ip4/127.0.0.1/tcp/9933/p2p/QmRpheLN4JWdAnY7HGJfWFNbfkQCb6tFf4vvA6hgjMZKrR \
  --chain=local \
  --bob \
  --port 9933 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```
Observe both nodes.
