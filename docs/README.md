## All related repotories
- [bifrost](https://github.com/bifrost-codes/bifrost) (branch: ark-bridge-module)
- [bifrost-eos-relay](https://github.com/bifrost-codes/bifrost-eos-relay) (branch: bridge-plugin)
- [bifrost-eos-contracts](https://github.com/bifrost-codes/bifrost-eos-contracts) (branch: master)
- [rust-eos](https://github.com/bifrost-codes/rust-eos) (branch: use-rust-secp256k1)

## Bifrost

### 1. Compile
Follow the [readme](https://github.com/bifrost-codes/bifrost/tree/ark-bridge-module) to compile a bifrost node.
```
$ git clone https://github.com/bifrost-codes/bifrost.git
$ git checkout ark-bridge-module
$ cargo build --release
```

### 2. Run
```
$ ./target/release/bifrost-node --dev
```

**Tips**: 
> Write down the bifrost node address for next step.

## EOS

### 1. EOS runtime installation
Follow the instructions to install [eosio](https://developers.eos.io/eosio-home/docs/setting-up-your-environment)

---

### 2. Create a dev wallet

- Follow the instructions to create wallet [Create Development Wallet](https://developers.eos.io/eosio-home/docs/wallets).


**Tips**: 
> for the step 1, use the following command in case you forget the passoword
```
$ cleos wallet create --to-file
```

> The wallet folder will created under ```~/eosio-wallet```. If you forget the password to unclock the wallet, use the commands.
```
$ cat ~/eosio-wallet/default.pass # that will show the password
$ cleos wallet unlock # prompt you input the password
```
---

### 3. Compile and run eos node

#### Prerequisites
- Cmake
- LLVM@4.0
- Rust(better use latest stable rust)
- CDT(Contract Development Toolkit). Follow this tutorial to install [cdt](https://developers.eos.io/eosio-home/docs/installing-the-contract-development-toolkit).

#### Compile

```
$ git clone -b bridge-plugin https://github.com/bifrost-codes/eos
$ git submodule update --init --recursive
$ cd eos/
$ make build && cd build
$ cmake ..
$ make -j4
```

#### Run

The script: **start-producer.sh** && **start-relay.sh**.

Modify the script and find out **BIN_DIR** and **BASE_DIR**, pointer to your EOS project.

- Start block producer.
This node producers blocks.

```
$ ./start-producer.sh
```

- Start a EOS relay node.

This node is responsible for message sending like merkle root verification data, and surely synchronize blocks from block producers.
You have to modify this shell script before start this service.

Find this line, leave that block producer's address here.
```
--p2p-peer-address 127.0.0.1:9876
```

Replace it with bifrost address that you just start in step **Compile and run Bifrost node**
```
--bifrost-node=[bifrost_ip_address]
```

start it.
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
$ git clone https://github.com/bifrost-codes/bifrost-eos-contracts
$ cd bifrost-eos-contracts
$ make build && cd build
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
- Replace all [block_producer_address] as block prodcuer address.

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

Before you send a transaction to Bifrost, check **jim**'s balance.

```
# should print 10000.0000 EOS
$ cleos cleos get currency balance eosio.token jim
```

Now send a transaction.
```
$ cleos -u [block_producer_address] push action eosio.token transfer '["jim", "bifrost", "100.0000 EOS", "alice@bifrost:EOS"]' -p jim@active
```

Check **jim**'s account again.
```
# should print 9900.0000 EOS
$ cleos cleos get currency balance eosio.token jim
```

Go to [polkadot.js.org](https://polkadot.js.org/apps/#/extrinsics), to check whether transaction is sent successfully to Bifrost or not.

Wait about 90 seconds for the transaction is verified. If all go well, you can see a event like the following screencapture.

![prove_action_event](prove_action_event.png)

### Bifrost to EOS

By tool subkey to add related data to Bifrost.

Add EOS node address info and EOS secret key to running Bifrost node.

```
# add eos node address
$ ./target/release/subkey EOS_NODE_URL http://127.0.0.1:8888/

# add eos node secret key(public key: EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV)
$ ./target/release/subkey EOS_SECRET_KEY 5KQwrPbwdL6PhXujxW37FSSQZ1JiwsST4cqQzDeyXtP79zkvFD3
```

Send a transaction to EOS node.

In present, we cannot trigger a transaction on [polkadot.js.org](https://polkadot.js.org/apps/#/extrinsics), so we have to
send a transaction by rpc, that will trigger a transaction from Bifrost node to EOS node.

```
# float conversion, 10000 in Bifrost equas to 1.0000 in EOS
$ ./target/release/subkey send_transaction jim 10000
```

Check jim's balance in EOS node.

```
# should print 9901 EOS
$ cleos cleos get currency balance eosio.token jim
```