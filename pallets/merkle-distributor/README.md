# Overview
    Merkle-Distributor is used to send rewards to users in batches. 
    Compared with Airdrop, it uses a passive way to distribute rewards.
    We use user data as the leaves of the merkle tree. Anyone can use the 
    data of a certain user and its path in the merkle tree to calculate 
    a merkle tree root.This is to prove the legitimacy of this user and the reward he deserves.

# Usage
- Build a merkle tree 
```
Prepare user data like `script/merkle-distributor/scripts/example.json`.
It's a address-rewardAmount structure. Then run command `generate-merkle-root:example`,
you can see merkle-root and proof of user.
```
- Create a merkle distributor
- Charge
- Claim