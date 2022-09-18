# Collator Staking

[![Rust Check & Build](https://github.com/bifrost-finance/parachain-staking/actions/workflows/ci.yml/badge.svg)](https://github.com/bifrost-finance/parachain-staking/actions/workflows/ci.yml)

## check && build

```sh
make help
make fmt
make check
make test
make build
```

## how to participant as candidate/collator

<https://docs.moonbeam.network/node-operators/networks/collator/>

### apply as candidate

extrinsic

![extrinsic](https://i.imgur.com/RqEb2kZ.png)

event

![event](https://i.imgur.com/Vwg6zOi.png)

### run collator node & map sessionKey

```sh
/home/bifrost/ronyang/parachain/bifrost/target/release/bifrost --ws-port=18850 --port=38850 --collator --base-path ./data --chain=bifrost-local --unsafe-ws-external --unsafe-rpc-external --rpc-cors=all --rpc-methods=Unsafe --rpc-port=28850 --wasm-execution=compiled --execution=wasm -- --chain=/home/bifrost/ronyang/parachain/polkadot-launch/rococo-local-raw.json --wasm-execution=compiled --execution=wasm --no-beefy
```

![i9Zyphq](https://i.imgur.com/i9Zyphq.png)

![AMGQnJM](https://i.imgur.com/AMGQnJM.png)

### bond more by self

extrinsic

![extrinsic](https://i.imgur.com/UJzYnlO.png)

event

![event](https://i.imgur.com/mopdIaG.png)

after 2 rounds the new collator begin produce blocks and receive Rewards

![Rewards](https://i.imgur.com/II2bzsn.png)

## Participate as Nominator/Delegator

<https://docs.moonbeam.network/tokens/staking/stake/>

### delegate to some candidate

extrinsic

![extrinsic](https://i.imgur.com/QFz4uJo.png)

event

![event](https://i.imgur.com/sXZPeyE.jpg)

### delegate more

we can delegate more any time

![delegate_more](https://i.imgur.com/RmztmOr.png)

### delegate less

to delegator less require schedule request first

![delegate_less](https://i.imgur.com/LF9prQ0.png)

then any one can execute the request after 36 rounds

![request](https://i.imgur.com/y27boWV.png)

### revoke delegate

to revoke means unbond from specific collator

![unbond](https://i.imgur.com/9rEDmZS.png)

then any one can execute the request after 36 rounds

![request](https://i.imgur.com/y27boWV.png)

### leave delegator

to leave means unbound from all collators and require schedule first

![unbound](https://i.imgur.com/LfYFvD4.png)

then execute the request after 36 rounds

![request](https://i.imgur.com/Lmy13x1.png)

## Staking Reward

Rewards for collators and their delegators are calculated at the start of every round for their work prior to the reward payout delay(2 rounds).

<https://docs.moonbeam.network/learn/features/staking/>

![Reward](https://i.imgur.com/AII0zJj.png)

## Some Revamp

mainly reference moonbeam implementation while decouple `nimbus` from staking, implement traits to integrate with `Session` module

### Decomple with Nimbus

<https://github.com/bifrost-finance/moonbeam/commit/2e3f7dddad6294b661e08d17b45f42e853b4ecff>

## Benifit of Nimbus

<https://docs.moonbeam.network/cn/learn/features/consensus/>

actually we've prepared another branch with nimbus integration and we may try it later if required

<https://github.com/bifrost-finance/bifrost/tree/collator-staking>

## api docs

<https://purestake.github.io/moonbeam/parachain_staking/>

## runtime storage

### candidate state

![candidate](https://i.imgur.com/e9fItmx.png)

### delegate state

![delegate](https://i.imgur.com/j1u4fMP.jpg)

unbounding/revoke request also in delegate state

### candidate pool & topN selected

![pool](https://i.imgur.com/ncQLdgN.jpg)

after each round will choose candidate from selected pool as potential block producer

### MinDelegation

there is a const value defines the min value to participant the staking

![MinDelegation](https://i.imgur.com/PIRTeP8.png)

but not all delegators for the collator will receive reward, only the top  `T::MaxDelegatorsPerCollator)`(100 by default) will, so actually the `MinDelegation` will be dynamically calculated on fly
