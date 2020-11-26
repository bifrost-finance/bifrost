## All related repotories
- [bifrost](https://github.com/bifrost-finance/bifrost) (branch: develop)
- [rust-eos](https://github.com/bifrost-finance/rust-iost) (branch: master)

## Bifrost

### 1. Compile
Follow the [readme](https://github.com/bifrost-finance/bifrost/tree/develop) to compile a bifrost node.
```
$ git clone https://github.com/bifrost-finance/bifrost.git
$ git checkout develop
$ cargo build --release
```

### 2. Run nodes

Start two Bifrost nodes.

Alice node:
```
$ ./target/release/bifrost --base-path /tmp/alice \
--rpc-port 4321 \
--ws-port 9944 \
--chain=dev \
--alice \
--port 30333 \
--node-key 0000000000000000000000000000000000000000000000000000000000000001 \
--validator
```

Bob node:
```
$ ./target/release/bifrost --base-path /tmp/bob \
--rpc-port 1234 \
--ws-port 9933 \
--bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp \
--chain=dev \
--bob \
--port 30334 \
--validator
```

Ensure both node are producing blocks and synchronizing each other.

## IOST

### 1. Build IOST
Follow the instructions to build [IOST](https://developers.iost.io/docs/en/4-running-iost-node/Building-IOST.html)


### 2. Launch local IOST server

```shell
$ iserver -f ./config/iserver.yml
```

- Follow the instructions to create accounts [Command Line Wallet Tool](https://developers.iost.io/docs/en/4-running-iost-node/iWallet.html).


**Tips**:
> While you're creating account for test, use the `--sign_algo secp256k1` to specify the sign algorithm as SECP256K1.

```shell

$ iwallet --server 127.0.0.1:30002 --account admin --amount_limit "ram:1000|iost:10" --sign_algo secp256k1  account create lispczz4 --initial_balance 0 --initial_gas_pledge 10 --initial_ram 0
$ iwallet --server 127.0.0.1:30002 --account admin --amount_limit "ram:1000|iost:10" --sign_algo secp256k1  account create lispczz5 --initial_balance 0 --initial_gas_pledge 10 --initial_ram 0
$ iwallet --server 127.0.0.1:30002 --account admin --amount_limit "ram:1000|iost:10" --sign_algo secp256k1  account create bifrost --initial_balance 0 --initial_gas_pledge 10 --initial_ram 0

```
> If don't have enought gas, using following command to issue more [Economic Contract](https://developers.iost.io/docs/en/6-reference/EconContract.html#pledgepledgor-to-amount).

```
$ iwallet --account lispczz4 call 'gas.iost' 'pledge' '["lispczz4","lispczz4","10"]'
```

## Testing

### Configure Browser

Go to [polkadot.js.org](https://polkadot.js.org/apps/#/settings/developer), Copy content data from the file ```developer_setting.json``` to **Deveoper** tab like this, and save it.
![developer_setting](developer_setting.png)

### EOS to Bifrost

Before you send a transaction to Bifrost, check **jim**'s and **bifrost**'s balance.

```
# should print 10000.0000 EOS
$ cleos get currency balance eosio.token jim

# bifrost is contract account, should print nothing
$ cleos get currency balance eosio.token bifrostcross
```

Now send a transaction.
```
$ cleos push action eosio.token transfer '["jim", "bifrostcross", "100.0000 EOS", "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY@bifrost:vEOS"]' -p jim@active
```

Go to [polkadot.js.org](https://polkadot.js.org/apps/#/extrinsics), to check whether transaction is sent successfully to Bifrost or not.

Wait about 90 seconds for the transaction is verified. If all go well, you can see a event like the following screencapture.

![prove_action_event](prove_action_event.png)

If that event happens, Alice's assets will be created, the amount is 1000000(due to EOS precision), go to check Alice's assets that just created.

![assets_creation](assets_creation.png)

If you see that figure above, go check **jim**'s and **bifrostcross**'s balance again.
```
# should print 9900.0000 EOS
$ cleos get currency balance eosio.token jim

# should print 100.0000 EOS
$ cleos get currency balance eosio.token bifrostcross
```

### Bifrost to EOS

Before testing, you have to setup some necessary steps.

- Multisignature Configuration

Bifrost side:

There're two Bifrost nodes that you start in previous steps, here you need add EOS node address info and EOS secret key
to both running Bifrost nodes by tool **subkey**.

Execute the script. This script will add necessary data to alice node and bob node.
```
$ ./subkey_setting.sh
```

EOS side:

```
$ cleos set account permission bifrostcross active '{"threshold":2,"keys":[],"accounts":[{"permission":{"actor":"testa","permission":"active"},"weight":1}, {"permission":{"actor":"testb","permission":"active"},"weight":1}, {"permission":{"actor":"testc","permission":"active"},"weight":1}, {"permission":{"actor":"testd","permission":"active"},"weight":1}]}' owner
```

After you set permission for account bifrost, try this command to verify the result.
```
$ cleos get account bifrostcross
```

It should print some info like this.

```
permissions:
     owner     1:    1 EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV
        active     2:    1 testa@active, 1 testb@active, 1 testc@active, 1 testd@active
```

- Send transaction

Now, we can send a transaction to EOS node.

Follow the picture to send a transaction to EOS node( "jim" to hex: "0x6a696d").
![send_transaction](transaction_to_eos.png)

Surely you can go to [polkadot.js.org](https://polkadot.js.org/apps/#/extrinsics) to check Alice's assets change or not

Check jim's and bifrostcross's balance in EOS node if it runs without error.

```
# should print 9910 EOS
$ cleos get currency balance eosio.token jim

# should print 90 EOS
$ cleos get currency balance eosio.token bifrostcross
```
