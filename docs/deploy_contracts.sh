cleos -u http://[block_producer_address]:8888 create account eosio eosio.token EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV

# create a account for transaction
cleos -u http://[block_producer_address]:8888 create account eosio jim EOS5gQZcp6e89AgKPxtBC8DPiTSZ7NTuiip9YgycwYrdDRCpEwvTc

# deploy eosio contract
cleos -u http://[block_producer_address]:8888 set contract eosio.token [eos_project_path]/unittests/contracts/eosio.token/ eosio.token.wasm eosio.token.abi -p eosio.token@active

# create bifrost contract account
cleos -u http://[block_producer_address]:8888 create account eosio bifrost EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV

# deploy bofrost contract
cleos -u http://[block_producer_address]:8888 set contract bifrost [bifrost-eos-contracts_project_path]/build/contracts/bifrost.bridge bifrost.bridge.wasm bifrost.bridge.abi -p bifrost@active

# create token
cleos -u http://[block_producer_address]:8888 push action eosio.token create '{"issuer":"eosio", "maximum_supply":"10000.0000 EOS"}' -p eosio.token@active

# issue token
cleos -u http://[block_producer_address]:8888 push action eosio.token issue '["jim", "10000.0000 EOS", "memo"]' -p eosio@active

# register token just created
cleos -u http://[block_producer_address]:8888 push action bifrost regtoken '["eosio.token", "4,EOS", "10000.0000 EOS", "1.0000 EOS", "1000.0000 EOS", "10000.0000 EOS", "1"]' -p bifrost@active