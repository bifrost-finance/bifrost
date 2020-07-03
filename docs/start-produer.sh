BIN_DIR="[eos_project_path]/build"
BASE_DIR="[eos_project_path]/localnet/node/producer"
CONF_DIR=$BASE_DIR/config
DATA_DIR=$BASE_DIR/data

$BIN_DIR/bin/nodeos --max-transaction-time=1000 \
--enable-stale-production \
--producer-name eosio \
--plugin eosio::chain_api_plugin \
--plugin eosio::net_api_plugin \
--plugin eosio::producer_api_plugin \
--http-server-address=0.0.0.0:8888 \
--p2p-server-address=0.0.0.0:9876 \
--config-dir $CONF_DIR \
--data-dir $DATA_DIR -l $BASE_DIR/logging.json
