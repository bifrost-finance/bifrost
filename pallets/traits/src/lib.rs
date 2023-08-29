#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{dispatch::DispatchError, traits::tokens::Balance as BalanceT};
use num_bigint::{BigUint, ToBigUint};
use scale_info::TypeInfo;
use sp_runtime::{traits::Zero, RuntimeDebug};
use sp_std::prelude::*;

use primitives::{CurrencyId, DerivativeIndex, PriceDetail, Rate, Timestamp};

pub mod loans;
pub mod ump;
// pub mod xcm;
pub use loans::*;

pub trait EmergencyCallFilter<Call> {
	fn contains(call: &Call) -> bool;
}

pub trait PriceFeeder {
	fn get_price(asset_id: &CurrencyId) -> Option<PriceDetail>;
}

pub trait DecimalProvider<CurrencyId> {
	fn get_decimal(asset_id: &CurrencyId) -> Option<u8>;
}

pub trait EmergencyPriceFeeder<CurrencyId, Price> {
	fn set_emergency_price(asset_id: CurrencyId, price: Price);
	fn reset_emergency_price(asset_id: CurrencyId);
}

pub trait ExchangeRateProvider<CurrencyId> {
	fn get_exchange_rate(asset_id: &CurrencyId) -> Option<Rate>;
}

pub trait LiquidStakingConvert<Balance> {
	fn staking_to_liquid(amount: Balance) -> Option<Balance>;
	fn liquid_to_staking(liquid_amount: Balance) -> Option<Balance>;
}

pub trait LiquidStakingCurrenciesProvider<CurrencyId> {
	fn get_staking_currency() -> Option<CurrencyId>;
	fn get_liquid_currency() -> Option<CurrencyId>;
}

pub trait VaultTokenExchangeRateProvider<CurrencyId> {
	fn get_exchange_rate(asset_id: &CurrencyId, init_rate: Rate) -> Option<Rate>;
}

pub trait LPVaultTokenExchangeRateProvider<CurrencyId> {
	fn get_exchange_rate(lp_asset_id: &CurrencyId) -> Option<Rate>;
}

pub trait VaultTokenCurrenciesFilter<CurrencyId> {
	fn contains(asset_id: &CurrencyId) -> bool;
}

pub trait LPVaultTokenCurrenciesFilter<CurrencyId> {
	fn contains(lp_asset_id: &CurrencyId) -> bool;
}

#[derive(
	Encode,
	Decode,
	Eq,
	PartialEq,
	Copy,
	Clone,
	RuntimeDebug,
	PartialOrd,
	Ord,
	TypeInfo,
	MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Pool<CurrencyId, Balance, BlockNumber> {
	pub base_amount: Balance,
	pub quote_amount: Balance,
	pub base_amount_last: Balance,
	pub quote_amount_last: Balance,
	pub lp_token_id: CurrencyId,
	pub block_timestamp_last: BlockNumber,
	pub price_0_cumulative_last: Balance,
	pub price_1_cumulative_last: Balance,
}

impl<CurrencyId, Balance: BalanceT, BlockNumber: BalanceT> Pool<CurrencyId, Balance, BlockNumber> {
	pub fn new(lp_token_id: CurrencyId) -> Self {
		Self {
			base_amount: Zero::zero(),
			quote_amount: Zero::zero(),
			base_amount_last: Zero::zero(),
			quote_amount_last: Zero::zero(),
			lp_token_id,
			block_timestamp_last: Zero::zero(),
			price_0_cumulative_last: Zero::zero(),
			price_1_cumulative_last: Zero::zero(),
		}
	}

	pub fn is_empty(&self) -> bool {
		self.base_amount.is_zero() && self.quote_amount.is_zero()
	}
}

/// Exported traits from our AMM pallet. These functions are to be used
/// by the router to enable multi route token swaps
pub trait AMM<AccountId, CurrencyId, Balance, BlockNumber> {
	/// Based on the path specified and the available pool balances
	/// this will return the amounts outs when trading the specified
	/// amount in
	fn get_amounts_out(
		amount_in: Balance,
		path: Vec<CurrencyId>,
	) -> Result<Vec<Balance>, DispatchError>;

	/// Based on the path specified and the available pool balances
	/// this will return the amounts in needed to produce the specified
	/// amount out
	fn get_amounts_in(
		amount_out: Balance,
		path: Vec<CurrencyId>,
	) -> Result<Vec<Balance>, DispatchError>;

