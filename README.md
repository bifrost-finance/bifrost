<a href="https://bifrost.finance"><img align="center" src="./docs/bifrost-banner.svg" alt="Bifrost Banner"/></a>

<a href="https://bifrost.finance"><img align="right" width="100" src="./docs/bifrost-black-logo.svg" alt="Bifrost Logo"/></a>

<h1 align="left"><a href="https://bifrost.finance">Homepage</a></h1>

<h4>🐣 Supported by</h4>

<p align="left">
  <a href="https://web3.foundation/grants"><img src="./docs/web3-foundation-grant.svg" width="200" alt="Web3 Foundation Grants"></a>
  <a href="https://www.substrate.io/builders-program"><img src="./docs/substrate-builder.svg" width="200" alt="Substrate Builders Program"></a>
  <a href="https://bootcamp.web3.foundation/"><img src="./docs/web3-bootcamp.svg" width="200" alt="Web3 Bootcamp"></a>
</p>

[![master-build](https://github.com/bifrost-finance/bifrost/workflows/master-build/badge.svg)](https://github.com/bifrost-finance/bifrost/actions)
[![Codacy Badge](https://app.codacy.com/project/badge/Grade/acec53276777415593c2b02b2200f62e)](https://www.codacy.com/gh/bifrost-finance/bifrost?utm_source=github.com&amp;utm_medium=referral&amp;utm_content=bifrost-finance/bifrost&amp;utm_campaign=Badge_Grade)
[![Substrate Version](https://img.shields.io/badge/Substrate-2.0.0-brightgreen?logo=Parity%20Substrate)](https://github.com/paritytech/substrate)
[![Docker](https://img.shields.io/badge/Docker-v0.4.0-brightgreen?logo=Docker)](https://hub.docker.com/repository/docker/bifrostnetwork/bifrost)
[![License](https://img.shields.io/github/license/bifrost-finance/bifrost?color=blue)](https://github.com/bifrost-finance/bifrost/blob/master/LICENSE)
[![Faucet](https://img.shields.io/badge/-Faucet-5c5c5c?logo=Telegram)](https://t.me/bifrost_faucet)
[![Twitter](https://img.shields.io/badge/-Twitter-5c5c5c?logo=Twitter)](https://twitter.com/bifrost_network)
[![Medium](https://img.shields.io/badge/-Medium-5c5c5c?logo=Medium)](https://medium.com/bifrost-network)

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

We'll start Alice's bifrost node first on default TCP port 30333 with her chain database stored locally at `/tmp/alice`. The bootnode ID of her node is `12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp`, which is generated from the `--node-key` value that we specify below:

```bash
cargo run -- \
--base-path /tmp/alice \
--chain=dev \
--alice \
--port 30333 \
--node-key 0000000000000000000000000000000000000000000000000000000000000001 \
--telemetry-url 'wss://telemetry.polkadot.io/submit/ 0' \
--validator
```

In the second terminal, we'll start Bob's bifrost node on a different TCP port of 30334, and with his chain database stored locally at `/tmp/bob`. We'll specify a value for the `--bootnodes` option that will connect his node to Alice's bootnode ID on TCP port 30333:

```bash
cargo run -- \
--base-path /tmp/bob \
--bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp \
--chain=dev \
--bob \
--port 30334 \
--telemetry-url 'wss://telemetry.polkadot.io/submit/ 0' \
--validator
```

Additional CLI usage options are available and may be shown by running `cargo run -- --help`.

### Quick way
You can use docker to run Bifrost chain, and you don't need to install rust.

If docker isn't installed on your machine, just check here to install it: [Docker Installation](https://docs.docker.com/install/).

After installation, pull the docker image by the following command:
```bash
docker pull bifrostnetwork/bifrost:v0.4.0
```

#### Start a single chain
Run the chain in quick way:
```bash
docker run -p 9944:9944 bifrostnetwork/bifrost:v0.4.0 --unsafe-ws-external --ws-port 9944 --dev
```

#### Start multi-nodes

Start alice node.
```bash
docker run -p 9944:9944 --name=alice bifrostnetwork/bifrost:v0.4.0 --base-path /tmp/alice \
--unsafe-ws-external \
--ws-port 9944 \
--chain=dev \
--alice \
--node-key 0000000000000000000000000000000000000000000000000000000000000001 \
--telemetry-url 'wss://telemetry.polkadot.io/submit/ 0' \
--validator
```

Start bob node.
```bash
docker run -p 9933:9933 --name=bob bifrostnetwork/bifrost:v0.4.0 --base-path /tmp/bob \
--bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp \
--chain=dev \
--unsafe-ws-external \
--ws-port 9933 \
--bob \
--port 30334 \
--telemetry-url 'wss://telemetry.polkadot.io/submit/ 0' \
--validator
```

Ensure both nodes are synchronizing each other.
