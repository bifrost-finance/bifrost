../target/release/subkey insert //Alice//stash acco http://127.0.0.1:4321/
../target/release/subkey insert //Bob//stash acco http://127.0.0.1:1234/

# EOS node address
../target/release/subkey localstorage-set EOS_NODE_URL http://[eos_producer_ip]:8888/ http://127.0.0.1:1234
../target/release/subkey localstorage-set EOS_NODE_URL http://[eos_producer_ip]:8888/ http://127.0.0.1:4321

# EOS accounts for Multisignature
../target/release/subkey localstorage-set EOS_SECRET_KEY 5JNV39rZLZWr5p1hdLXVVNvJsXpgZnzvTrcZYJggTPuv1GzChB6 http://127.0.0.1:1234 # testa
../target/release/subkey localstorage-set EOS_SECRET_KEY 5KDXMiphWpzETsNpp3eL3sjWAa4gMvMXCtMquT2PDpKtV1STbHp http://127.0.0.1:4321 # testb
