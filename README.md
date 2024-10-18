<a href="https://bifrost.io"><img align="center" src="./docs/res/readme/bifrost-banner.svg" alt="Bifrost Banner"/></a>

<a href="https://bifrost.io"><img align="right" width="100" src="./docs/res/readme/bifrost-black-logo.svg" alt="Bifrost Logo"/></a>

<h1 align="left"><a href="https://bifrost.io">Homepage</a></h1>

Welcome,

Bifrost is a Web3 derivatives protocol that provides decentralized cross-chain liquidity for staked assets. By leveraging on the cross-consensus message ([XCM](https://wiki.polkadot.network/docs/learn-xcm)) it can provide cross-chain liquid staking services for multiple chains.

[Our mission](https://notion.bifrost.io/Bifrost-Roadmap-ba6c44bd4f684e5aa875ec44388b2330) is to provide standardized cross-chain interest-bearing derivatives for [Polkadot](https://polkadot.network) relay chains, parachains, and heterogeneous chains bridged with Polkadot.

👉 _Discover the Bifrost at [bifrost.io](https://bifrost.io/)._  
👉 _Learn to use the Bifrost with our [docs](https://docs.bifrost.io/)._

<h4>🐣 Supported by</h4>

<p align="left">
  <a href="https://web3.foundation/grants"><img src="docs/res/readme/web3-foundation-grant.svg" width="200" alt="Web3 Foundation Grants"></a>
  <a href="https://www.substrate.io/builders-program"><img src="docs/res/readme/substrate-builder.svg" width="200" alt="Substrate Builders Program"></a>
  <a href="https://bootcamp.web3.foundation/"><img src="docs/res/readme/web3-bootcamp.svg" width="200" alt="Web3 Bootcamp"></a>
</p>

[![master-build](https://img.shields.io/github/actions/workflow/status/bifrost-io/bifrost/ci-build.yml?logo=Buddy)](https://github.com/bifrost-io/bifrost/actions/workflows/ci-build.yml)
[![Codacy Badge](https://app.codacy.com/project/badge/Grade/acec53276777415593c2b02b2200f62e)](https://www.codacy.com/gh/bifrost-io/bifrost?utm_source=github.com&utm_medium=referral&utm_content=bifrost-io/bifrost&utm_campaign=Badge_Grade)
[![Substrate Version](https://img.shields.io/badge/Substrate-latest-brightgreen?logo=Parity%20Substrate)](https://github.com/paritytech/substrate)
[![License](https://img.shields.io/github/license/bifrost-io/bifrost?color=blue)](https://github.com/bifrost-io/bifrost/blob/master/LICENSE)
[![Dapp](https://img.shields.io/badge/Dapp-5c5c5c?logo=Icinga)](https://app.bifrost.io)
[![Analytics](https://img.shields.io/badge/-Analytics-5c5c5c?logo=Google%20Analytics)](https://stats.bifrost.io)
[![Discord](https://img.shields.io/badge/-Discord-5c5c5c?logo=Discord)](https://discord.gg/bifrost-io)
[![Twitter](https://img.shields.io/badge/-X-5c5c5c?logo=X&logoColor=white)](https://x.com/Bifrost)

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
make format
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
# bifrost kusama
make try-kusama-runtime-upgrade

# bifrost polkadot
make try-polkadot-runtime-upgrade
```

## Run development chain

run node with `--chain=bifrost-polkadot-dev` to enable development mode.

Before use dev mode, modify OnTimestampSet to be ()

```rust
impl pallet_timestamp::Config for Runtime {
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
   -type OnTimestampSet = Aura;
   +type OnTimestampSet = ();
	type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

```

## Run local testnet with polkadot-launch

### Install `polkadot-launch`

```bash
yarn global add polkadot-launch
cd -
```

### Build polkadot

```bash
# replace version with your target polkadot version
cargo install --git https://github.com/paritytech/polkadot --tag <version> polkadot --locked
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
  --trie-cache-size 0 \
  --execution wasm
```

### snapshot

There are also some snapshots you can use to quickly get started, these are provided by the community.

-   Pre-req .

    zstd and aria2

    ```sh

    sudo apt install zstd
    sudo apt install aria2
    ```

#### bifrost-kusama snapshots

-   relay-chain data

```sh
# download dict
wget https://snapshot-1258776962.cos.ap-hongkong.myqcloud.com/bifrost-kusama/relay.dict

# download zst data
aria2c -x10 https://snapshot-1258776962.cos.ap-hongkong.myqcloud.com/bifrost-kusama/relay.tar.zst

# decompress: node is basepath, you can replace any dicrectory you like
mkdir node
tar -I 'zstd -vd -T0 -D relay.dict' -xvf relay.tar.zst -C node/.
```

-   parachain data

```sh
wget https://snapshot-1258776962.cos.ap-hongkong.myqcloud.com/bifrost-kusama/para.dict
aria2c -x10  https://snapshot-1258776962.cos.ap-hongkong.myqcloud.com/bifrost-kusama/para.tar.zst

tar -I 'zstd -vd -T0 -D para.dict' -xvf para.tar.zst -C node/.
```

#### bifrost-polkadot snapshots

link:

-   [relay chain dict](https://snapshot-1258776962.cos.ap-hongkong.myqcloud.com/bifrost-polkadot/relay.dict)
-   [relay chain zst data](https://snapshot-1258776962.cos.ap-hongkong.myqcloud.com/bifrost-polkadot/relay.tar.zst)

-   [para chain dict](https://snapshot-1258776962.cos.ap-hongkong.myqcloud.com/bifrost-polkadot/para.dict)
-   [para chain zst data](https://snapshot-1258776962.cos.ap-hongkong.myqcloud.com/bifrost-polkadot/para.tar.zst)
