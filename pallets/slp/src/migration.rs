use super::{Config, MinimumsAndMaximums, Weight};
use crate::{BalanceOf, Decode, Encode, MinimumsMaximums, RuntimeDebug, TypeInfo};
use frame_support::traits::Get;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct DeprecatedMinimumsMaximums<Balance> {
	/// The minimum bonded amount for a delegator at any time.
	#[codec(compact)]
	pub delegator_bonded_minimum: Balance,
	/// The minimum amount each time a delegator needs to bond for extra
	#[codec(compact)]
	pub bond_extra_minimum: Balance,
	/// The minimum unbond amount each time a delegator to unbond.
	#[codec(compact)]
	pub unbond_minimum: Balance,
	/// The minimum amount each time a delegator needs to rebond
	#[codec(compact)]
	pub rebond_minimum: Balance,
	/// The maximum number of unbond records at the same time.
	#[codec(compact)]
	pub unbond_record_maximum: u32,
	/// The maximum number of validators for a delegator to support at the same time.
	#[codec(compact)]
	pub validators_back_maximum: u32,
	/// The maximum amount of active staking for a delegator. It is used to control ROI.
	#[codec(compact)]
	pub delegator_active_staking_maximum: Balance,
}

pub fn update_minimums_maximums<T: Config>() -> Weight {
	MinimumsAndMaximums::<T>::translate::<DeprecatedMinimumsMaximums<BalanceOf<T>>, _>(
		|_currency_id, mins_maxs| {
			let new_entry = MinimumsMaximums::<BalanceOf<T>> {
				delegator_bonded_minimum: mins_maxs.delegator_bonded_minimum,
				bond_extra_minimum: mins_maxs.bond_extra_minimum,
				unbond_minimum: mins_maxs.unbond_minimum,
				rebond_minimum: mins_maxs.rebond_minimum,
				unbond_record_maximum: mins_maxs.unbond_record_maximum,
				validators_back_maximum: mins_maxs.validators_back_maximum,
				delegator_active_staking_maximum: mins_maxs.delegator_active_staking_maximum,
				validators_reward_maximum: 0,
				delegation_amount_minimum: 0,
			};
			Some(new_entry)
		},
	);

	T::DbWeight::get().reads(1) + T::DbWeight::get().writes(1)
}
