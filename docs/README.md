## All related repotories
- [bifrost](https://github.com/bifrost-finance/bifrost) (branch: master)
- [bifrost-eos-relay](https://github.com/bifrost-finance/bifrost-eos-relay) (branch: master)
- [bifrost-eos-contracts](https://github.com/bifrost-finance/bifrost-eos-contracts) (branch: master)
- [rust-eos](https://github.com/bifrost-finance/rust-eos) (branch: master)

## Bifrost

### 1. Compile
Follow the [readme](https://github.com/bifrost-finance/bifrost/tree/master) to compile a bifrost node.
```
$ git clone https://github.com/bifrost-finance/bifrost.git
$ git checkout master
$ cargo build --release
```

### 2. Run nodes

Start two Bifrost nodes.

Alice node: 
```
$ ./target/release/bifrost-node --base-path /tmp/alice \
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
$ ./target/release/bifrost-node --base-path /tmp/bob \
--rpc-port 1234 \
--ws-port 9933 \
--bootnodes /ip4/127.0.0.1/tcp/30333/p2p/QmRpheLN4JWdAnY7HGJfWFNbfkQCb6tFf4vvA6hgjMZKrR \
--chain=dev \
--bob \
--port 30334 \
--validator
```

Ensure both node are producing blocks and synchronizing each other.

## EOS

### 1. EOS runtime installation
Follow the instructions to install [eosio](https://developers.eos.io/eosio-home/docs/setting-up-your-environment)

---

### 2. Create a dev wallet

**Tips**: 
> While you're creating wallet, use the following command in case you forget the passoword
```
# do not use cleos wallet create --to-console
$ cleos wallet create --to-file
```

- Follow the instructions to create wallet [Create Development Wallet](https://developers.eos.io/eosio-home/docs/wallets).


> The wallet folder will created under ```~/eosio-wallet```. If you forget the password but you have to unclock the wallet, use the following commands.
```
$ cat ~/eosio-wallet/default.pass # that will show the password
$ cleos wallet unlock # prompt you input the password
```
---

### 3. Compile and run eos node

#### Prerequisites
- Cmake
- LLVM@7.0(at most 9.0)
- Rust(at least 1.40)
- CDT(Contract Development Toolkit). Follow this tutorial to install [cdt](https://developers.eos.io/eosio-home/docs/installing-the-contract-development-toolkit).

#### Compile

```
$ git clone https://github.com/bifrost-finance/bifrost-eos-relay.git
$ git checkout master
$ cd bifrost-eos-relay/
$ git submodule update --init --recursive
$ git tag v2.0.4 -m 'fix no tag'
$ mkdir build && cd build
$ cmake ..
$ make -j4
```

#### Run

The script: **start-producer.sh** && **start-relay.sh**.

Modify the script and find out **BIN_DIR** and **BASE_DIR**, point to your EOS project.

- Start block producer.
This node producers blocks.

```
$ ./start-producer.sh
```

- Start a EOS relay node.

This node is responsible for message sending like merkle root verification data, and surely synchronize blocks from block producers.
You have to modify this shell script before start this service.

Start it.
```
$ ./start-relay.sh
```

**Tips**: 
> If you get a error like dirty database,
```
rethrow "state" database dirty flag set: 
    {"what":"\"state\" database dirty flag set"}
    thread-0  chain_plugin.cpp:958 plugin_initialize
```

> or want to delete all histoty blocks, run the following command.
```shell
$ ./build/bin/nodeos --delete-all-blocks --delete-state-history --delete-relay-history --plugin eosio::bridge_plugin
```

---

### 4. Compile && Deploy contract

#### Compile
```
$ git clone https://github.com/bifrost-finance/bifrost-eos-contracts
$ cd bifrost-eos-contracts
$ mkdir build && cd build
$ cmake ..
$ make -j4
```
The abi and wasm file will generated under folder **build/contracts/bifrost.bridge**, 
files like **bifrost.bridge.abi**, **bifrost.bridge.wasm**.

#### Deployment
The script: **deploy_contracts.sh**

What the script will do:

- Deploy contract.
- Creates two accounts for testing, **jim** and **bifrost**.
- Issue 10000.0000 EOS to jim.

Modify the script.
- Line 7, point to eos project.
- Line 13, point to bifrost-eos-contracts project.

Execute it.
```shell
$ ./deploy_contracts.sh
```
It should run without errors.

**Tips**:
> If you get error like 
```
Error 3120003: Locked wallet
Ensure that your wallet is unlocked before using it!
Error Details:
You don't have any unlocked wallet!
```
Go back to section **Create a dev wallet**'s tips to unlock the wallet.

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
