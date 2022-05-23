<a href="https://bifrost.finance"><img align="center" src="./docs/res/readme/bifrost-banner.svg" alt="Bifrost Banner"/></a>

<a href="https://bifrost.finance"><img align="right" width="100" src="./docs/res/readme/bifrost-black-logo.svg" alt="Bifrost Logo"/></a>

<h1 align="left"><a href="https://bifrost.finance">Homepage</a></h1>

Welcome,

Bifrost is a Web3 derivatives protocol that provides decentralized cross-chain liquidity for staked assets. By leveraging on the cross-consensus message (XCM) it can provide cross-chain liquid staking services for polkadot relay chains, parachains, and heterogeneous chains bridged with polkadot.

👉 *Discover the Bifrost project at [bifrost.finance](https://bifrost.finance/).*  
👉 *Learn to use the Bifrost network with our [wiki](https://wiki.bifrost.finance/network).*  

<h4>🐣 Supported by</h4>

<p align="left">
  <a href="https://web3.foundation/grants"><img src="docs/res/readme/web3-foundation-grant.svg" width="200" alt="Web3 Foundation Grants"></a>
  <a href="https://www.substrate.io/builders-program"><img src="docs/res/readme/substrate-builder.svg" width="200" alt="Substrate Builders Program"></a>
  <a href="https://bootcamp.web3.foundation/"><img src="docs/res/readme/web3-bootcamp.svg" width="200" alt="Web3 Bootcamp"></a>
</p>

[![master-build](https://img.shields.io/github/workflow/status/bifrost-finance/bifrost/master-build/master)](https://github.com/bifrost-finance/bifrost/actions)
[![Codacy Badge](https://app.codacy.com/project/badge/Grade/acec53276777415593c2b02b2200f62e)](https://www.codacy.com/gh/bifrost-finance/bifrost?utm_source=github.com&amp;utm_medium=referral&amp;utm_content=bifrost-finance/bifrost&amp;utm_campaign=Badge_Grade)
[![Substrate Version](https://img.shields.io/badge/Substrate-3.0.0-brightgreen?logo=Parity%20Substrate)](https://github.com/paritytech/substrate)
[![Docker](https://img.shields.io/badge/Docker-v0.4.0-brightgreen?logo=Docker)](https://hub.docker.com/repository/docker/bifrostnetwork/bifrost)
[![License](https://img.shields.io/github/license/bifrost-finance/bifrost?color=blue)](https://github.com/bifrost-finance/bifrost/blob/master/LICENSE)
[![Discord](https://img.shields.io/badge/-Discord-5c5c5c?logo=Discord)](https://discord.gg/bifrost-finance)
[![Twitter](https://img.shields.io/badge/-Twitter-5c5c5c?logo=Twitter)](https://twitter.com/bifrost_finance)
[![Medium](https://img.shields.io/badge/-Medium-5c5c5c?logo=Medium)](https://medium.com/bifrost-finance)

## Get Build Help

```sh
make help
```

## Install Rust and required tools

```bash
curl https://sh.rustup.rs -sSf | sh
make init
```

## Build binary

```bash
make build-all-release
```

## Format code

```sh
make fmt
```

## Lint code

```sh
make clippy
```

## Testing

```bash
make test-all
```

## Generate runtime weights

if runtime logic change we may do the benchmarking to regenerate WeightInfo for dispatch calls

```bash
make generate-all-weights
```

## Testing runtime migration

If modify the storage, should test the data migration before production upgrade.

```bash
make try-bifrost-runtime-upgrade
```

## Run development chain

```bash
make run-dev
```

## Run local testnet with polkadot-launch

### Install `polkadot-launch`

```bash
yarn global add polkadot-launch
cd -
```

### Build polkadot

```bash
git clone -n https://github.com/paritytech/polkadot.git /tmp/polkadot
cd /tmp/polkadot
git checkout release-v0.9.19
cargo build --release
cd -
```

### Launch Polkadot and the parachain

```bash
cd -
polkadot-launch ./scripts/bifrost-launch.json
```

It will take about 1-2 minutes for the parachain to start producing blocks.

## Run local testnet with parachain-launch

### Install `parachain-launch`

```sh
yarn global add @open-web3/parachain-launch
```

### Generate docker files

```sh
parachain-launch generate --config=scripts/bifrost-docker-launch.yml --yes
```

It will pull images and generate required docker files in a folder called `output` in your current working directory

### Start relaychain and parachain

To start the nodes, navigate to the output folder that the generated docker scripts in and start containers:

```sh
cd ./output
docker-compose up -d --build
```

## Run full node

### Create `bifrost-fullnode` directory, generate `node-key` and get `bifrost.json`

```sh
mkdir -p ~/node-key
subkey generate-node-key --file ~/node-key/bifrost.key
```

### Start full node

Replace your-fullnode-name

```sh
docker pull bifrostnetwork/bifrost:latest
docker run -d \
-v ~/node-key:/node-key \
-p 9944:9944 \
-p 9933:9933 \
-p 30333:30333 \
bifrostnetwork/bifrost:latest \
  --name your-fullnode-name \
  --base-path "/data" \
  --node-key-file "/node-key/bifrost.key" \
  --chain "/spec/bifrost.json" \
  --pruning=archive \
  --rpc-external \
  --ws-external \
  --rpc-cors all \
  --state-cache-size 0 \
  --execution wasm
```
