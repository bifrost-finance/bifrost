pub use crate::*;
use orml_traits::arithmetic::Zero;
use sp_core::U256;

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct UserMarkupInfo {
	// pub old_locked: LockedBalance<Balance, BlockNumber>,
	pub old_markup_coefficient: FixedU128,
	pub markup_coefficient: FixedU128,
}

pub trait MarkupInfo<AccountId> {
	fn update_markup_info(
		addr: &AccountId,
		new_markup_coefficient: FixedU128,
		user_markup_info: &mut UserMarkupInfo,
	);
}

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

impl<CurrencyId, Balance, BlockNumber, AccountId> Default
	for IncentiveConfig<CurrencyId, Balance, BlockNumber, AccountId>
where
	CurrencyId: Default,
	Balance: Default,
	BlockNumber: Default,
{
	fn default() -> Self {
		IncentiveConfig {
			reward_rate: Default::default(),
			reward_per_token_stored: Default::default(),
			rewards_duration: Default::default(),
			period_finish: Default::default(),
			last_update_time: Default::default(),
			incentive_controller: None,
			last_reward: Default::default(),
		}
	}
}

impl<AccountId, CurrencyId, Balance, BlockNumber>
	VeMintingInterface<AccountId, CurrencyId, Balance, BlockNumber> for ()
where
	Balance: orml_traits::arithmetic::Zero,
{
	fn create_lock_inner(
		_who: &AccountId,
		_value: Balance,
		_unlock_time: BlockNumber,
	) -> DispatchResult {
		Ok(())
	}

	fn increase_unlock_time_inner(
		_who: &AccountId,
		_position: u128,
		_unlock_time: BlockNumber,
	) -> DispatchResult {
		Ok(())
	}

	fn increase_amount_inner(_who: &AccountId, _position: u128, _value: Balance) -> DispatchResult {
		Ok(())
	}

	fn deposit_for(_who: &AccountId, _position: u128, _value: Balance) -> DispatchResult {
		Ok(())
	}

	fn withdraw_inner(_who: &AccountId, _position: u128) -> DispatchResult {
		Ok(())
	}

	fn balance_of(_addr: &AccountId, _time: Option<BlockNumber>) -> Result<Balance, DispatchError> {
		Ok(Zero::zero())
	}

	fn find_block_epoch(_block: BlockNumber, _max_epoch: U256) -> U256 {
		U256::zero()
	}

	fn total_supply(_t: BlockNumber) -> Result<Balance, DispatchError> {
		Ok(Zero::zero())
	}

	fn supply_at(
		_point: Point<Balance, BlockNumber>,
		_t: BlockNumber,
	) -> Result<Balance, DispatchError> {
		Ok(Zero::zero())
	}

	fn auto_notify_reward(
		_pool_id: PoolId,
		_n: BlockNumber,
		_rewards: Vec<(CurrencyId, Balance)>,
	) -> DispatchResult {
		Ok(())
	}

	fn update_reward(
		_pool_id: PoolId,
		_addr: Option<&AccountId>,
		_share_info: Option<(Balance, Balance)>,
	) -> DispatchResult {
		Ok(())
	}

	fn get_rewards(
		_pool_id: PoolId,
		_addr: &AccountId,
		_share_info: Option<(Balance, Balance)>,
	) -> DispatchResult {
		Ok(())
	}

	fn set_incentive(
		_pool_id: PoolId,
		_rewards_duration: Option<BlockNumber>,
		_controller: Option<AccountId>,
	) {
	}
	fn add_reward(
		_addr: &AccountId,
		_conf: &mut IncentiveConfig<CurrencyId, Balance, BlockNumber, AccountId>,
		_rewards: &Vec<(CurrencyId, Balance)>,
		_remaining: Balance,
	) -> DispatchResult {
		Ok(())
	}
	fn notify_reward(
		_pool_id: PoolId,
		_addr: &Option<AccountId>,
		_rewards: Vec<(CurrencyId, Balance)>,
	) -> DispatchResult {
		Ok(())
	}
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct IncentiveConfig<CurrencyId, Balance, BlockNumber, AccountId> {
	pub reward_rate: BTreeMap<CurrencyId, Balance>,
	pub reward_per_token_stored: BTreeMap<CurrencyId, Balance>,
	pub rewards_duration: BlockNumber,
	pub period_finish: BlockNumber,
	pub last_update_time: BlockNumber,
	pub incentive_controller: Option<AccountId>,
	pub last_reward: Vec<(CurrencyId, Balance)>,
}
