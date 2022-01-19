# Collator Staking Guidance

## how to participant as candidate/collator
https://docs.moonbeam.network/node-operators/networks/collator/

### apply as candidate
extrinsic
![](https://i.imgur.com/RqEb2kZ.png)
event
![](https://i.imgur.com/Vwg6zOi.png)

### run collator node & map sessionKey

```
/home/bifrost/ronyang/parachain/bifrost/target/release/bifrost --ws-port=18850 --port=38850 --collator --base-path ./data --chain=asgard-local --unsafe-ws-external --unsafe-rpc-external --rpc-cors=all --rpc-methods=Unsafe --rpc-port=28850 --wasm-execution=compiled --execution=wasm -- --chain=/home/bifrost/ronyang/parachain/polkadot-launch/rococo-local-raw.json --wasm-execution=compiled --execution=wasm --no-beefy
```

![](https://i.imgur.com/i9Zyphq.png)

![](https://i.imgur.com/AMGQnJM.png)


### bond more by self
extrinsic
![](https://i.imgur.com/UJzYnlO.png)
event
![](https://i.imgur.com/mopdIaG.png)

after 2 rounds the new candidate begin produce blocks and receive Rewards

![](https://i.imgur.com/II2bzsn.png)


## how to participant as delegator/nominator
https://docs.moonbeam.network/tokens/staking/stake/

## api docs
https://purestake.github.io/moonbeam/parachain_staking/

## Participate as Nominator/Delegator

### delegate to some candidate
extrinsic
![](https://i.imgur.com/QFz4uJo.png)
event
![](https://i.imgur.com/sXZPeyE.jpg)

### delegate more

we can delegate more any time
![](https://i.imgur.com/RmztmOr.png)


### leave delegator

to leave delegator require schedule request first
![](https://i.imgur.com/LfYFvD4.png)

then execute the request after 24 rounds
![](https://i.imgur.com/Lmy13x1.png)


## Staking Reward

Rewards for collators and their delegators are calculated at the start of every round for their work prior to the reward payout delay.

https://docs.moonbeam.network/learn/features/staking/

![](https://i.imgur.com/AII0zJj.png)


## Storages

### candidate state

![](https://i.imgur.com/e9fItmx.png)


### delegate state
![](https://i.imgur.com/VBZ00aE.png)

### candidate pool & topN selected

![](https://i.imgur.com/ncQLdgN.jpg)

after each round will choose candidate from selected pool as potential block producer


# Some Revamp

mainly reference moonbeam implementation while decouple nimbus from staking, implement traits to integrate with Session module


## Decomple with Nimbus

https://github.com/bifrost-finance/moonbeam/commit/2e3f7dddad6294b661e08d17b45f42e853b4ecff


## Benifit of Nimbus

https://docs.moonbeam.network/cn/learn/features/consensus/

actually we've another branch with nimbus integration and we may try it later if required 
https://github.com/bifrost-finance/bifrost/tree/collator-staking


