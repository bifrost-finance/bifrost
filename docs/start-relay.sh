BIN_DIR="[eos_project_path]/build"
BASE_DIR="[eos_project_path]/localnet/node/producer"
CONF_DIR=$BASE_DIR/config
DATA_DIR=$BASE_DIR/data

$BIN_DIR/bin/nodeos --plugin eosio::chain_api_plugin \
--plugin eosio::bridge_plugin \
--plugin eosio::http_plugin \
--http-server-address=127.0.0.1:8889 \
--p2p-listen-endpoint 127.0.0.1:9877 \
--p2p-peer-address 127.0.0.1:9876 \
--config-dir $CONF_DIR \
--data-dir $DATA_DIR -l $BASE_DIR/logging.json \
--bifrost-node=ws://127.0.0.1:9944 \
--bifrost-crossaccount=bifrostcross \
--bifrost-signer=//Alice