	/// Handles a "swap" on the AMM side for "who".
	/// This will move the `amount_in` funds to the AMM PalletId,
	/// trade `pair.0` to `pair.1` and return a result with the amount
	/// of currency that was sent back to the user.
	fn swap(
		who: &AccountId,
		pair: (CurrencyId, CurrencyId),
		amount_in: Balance,
	) -> Result<(), DispatchError>;

	/// Iterate keys of asset pair in AMM Pools
	fn get_pools() -> Result<Vec<(CurrencyId, CurrencyId)>, DispatchError>;

	///  Returns pool by lp_asset
	fn get_pool_by_lp_asset(
		asset_id: CurrencyId,
	) -> Option<(CurrencyId, CurrencyId, Pool<CurrencyId, Balance, BlockNumber>)>;

	/// Returns pool by asset pair
	fn get_pool_by_asset_pair(
		pair: (CurrencyId, CurrencyId),
	) -> Option<Pool<CurrencyId, Balance, BlockNumber>>;
}

/// Exported traits from StableSwap pallet. These functions are to be used
/// by the router.
pub trait StableSwap<AccountId, CurrencyId, Balance> {
	/// Based on the path specified and the available pool balances
	/// this will return the amounts outs when trading the specified
	/// amount in
	fn get_amounts_out(
		amount_in: Balance,
		path: Vec<CurrencyId>,
	) -> Result<Vec<Balance>, DispatchError>;

	/// Based on the path specified and the available pool balances
	/// this will return the amounts in needed to produce the specified
	/// amount out
	fn get_amounts_in(
		amount_out: Balance,
		path: Vec<CurrencyId>,
	) -> Result<Vec<Balance>, DispatchError>;

	/// Handles a "swap" on the AMM side for "who".
	/// This will move the `amount_in` funds to the AMM PalletId,
	/// trade `pair.0` to `pair.1` and return a result with the amount
	/// of currency that was sent back to the user.
	fn swap(
		who: &AccountId,
		pair: (CurrencyId, CurrencyId),
		amount_in: Balance,
	) -> Result<(), DispatchError>;

	fn get_pools() -> Result<Vec<(CurrencyId, CurrencyId)>, DispatchError>;

	fn get_reserves(
		asset_in: CurrencyId,
		asset_out: CurrencyId,
	) -> Result<(Balance, Balance), DispatchError>;
}

pub trait ConvertToBigUint {
	fn get_big_uint(&self) -> BigUint;
}

impl ConvertToBigUint for u128 {
	fn get_big_uint(&self) -> BigUint {
		self.to_biguint().unwrap()
	}
}

/// Get relaychain validation data
// pub trait ValidationDataProvider {
// 	fn validation_data() -> Option<PersistedValidationData>;
// }

/// Distribute liquidstaking asset to multi-accounts
pub trait DistributionStrategy<Balance> {
	fn get_bond_distributions(
		bonded_amounts: Vec<(DerivativeIndex, Balance, Balance)>,
		input: Balance,
		cap: Balance,
		min_nominator_bond: Balance,
	) -> Vec<(DerivativeIndex, Balance)>;
	fn get_unbond_distributions(
		active_bonded_amounts: Vec<(DerivativeIndex, Balance)>,
		input: Balance,
		min_nominator_bond: Balance,
	) -> Vec<(DerivativeIndex, Balance)>;
	fn get_rebond_distributions(
		unbonding_amounts: Vec<(DerivativeIndex, Balance)>,
		input: Balance,
	) -> Vec<(DerivativeIndex, Balance)>;
}

pub trait Streaming<AccountId, CurrencyId, Balance> {
	fn create(
		sender: AccountId,
		recipient: AccountId,
		deposit: Balance,
		asset_id: CurrencyId,
		start_time: Timestamp,
		end_time: Timestamp,
		cancellable: bool,
	) -> Result<(), DispatchError>;
}

impl<AccountId, CurrencyId, Balance> Streaming<AccountId, CurrencyId, Balance> for () {
	fn create(
		_sender: AccountId,
		_recipient: AccountId,
		_deposit: Balance,
		_asset_id: CurrencyId,
		_start_time: Timestamp,
		_end_time: Timestamp,
		_cancellable: bool,
	) -> Result<(), DispatchError> {
		Ok(())
	}
}

pub trait OnExchangeRateChange<CurrencyId> {
	fn on_exchange_rate_change(currency_id: &CurrencyId);
}

#[impl_trait_for_tuples::impl_for_tuples(3)]
impl<CurrencyId> OnExchangeRateChange<CurrencyId> for Tuple {
	fn on_exchange_rate_change(currency_id: &CurrencyId) {
		for_tuples!( #(
            Tuple::on_exchange_rate_change(currency_id);
        )* );
	}
}
