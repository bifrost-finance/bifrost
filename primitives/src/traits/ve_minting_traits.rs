use crate::{IncentiveConfig, Point, PoolId};
use sp_core::U256;
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::vec::Vec;

pub trait VeMintingInterface<AccountId, CurrencyId, Balance, BlockNumber> {
	fn deposit_for(_who: &AccountId, position: u128, value: Balance) -> DispatchResult;
	fn withdraw_inner(who: &AccountId, position: u128) -> DispatchResult;
	fn balance_of(addr: &AccountId, time: Option<BlockNumber>) -> Result<Balance, DispatchError>;
	fn total_supply(t: BlockNumber) -> Result<Balance, DispatchError>;
	fn supply_at(
		point: Point<Balance, BlockNumber>,
		t: BlockNumber,
	) -> Result<Balance, DispatchError>;
	fn find_block_epoch(_block: BlockNumber, max_epoch: U256) -> U256;
	fn create_lock_inner(
		who: &AccountId,
		_value: Balance,
		_unlock_time: BlockNumber,
	) -> DispatchResult; // Deposit `_value` BNC for `addr` and lock until `_unlock_time`
	fn increase_amount_inner(who: &AccountId, position: u128, value: Balance) -> DispatchResult; // Deposit `_value` additional BNC for `addr` without modifying the unlock time
	fn increase_unlock_time_inner(
		who: &AccountId,
		position: u128,
		_unlock_time: BlockNumber,
	) -> DispatchResult; // Extend the unlock time for `addr` to `_unlock_time`
	fn auto_notify_reward(
		pool_id: PoolId,
		n: BlockNumber,
		rewards: Vec<(CurrencyId, Balance)>,
	) -> DispatchResult;
	fn update_reward(
		pool_id: PoolId,
		addr: Option<&AccountId>,
		share_info: Option<(Balance, Balance)>,
	) -> DispatchResult;
	fn get_rewards(
		pool_id: PoolId,
		addr: &AccountId,
		share_info: Option<(Balance, Balance)>,
	) -> DispatchResult;
	fn set_incentive(
		pool_id: PoolId,
		rewards_duration: Option<BlockNumber>,
		controller: Option<AccountId>,
	);
	fn add_reward(
		addr: &AccountId,
		conf: &mut IncentiveConfig<CurrencyId, Balance, BlockNumber, AccountId>,
		rewards: &Vec<(CurrencyId, Balance)>,
		remaining: Balance,
	) -> DispatchResult;
	fn notify_reward(
		pool_id: PoolId,
		addr: &Option<AccountId>,
		rewards: Vec<(CurrencyId, Balance)>,
	) -> DispatchResult;
}
