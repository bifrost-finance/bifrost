module.exports = {
  apps : [{
    name   : "asgard-local-alice",
    exec_interpreter: "none",
    exec_mode  : "fork_mode",
    script : "./target/release/bifrost",
    args   : "--tmp --execution=wasm --chain=asgard-local --alice --force-authoring --parachain-id 3000 --rpc-cors=all --unsafe-ws-external --unsafe-rpc-external -- --execution=wasm --chain ./rococo.json"
  }]
};
