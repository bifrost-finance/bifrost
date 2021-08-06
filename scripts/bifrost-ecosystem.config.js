module.exports = {
  apps : [{
    name   : "bifrost-live",
    exec_interpreter: "none",
    exec_mode  : "fork_mode",
    script : "/home/bifrost/app/target/release/bifrost",
    args   : "--collator --execution=wasm --keystore-path /home/bifrost/app/data/keystore --chain=/home/bifrost/app/data/bifrost.json --force-authoring --parachain-id 2001 --rpc-cors=all --unsafe-ws-external --unsafe-rpc-external -- --execution=wasm --chain /home/bifrost/app/data/kusama.json"
  }],

  deploy : {
    production : {
      "user" : "bifrost",
      "host" : ["192.168.0.13", "192.168.0.14", "192.168.0.15"],
      "key": "~/.ssh/deploy_rsa.pub",
      "ref"  : "origin/develop",
      "repo" : "git@github.com/bifrost-finance/bifrost.git",
      "path" : "/home/bifrost/app",
      "post-setup": "make build-bifrost-release",
      'post-deploy' : 'pm2 reload scripts/bifost-ecosystem.config.js --env production'
    }
  }
};
