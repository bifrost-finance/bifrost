#### P2P protocol for EOS
Basicly, we have developed a customized EOS([bifrost-eos-relay](https://github.com/bifrost-finance/bifrost-eos-relay)) node which is capable of p2p communication already.
We add a plugin named [bridge-plugin](https://github.com/bifrost-finance/bifrost-eos-relay/tree/master/plugins/bridge_plugin).
And you have tested it as well, start two EOS nodes, one is for producing blocks, and another one as relay node to synchronize blocks and send block headers and transactions 
to Bifrost node to verify and map transactions to to Bifrost node.

#### Support EOS localnet/testnet/mainnet
- localnet. You have tested it.
- testnet. We're in phase [CC2](https://dash.bifrost.finance/#/explorer) now, and deployed EOS contract on [jungle3](http://monitor3.jungletestnet.io/#home). if you want to try it,
just tell me.
- mainnet. From testing perspective, there's no difference between testnet and mainnet, just the mainnet is official
for real users.

#### Block listener/verify
- Block listener. You can check [line 343](https://github.com/bifrost-finance/bifrost-eos-relay/blob/master/plugins/bridge_plugin/bridge_plugin.cpp#L343) at project bifrost-eos-relay.
- Block verify. You can check [line 505](https://github.com/bifrost-finance/bifrost/blob/web3_m1/brml/bridge-eos/src/lib.rs#L505) at bifrost.

#### Transaction listener/filter/verify
- Transaction listener. You can check [line 422](https://github.com/bifrost-finance/bifrost-eos-relay/blob/master/plugins/bridge_plugin/bridge_plugin.cpp#L422) at project bifrost-eos-relay.
- Transaction filter. You can check [line 449](https://github.com/bifrost-finance/bifrost-eos-relay/blob/master/plugins/bridge_plugin/bridge_plugin.cpp#L449) at project bifrost-eos-relay.
- Transaction verify. You can check [line 489](https://github.com/bifrost-finance/bifrost/blob/web3_m1/brml/bridge-eos/src/lib.rs#L489) at project bifrost.

#### Merkel tree verification
This implementation is at [merkle](https://github.com/bifrost-finance/rust-eos/blob/master/chain/src/merkle.rs) at project [rust-eos](https://github.com/bifrost-finance/rust-eos).
We use it at file [line 489](https://github.com/bifrost-finance/bifrost/blob/web3_m1/brml/bridge-eos/src/lib.rs#L489) at project bifrost, this ensures transaction is in merkle tree to avoid someone fake it.

#### Multisig transaction
1. Set threshold for signing a transaction, meaning how many signers to sign a transaction, you can find step at [readme](https://github.com/bifrost-finance/bifrost/tree/web3_m1/docs#bifrost-to-eos).
2. Save private keys to Bifrost nodes, each node has one EOS signer's key, but not store it on-chain, you might remember you executed a script named [subkey_setting.sh](https://github.com/bifrost-finance/bifrost/blob/web3_m1/docs/subkey_setting.sh)
3. About related data structures at file [transaction](https://github.com/bifrost-finance/bifrost/blob/web3_m1/brml/bridge-eos/src/transaction.rs), [line 34](https://github.com/bifrost-finance/bifrost/blob/web3_m1/brml/bridge-eos/src/transaction.rs#L34) - [line 115](https://github.com/bifrost-finance/bifrost/blob/web3_m1/brml/bridge-eos/src/transaction.rs#L115).
4. And when to sign a transaction, see file [bridge-eos](https://github.com/bifrost-finance/bifrost/blob/web3_m1/brml/bridge-eos/src/lib.rs) at [line 898](https://github.com/bifrost-finance/bifrost/blob/web3_m1/brml/bridge-eos/src/lib.rs#L898) - [line 919](https://github.com/bifrost-finance/bifrost/blob/web3_m1/brml/bridge-eos/src/lib.rs#L919).

#### Bridge contract on EOS
As you said, you have tested it.
