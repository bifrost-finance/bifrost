cleos create account eosio eosio.token EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV

# create a account for transaction
cleos create account eosio jim EOS5gQZcp6e89AgKPxtBC8DPiTSZ7NTuiip9YgycwYrdDRCpEwvTc

# accounts for multisignature
cleos create account eosio testa EOS53o1JSZsySAbdQ9LFgH7gx6Mw6eURJCXawoTEomSdcT6672ZTa # 5KDXMiphWpzETsNpp3eL3sjWAa4gMvMXCtMquT2PDpKtV1STbHp
cleos create account eosio testb EOS8VP7UrNknXZ7mqpVteJd7YAnMipHZTiohZETzgWHr8ZCSCUrAx # 5JNV39rZLZWr5p1hdLXVVNvJsXpgZnzvTrcZYJggTPuv1GzChB6
cleos create account eosio testc EOS6BsmRFwBdqPaFB3zeV7gL7D54s6dS61JrHT2rK3Q7KXj2FRChE # 5JxA9GR73U5abo7xps6i5axPQh8zUvkr34rTKAKeat7SHokHUU2
cleos create account eosio testd EOS7cpYZnAWdY9k84Ux4Jvz7tR16yKtAV7Ni38tg8fggwpEuGhzYP # 5Hv6y4aMCLnGtsXwLp3ZgmAhzHTANFduDvwhfvdcc1bnp6Wm1wN

# deploy eosio contract
cleos set contract eosio.token [eos_project_path]/unittests/contracts/eosio.token/ eosio.token.wasm eosio.token.abi -p eosio.token@active

# create bifrost contract account
cleos create account eosio bifrost EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV

# deploy bofrost contract
cleos set contract bifrost [bifrost-eos-contracts_project_path]/build/contracts/bifrost.bridge bifrost.bridge.wasm bifrost.bridge.abi -p bifrost@active

# create token
cleos push action eosio.token create '{"issuer":"eosio", "maximum_supply":"10000.0000 EOS"}' -p eosio.token@active

# issue token
cleos push action eosio.token issue '["jim", "10000.0000 EOS", "memo"]' -p eosio@active

# register token just created
cleos push action bifrost regtoken '["eosio.token", "4,EOS", "10000.0000 EOS", "1.0000 EOS", "1000.0000 EOS", "10000.0000 EOS", "1"]' -p bifrost@active