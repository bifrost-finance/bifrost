../../target/debug/subkey insert --key-type acco --suri //Alice --chain dev --base-path /tmp/alice
../../target/debug/subkey insert --key-type acco --suri //Bob --chain dev --base-path /tmp/bob 

# IOST node address
../../target/debug/subkey localstorage-set --key IOST_NODE_URL --value http://127.0.0.1:30001 http://127.0.0.1:1234
../../target/debug/subkey localstorage-set --key IOST_NODE_URL --value http://127.0.0.1:30001 http://127.0.0.1:4321

# IOST account name
../../target/debug/subkey localstorage-set --key IOST_ACCOUNT_NAME --value bifrost http://127.0.0.1:1234
../../target/debug/subkey localstorage-set --key IOST_ACCOUNT_NAME --value bifrost http://127.0.0.1:4321

# IOST accounts for Multisignature
../../target/debug/subkey localstorage-set --key IOST_SECRET_KEY --value 3mjUtYRvNnoQbxSb6zM86NCM8KqUeMgV7YLJFvtiHrjZ http://127.0.0.1:1234 # testa
../../target/debug/subkey localstorage-set --key IOST_SECRET_KEY --value 3mjUtYRvNnoQbxSb6zM86NCM8KqUeMgV7YLJFvtiHrjZ http://127.0.0.1:4321 # testb


# IOST cross account signature algorithm
../../target/debug/subkey localstorage-set --key IOST_ACCOUNT_SIG_ALOG --value SECP256K1 http://127.0.0.1:1234 # testa
../../target/debug/subkey localstorage-set --key IOST_ACCOUNT_SIG_ALOG --value SECP256K1 http://127.0.0.1:4321 # testb
