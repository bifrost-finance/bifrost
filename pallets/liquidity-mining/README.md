# Liquidity-Mining Pallet

###### Read this in other languages: English | [简体中文](./README_zh-CN.md)

The functionalities the pallet provides:
- creating, maintaining liquidity-pools, and managing the pool's lifecycle;
- users are permitted to do operations like `deposit to`/`redeem from`/`claim from` a liquidity-pool;
- maintaining the tokens the users have deposited;

__NOTE__: Permission to perform dangerous operations such as `create_*_pool`/`kill_pool`/`force_retire_pool`
can be set by `Config::ControlOrigin`.

## FLOW

![flow](./img/liquidity-mining-flow@2x.png)

In the graph above:
1. `council`, the authorized account, who are permitted to do operations such as `create_*_pool`/`kill_pool`/`force_retire_pool`;
2. `user` is the general account who are permitted to mining in a pool;
3. The blue box with ellipse corner refers to extrinsic of the pallet, the DB graph refers to the liquidity-pool;

### General Flow(The Pool)
1. The authorized account calls `create_*_pool` to create a liquidity-pool, which is at `Uncharged` state initially, 
must be charged before users are permitted to do mining operations.
   1. Want to delete the pool? Call `kill_pool` to kill the pool which is at `Uncharged` state, then recreate a new one;
2. Someone charges the pool has created above, which state will transform to `Charged`; Meanwhile, users are permitted to 
do `deposit` operation on the pool;
   1. Want to delete the pool?, Call `force_retire_pool` to retire the pool which is at `Charged`;
3. The moment the pool at `Charged` state meets the condition set when created, will transforms to `Ongoing`;
Meanwhile, users are permitted to do `deposit`/`redeem`/`claim` operations on the pool;
4. The pool will transform to `Retired` when it reaches the end of life, at the time, users are only permitted to do
`redeem` operation on the pool;
   1. Want to retire the ongoing-pool in advance? Call `force_retire_pool` to transform the state of it to `Retired`
   forcefully;
5. The pool will be deleted automatically when the deposit of it becomes zero;

### General Flow(The User)
1. When the pool is at `Charge` or `Ongoing` state, users are permitted to `deposit` tokens to it to participate in mining;
    1. __NOTE__: When the pool is at `Ongoing` state, the user will take the deserved rewards when deposit everytime;
2. When the pool is at `Ongoing` state, users are permitted to do:
    1. claim: withdraw the rewards but not redeem the tokens from the pool;
    2. redeem_*: redeem some(`redeem`) or all(`redeem_all`) tokens and withdraw the deserved rewards from the pool;
3. When the pool is at `Retired` state, 储户只能进行赎回(`redeem_*`)操作, 赎回所有质押的通证以及奖励;

## The Pools

1. `Mining`: Only accept depositing `LpToken`, the tokens will transfer to the pool keeper(module account) when do depositing;
2. `Farming`: Only accept depositing __free__ 1:1 `vsToken` and `vsBond`, the tokens will transfer to the pool keeper(module account) when do depositing;
3. `Early-Bird-Farming`: Only accept depositing __reserved__ 1:1 `vsToken` and `vsBond`, the tokens will still stay under the user account;

## The Method To Calculate Reward

### Terminology

- __PER_BLOCK__: The amount of reward releases per block;
  - If the `total` reward is 10_000, `duration` is 100 block, then `per_block = total/duration = 100`;
- __RDB(REWARD_PER_DEPOSIT_PER_BLOCK)__: reward gain per unit deposit per block;
    - If `per_block` is 100, the `deposit` of pool is 100, then `RDB = per_block / deposit = 1`;
- __RD(REWARD_PER_DEPOSIT)__: reward gain per unit deposit from the start of pool to now;
    - If `RDB` is 1 and will be no change in 20 blocks, then `RD = 0` when the block is 0,
  `RD = 5` when the block is 5 and so on..;

### Example

Let the chain block height is 0, and the pool A has been created and charged, the `total` reward is 10_000,
the `duration` is 100 blocks;

1. When the block is 0, `Alice`, the user, deposits 100 tokens to pool A;
2. When the block is 10, `Bob`, the user, deposits 100 tokens to pool A;
3. When the block is 15, `Alice` claims from pool A, withdraw the deserved reward;
4. When the block is 20, `Alice` redeems  all tokens from pool A, withdraw the deserved reward;

Then, the relationships of `BLOCK`, `RDB`, `RD`, `RD_ALICE`, `RD_BOB`, `REWARD_ALICE`, `REWARD_BOB` look like the following table:

| BLOCK | RDB | RD   | RD_ALICE | RD_BOB | REWARD_ALICE | REWARD_BOB |
| ----- | --- | ---- | -------- | ------ | ------------ | ---------- |
| 0     | 1   | 0    | 0        | none   | 0            | none       |
| 1     | 1   | 1    | 0        | none   | 100          | none       |
| 2     | 1   | 2    | 0        | none   | 200          | none       |
| 3     | 1   | 3    | 0        | none   | 300          | none       |
| 4     | 1   | 4    | 0        | none   | 400          | none       |
| 5     | 1   | 5    | 0        | none   | 500          | none       |
| 6     | 1   | 6    | 0        | none   | 600          | none       |
| 7     | 1   | 7    | 0        | none   | 700          | none       |
| 8     | 1   | 8    | 0        | none   | 800          | none       |
| 9     | 1   | 9    | 0        | none   | 900          | none       |
| 10    | 0.5 | 10   | 0        | 10     | 1000         | 0          |
| 11    | 0.5 | 10.5 | 0        | 10     | 1050         | 50         |
| 12    | 0.5 | 11   | 0        | 10     | 1100         | 100        |
| 13    | 0.5 | 11.5 | 0        | 10     | 1150         | 150        |
| 14    | 0.5 | 12   | 0        | 10     | 1200         | 200        |
| 15    | 0.5 | 12.5 | 12.5     | 10     | 0            | 250        |
| 16    | 0.5 | 13   | 12.5     | 10     | 50           | 300        |
| 17    | 0.5 | 13.5 | 12.5     | 10     | 100          | 350        |
| 18    | 0.5 | 14   | 12.5     | 10     | 150          | 400        |
| 19    | 0.5 | 14.5 | 12.5     | 10     | 200          | 450        |
| 20    | 1   | 15   | none     | 10     | none         | 500        |
| 21    | 1   | 16   | none     | 10     | none         | 600        |
| 22    | 1   | 17   | none     | 10     | none         | 700        |
| 23    | 1   | 18   | none     | 10     | none         | 800        |
| ..    | .   | ..   | .......  | ..     | ....         | ....       |
| 100   | 1   | 95   | none     | 10     | none         | 8500       |

When the chain block height is 100, pool A will be `Retired,` the amount of reward be given to the users is equal to:

__reward_given = 1250(alice claims) + 250(alice redeem) + 8500(bob owns) = 10_000__

__NOTE__:
- `RD_ALICE`, `RD_BOB`: refers to the value of `RD` of pool A when the users do `deposit`/`redeem`/`claim` operations on it;
- `REWARD_ALICE`, `REWARD_BOB`: refers the deserved reward of the user;