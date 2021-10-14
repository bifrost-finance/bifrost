# 流动性挖矿模块

###### 使用其他语言阅读：[English](./README.md) | 简体中文

流动性挖矿模块主要提供:
- 矿池的`创建/维护`及`生命周期管理`;
- 挖矿功能, 储户可以用`质押(deposit)`/`赎回(redeem)`/`领取(claim)`等行为进行挖矿;
- 储户质押资产的维护;

__注意__: `创建矿池(create_*_pool)`/`终结矿池(kill_pool)`/`强停矿池(force_retire_pool)`等危险操作的权限可以通过
`Config::ControlOrigin`进行配置.

## 流程

![flow](./img/liquidity-mining-flow@2x.png)

在图中:
1. `council`即为授权账户(`Config::ControlOrigin`), 拥有`创建矿池(create_*_pool)`/`终结矿池(kill_pool)`/`强停矿池(force_retire_pool)`的权限;
2. `user`指一般账户, 可以参与挖矿;
3. 蓝色椭圆框指`模块函数`, 圆形指`参与者`, 数据库图形指`矿池`;

### 一般流程描述(矿池端)
1. 授权账户创建矿池(`create_*_pool`), 刚创建的矿池处于`Uncharged`状态, 需要有人充值设置的奖励;
   1. 若想要删除矿池, 可以终结(`kill_pool`)处于`Uncharged`状态的矿池, 然后重新创建;
2. 矿池的参与方对上述刚创建的矿池进行充值(`charge`), 此时矿池转变状态为`Charged`, 此时矿池接受储户的质押(`deposit`)操作;
   1. 若此时想要删除矿池, 可以强停(`force_retire_pool`)处于`Charged`状态的矿池(充值的资金会退回给参与者);
3. 当处于`Charged`的矿池满足条件(在创建矿池时设置的), 会自动转变状态为`Ongoing`, 此时矿池接受储户的赎回(`redeem`)/领取(`claim`)操作;
4. 当处于`Ongoing`的矿池到生命尽头, 会自动转变状态为`Retired`, 此时矿池只接受储户的赎回(`redeem`)操作;
   1. 若要提前`retire`掉处于`Ongoing`的矿池, 可以调用强停(`force_retire_pool`)操作, 矿池状态转换为`Retired`;
5. 当所有质押资金从矿池中赎回(`redeem`)后, 矿池将被自动删除;

### 一般流程描述(储户端)
1. 当矿池处于`Charged`或`Ongoing`状态时, 储户可以往里质押(`deposit`)矿池指定的通证, 参与挖矿;
   1. __注意__: 当矿池处于`Ongoing`状态时, 储户每次质押(`deposit`)都会领取其未领取的奖励;
2. 当矿池处于`Ongoing`状态时, 储户可以:
   1. claim: 在不动储户所质押的通证的前提下, 领取奖励;
   2. redeem_*: 赎回部分(`redeem`)或全部(`redeem_all`)质押通证, 并领取奖励;
3. 当矿池处于`Retired`状态时, 储户只能进行赎回(`redeem_*`)操作, 赎回所有质押的通证以及奖励;

## 函数

## 矿池

1. `Mining`矿池: 只接受`LpToken`通证作为质押, 质押时对应的通证会转移到矿池的保管者账户(模块账户)中;
2. `Farming`矿池: 只接受free状态下的1:1的`vsToken`与`vsBond`通证作为质押, 质押时对应的通证会转移到矿池的保管者账户(模块账户)中;
3. `Early-Bird-Farming`矿池: 只接受reserved状态下的1:1的`vsToken`与`vsBond`通证作为质押, 质押时只进行记账, 通证仍然保留在储户的账户下;

## 奖励计算方式

### TODO: 术语

### TODO: 举例

### TODO: 图表