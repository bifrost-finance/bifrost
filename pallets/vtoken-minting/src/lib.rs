// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod impls;
pub mod migration;
pub mod traits;
pub mod weights;
pub use weights::WeightInfo;

use crate::impls::Operation;
use bb_bnc::traits::BbBNCInterface;
use bifrost_primitives::{
	CurrencyId, RedeemType, SlpxOperator, TimeUnit, VTokenMintRedeemProvider,
};
use frame_support::{
	pallet_prelude::{DispatchResultWithPostInfo, *},
	sp_runtime::{
		traits::{CheckedAdd, CheckedSub, Saturating, Zero},
		DispatchError, Permill,
	},
	traits::LockIdentifier,
	BoundedVec, PalletId,
};
use frame_system::pallet_prelude::*;
use log;
use orml_traits::{MultiCurrency, MultiLockableCurrency, XcmTransfer};
pub use pallet::*;
use sp_std::vec;
pub use traits::*;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;
pub type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

pub type UnlockId = u32;

// incentive lock id for vtoken minted by user
const INCENTIVE_LOCK_ID: LockIdentifier = *b"vmincntv";

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Set default weight.
		type WeightInfo: WeightInfo;
		/// The only origin that can edit token issuer list
		type ControlOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		/// The multi currency trait.
		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiLockableCurrency<AccountIdOf<Self>>;
		/// Handler to notify the runtime when redeem success
		/// If you don't need it, you can specify the type `()`.
		type OnRedeemSuccess: OnRedeemSuccess<
			AccountIdOf<Self>,
			CurrencyIdOf<Self>,
			BalanceOf<Self>,
		>;
		/// Xtokens xcm transfer interface
		type XcmTransfer: XcmTransfer<AccountIdOf<Self>, BalanceOf<Self>, CurrencyIdOf<Self>>;
		/// Slpx operator
		type BifrostSlpx: SlpxOperator<BalanceOf<Self>>;
		/// bbBNC interface
		type BbBNC: BbBNCInterface<
			AccountIdOf<Self>,
			CurrencyIdOf<Self>,
			BalanceOf<Self>,
			BlockNumberFor<Self>,
		>;
		/// Channel commission provider
		type ChannelCommission: VTokenMintRedeemProvider<CurrencyId, BalanceOf<Self>>;

		/// Maximum unlock id of user
		#[pallet::constant]
		type MaximumUnlockIdOfUser: Get<u32>;
		/// Maximum unlock id of time unit
		#[pallet::constant]
		type MaximumUnlockIdOfTimeUnit: Get<u32>;
		/// Maximum unlocked vtoken records minted in an incentive mode
		#[pallet::constant]
		type MaxLockRecords: Get<u32>;
		/// Currency receive account
		#[pallet::constant]
		type EntranceAccount: Get<PalletId>;
		/// Currency exit account
		#[pallet::constant]
		type ExitAccount: Get<PalletId>;
		/// Fee account
		#[pallet::constant]
		type FeeAccount: Get<Self::AccountId>;
		/// Redeem fee account
		#[pallet::constant]
		type RedeemFeeAccount: Get<Self::AccountId>;

		#[pallet::constant]
		type IncentivePoolAccount: Get<PalletId>;

		#[pallet::constant]
		type RelayChainToken: Get<CurrencyId>;

		#[pallet::constant]
		type MoonbeamChainId: Get<u32>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Vtoken minted successfully.
		Minted {
			/// The minter account.
			minter: AccountIdOf<T>,
			/// The currency id minted.
			currency_id: CurrencyIdOf<T>,
			/// The currency amount minted.
			currency_amount: BalanceOf<T>,
			/// The v_currency amount minted.
			v_currency_amount: BalanceOf<T>,
			/// The mint fee.
			mint_fee: BalanceOf<T>,
			/// The remark of minting.
			remark: BoundedVec<u8, ConstU32<32>>,
			/// The channel id of minting.
			channel_id: Option<u32>,
		},
		///	Vtoken redeemed successfully.
		Redeemed {
			/// The redeemer account.
			redeemer: AccountIdOf<T>,
			/// The currency id redeemed.
			currency_id: CurrencyIdOf<T>,
			/// Will be received currency amount.
			currency_amount: BalanceOf<T>,
			/// The v_currency amount redeemed.
			v_currency_amount: BalanceOf<T>,
			/// The redeem fee.
			redeem_fee: BalanceOf<T>,
			/// The unlock_id of redeeming.
			unlock_id: UnlockId,
		},
		/// Process redeem successfully.
		RedeemSuccess {
			/// The redeemer account.
			redeemer: AccountIdOf<T>,
			/// The unlock_id redeemed.
			unlock_id: UnlockId,
			/// The currency id redeemed.
			currency_id: CurrencyIdOf<T>,
			/// Will transfer to this account.
			to: RedeemTo<AccountIdOf<T>>,
			/// The redeem amount.
			currency_amount: BalanceOf<T>,
		},
		/// Vtoken rebonded successfully.
		Rebonded {
			/// The rebonder account.
			rebonder: AccountIdOf<T>,
			/// The currency id rebonded.
			currency_id: CurrencyIdOf<T>,
			/// The currency amount rebonded.
			currency_amount: BalanceOf<T>,
			/// The v_currency amount rebonded.
			v_currency_amount: BalanceOf<T>,
			/// Mint fee
			fee: BalanceOf<T>,
		},
		/// Vtoken rebonded by unlock_id successfully.
		RebondedByUnlockId {
			/// The rebonder account.
			rebonder: AccountIdOf<T>,
			/// The currency id rebonded.
			currency_id: CurrencyIdOf<T>,
			/// The currency amount rebonded.
			currency_amount: BalanceOf<T>,
			/// The v_currency amount rebonded.
			v_currency_amount: BalanceOf<T>,
			/// Mint fee
			fee: BalanceOf<T>,
			/// The unlock_id rebonded.
			unlock_id: UnlockId,
		},
		/// Set unlock duration.
		UnlockDurationSet {
			/// The currency id set unlock duration.
			currency_id: CurrencyIdOf<T>,
			/// The unlock duration set.
			unlock_duration: TimeUnit,
		},
		/// Set minimum mint amount.
		MinimumMintSet {
			/// The currency id set minimum mint amount.
			currency_id: CurrencyIdOf<T>,
			/// The minimum mint amount set.
			minimum_amount: BalanceOf<T>,
		},
		/// Set minimum redeem amount.
		MinimumRedeemSet {
			/// The currency id set minimum redeem amount.
			currency_id: CurrencyIdOf<T>,
			/// The minimum redeem amount set.
			minimum_amount: BalanceOf<T>,
		},
		/// Support rebond token added.
		SupportRebondTokenAdded {
			/// The currency id support rebond.
			currency_id: CurrencyIdOf<T>,
		},
		/// Support rebond token removed.
		SupportRebondTokenRemoved {
			/// The currency id remove support rebond.
			currency_id: CurrencyIdOf<T>,
		},
		/// Set mint fee and redeem fee.
		FeeSet {
			/// The mint fee rate set.
			mint_fee: Permill,
			/// The redeem fee rate set.
			redeem_fee: Permill,
		},
		/// Set hook iteration limit.
		HookIterationLimitSet { limit: u32 },
		/// Set unlock total amount.
		UnlockingTotalSet {
			/// The currency id set unlock total amount.
			currency_id: CurrencyIdOf<T>,
			/// The unlock total amount set.
			currency_amount: BalanceOf<T>,
		},
		/// Set minimum time unit.
		MinTimeUnitSet {
			/// The currency id set minimum time unit.
			currency_id: CurrencyIdOf<T>,
			/// The minimum time unit set.
			time_unit: TimeUnit,
		},
		/// Fast redeem failed.
		FastRedeemFailed { err: DispatchError },
		/// Set ongoing time unit.
		SetOngoingTimeUnit {
			/// The currency id set ongoing time unit.
			currency_id: CurrencyIdOf<T>,
			/// The ongoing time unit set.
			time_unit: TimeUnit,
		},
		/// Incentivized minting.
		IncentivizedMinting {
			address: AccountIdOf<T>,
			currency_id: CurrencyIdOf<T>,
			currency_amount: BalanceOf<T>,
			locked_vtoken_amount: BalanceOf<T>,
			incentive_vtoken_amount: BalanceOf<T>,
		},
		/// Incentive coefficient set.
		VtokenIncentiveCoefSet { v_currency_id: CurrencyIdOf<T>, coefficient: Option<u128> },
		/// Incentive lock blocks set.
		VtokenIncentiveLockBlocksSet {
			v_currency_id: CurrencyIdOf<T>,
			blocks: Option<BlockNumberFor<T>>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Below minimum mint amount.
		BelowMinimumMint,
		/// Below minimum redeem amount.
		BelowMinimumRedeem,
		/// Invalid token to rebond.
		InvalidRebondToken,
		/// Token type not support.
		NotSupportTokenType,
		/// Not enough balance to unlock.
		NotEnoughBalanceToUnlock,
		/// Token unlock ledger not found.
		TokenToRebondNotZero,
		/// Ongoing time unit not set.
		OngoingTimeUnitNotSet,
		/// Token unlock ledger not found.
		TokenUnlockLedgerNotFound,
		/// User unlock ledger not found.
		UserUnlockLedgerNotFound,
		/// Time unit unlock ledger not found.
		TimeUnitUnlockLedgerNotFound,
		/// Unlock duration not found.
		UnlockDurationNotFound,
		/// Unexpected error.
		Unexpected,
		/// Calculation overflow.
		CalculationOverflow,
		/// Exceed maximum unlock id.
		ExceedMaximumUnlockId,
		/// Too many redeems.
		TooManyRedeems,
		/// Can not rebond.
		CanNotRebond,
		/// Not enough balance.
		NotEnoughBalance,
		/// veBNC checking error.
		VeBNCCheckingError,
		/// IncentiveCoef not found.
		IncentiveCoefNotFound,
		/// Too many locks.
		TooManyLocks,
		/// No unlock record.
		NoUnlockRecord,
		/// Fail to remove lock.
		FailToRemoveLock,
		/// Balance not zero.
		BalanceZero,
		/// IncentiveLockBlocksNotSet
		IncentiveLockBlocksNotSet,
	}

	/// The mint fee and redeem fee.
	#[pallet::storage]
	pub type Fees<T: Config> = StorageValue<_, (Permill, Permill), ValueQuery>;

	/// Token pool amount
	#[pallet::storage]
	pub type TokenPool<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	/// Unlock duration for each currency
	#[pallet::storage]
	pub type UnlockDuration<T: Config> = StorageMap<_, Twox64Concat, CurrencyIdOf<T>, TimeUnit>;

	/// Ongoing time unit for each currency
	#[pallet::storage]
	pub type OngoingTimeUnit<T: Config> = StorageMap<_, Twox64Concat, CurrencyIdOf<T>, TimeUnit>;

	/// Minimum mint amount for each currency
	#[pallet::storage]
	pub type MinimumMint<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	/// Minimum redeem amount for each currency
	#[pallet::storage]
	pub type MinimumRedeem<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	/// Next unlock id for each currency
	#[pallet::storage]
	pub type TokenUnlockNextId<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, u32, ValueQuery>;

	/// According to currency_id and unlock_id, unlock information are stored.
	#[pallet::storage]
	pub type TokenUnlockLedger<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		Blake2_128Concat,
		UnlockId,
		(
			// redeemer account
			T::AccountId,
			// redeem amount
			BalanceOf<T>,
			// lock to time unit
			TimeUnit,
			// redeem type
			RedeemType<AccountIdOf<T>>,
		),
		OptionQuery,
	>;

	/// According to the user's account, the locked amount and unlock id list are stored.
	#[pallet::storage]
	pub type UserUnlockLedger<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		(
			// Total locked amount
			BalanceOf<T>,
			// UnlockId list
			BoundedVec<UnlockId, T::MaximumUnlockIdOfUser>,
		),
		OptionQuery,
	>;

	/// The total amount of tokens that are currently locked for unlocking.
	#[pallet::storage]
	pub type TimeUnitUnlockLedger<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		TimeUnit,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		(
			// Total locked amount
			BalanceOf<T>,
			// UnlockId list
			BoundedVec<UnlockId, T::MaximumUnlockIdOfTimeUnit>,
			// CurrencyId
			CurrencyIdOf<T>,
		),
		OptionQuery,
	>;

	/// The total amount of tokens that are currently locked for rebonding.
	#[pallet::storage]
	pub type TokenToRebond<T: Config> = StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>>;

	/// The min time unit for each currency
	#[pallet::storage]
	pub type MinTimeUnit<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, TimeUnit, ValueQuery>;

	/// The total amount of tokens that are currently unlocking.
	#[pallet::storage]
	pub type UnlockingTotal<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	/// The hook iteration limit
	#[pallet::storage]
	pub type HookIterationLimit<T: Config> = StorageValue<_, u32, ValueQuery>;

	//【vtoken -> Blocks】, the locked blocks for each vtoken when minted in an incentive mode
	#[pallet::storage]
	pub type MintWithLockBlocks<T: Config> =
		StorageMap<_, Blake2_128Concat, CurrencyId, BlockNumberFor<T>>;

	//【vtoken -> incentive coefficient】,the incentive coefficient for each vtoken when minted in
	// an incentive mode
	#[pallet::storage]
	pub type VtokenIncentiveCoef<T: Config> = StorageMap<_, Blake2_128Concat, CurrencyId, u128>;

	//【user + vtoken -> (total_locked, vec[(locked_amount, due_block_num)])】, the locked vtoken
	// records for each user
	#[pallet::storage]
	pub type VtokenLockLedger<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		Blake2_128Concat,
		CurrencyId,
		(BalanceOf<T>, BoundedVec<(BalanceOf<T>, BlockNumberFor<T>), T::MaxLockRecords>),
		OptionQuery,
	>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			for currency in OngoingTimeUnit::<T>::iter_keys() {
				let result = Self::handle_ledger_by_currency(currency);
				match result {
					Ok(_) => (),
					Err(err) => {
						Self::deposit_event(Event::FastRedeemFailed { err });
						log::error!(
							target: "runtime::vtoken-minting",
							"Received invalid justification for {:?}",
							err,
						);
					},
				}
			}

			T::WeightInfo::on_initialize()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Mint v_currency by transferring currency to entrance_account.
		/// The minted v_currency will be deposited to the minter's account.
		/// Parameters:
		/// - `currency_id`: The currency to mint.
		/// - `currency_amount`: The amount of currency to mint.
		/// - `remark`: The remark of minting.
		/// - `channel_id`: The channel id of minting.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::mint())]
		pub fn mint(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			currency_amount: BalanceOf<T>,
			remark: BoundedVec<u8, ConstU32<32>>,
			channel_id: Option<u32>,
		) -> DispatchResult {
			// Check origin
			let minter = ensure_signed(origin)?;
			Self::do_mint(minter, currency_id, currency_amount, remark, channel_id)?;
			Ok(())
		}

		/// Redeem currency by burning v_currency. But need to wait for the unlock period.
		/// The redeemed currency will be transferred to the redeemer's account.
		/// Parameters:
		/// - `v_currency_id`: The v_currency to redeem.
		/// - `v_currency_amount`: The amount of v_currency to redeem.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::redeem())]
		pub fn redeem(
			origin: OriginFor<T>,
			v_currency_id: CurrencyIdOf<T>,
			v_currency_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let redeemer = ensure_signed(origin)?;
			Self::do_redeem(redeemer, v_currency_id, v_currency_amount, RedeemType::Native)
		}

		/// Already redeemed currency by burning v_currency. But need to wait for the unlock period.
		/// In unlock period, you call rebond to cancel the redeem.
		/// Parameters:
		/// - `currency_id`: The currency to rebond.
		/// - `currency_amount`: The amount of currency to rebond. The amount should be less than or
		///   equal to the redeem amount.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::rebond())]
		pub fn rebond(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			currency_amount: BalanceOf<T>,
		) -> DispatchResult {
			let rebonder = ensure_signed(origin)?;
			let v_currency_id =
				currency_id.to_vtoken().map_err(|_| Error::<T>::NotSupportTokenType)?;

			let (user_unlock_amount, unlock_id_list) =
				UserUnlockLedger::<T>::get(&rebonder, currency_id)
					.ok_or(Error::<T>::UserUnlockLedgerNotFound)?;
			ensure!(user_unlock_amount >= currency_amount, Error::<T>::NotEnoughBalanceToUnlock);

			let mut temp_currency_amount = currency_amount;
			for unlock_id in unlock_id_list.into_iter().rev() {
				let (_, mut unlock_amount, time_unit, _) =
					TokenUnlockLedger::<T>::get(currency_id, unlock_id)
						.ok_or(Error::<T>::TokenUnlockLedgerNotFound)?;

				if temp_currency_amount <= unlock_amount {
					unlock_amount = temp_currency_amount;
				} else {
					temp_currency_amount = temp_currency_amount.saturating_sub(unlock_amount);
				}

				let is_remove_record = Self::update_unlock_ledger(
					&rebonder,
					&currency_id,
					&unlock_amount,
					&unlock_id,
					&time_unit,
					None,
					Operation::Sub,
				)?;

				if !is_remove_record {
					break;
				}
			}

			let (_, v_currency_amount, fee) = Self::mint_without_transfer(
				&rebonder,
				v_currency_id,
				currency_id,
				currency_amount,
			)?;

			TokenToRebond::<T>::mutate(&currency_id, |maybe_value| -> Result<(), Error<T>> {
				match maybe_value {
					Some(rebonded_amount) => {
						*rebonded_amount = rebonded_amount
							.checked_add(&currency_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;
						Ok(())
					},
					None => Err(Error::<T>::InvalidRebondToken),
				}
			})?;

			Self::deposit_event(Event::Rebonded {
				rebonder,
				currency_id,
				currency_amount,
				v_currency_amount,
				fee,
			});
			Ok(())
		}

		/// Same function as Rebond. But need to provide unlock_id.
		/// Parameters:
		/// - `currency_id`: The currency to rebond.
		/// - `unlock_id`: The unlock_id to rebond.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::rebond_by_unlock_id())]
		pub fn rebond_by_unlock_id(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			unlock_id: UnlockId,
		) -> DispatchResult {
			let rebonder = ensure_signed(origin)?;

			let v_currency_id =
				currency_id.to_vtoken().map_err(|_| Error::<T>::NotSupportTokenType)?;

			let (who, unlock_amount, time_unit, _) =
				TokenUnlockLedger::<T>::get(currency_id, unlock_id)
					.ok_or(Error::<T>::TokenUnlockLedgerNotFound)?;
			ensure!(who == rebonder, Error::<T>::CanNotRebond);

			Self::update_unlock_ledger(
				&rebonder,
				&currency_id,
				&unlock_amount,
				&unlock_id,
				&time_unit,
				None,
				Operation::Sub,
			)?;

			let (currency_amount, v_currency_amount, fee) =
				Self::mint_without_transfer(&rebonder, v_currency_id, currency_id, unlock_amount)?;

			TokenToRebond::<T>::mutate(&currency_id, |maybe_value| -> Result<(), Error<T>> {
				match maybe_value {
					Some(rebonded_amount) => {
						*rebonded_amount = rebonded_amount
							.checked_add(&currency_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;
						Ok(())
					},
					None => Err(Error::<T>::InvalidRebondToken),
				}
			})?;

			Self::deposit_event(Event::RebondedByUnlockId {
				rebonder,
				currency_id,
				currency_amount: unlock_amount,
				v_currency_amount,
				fee,
				unlock_id,
			});
			Ok(())
		}

		/// Set the unlock duration for a currency.
		/// Parameters:
		/// - `currency_id`: The currency to set unlock duration.
		/// - `unlock_duration`: The unlock duration to set.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::set_unlock_duration())]
		pub fn set_unlock_duration(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			unlock_duration: TimeUnit,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			UnlockDuration::<T>::mutate(currency_id, |old_unlock_duration| {
				*old_unlock_duration = Some(unlock_duration.clone());
			});

			Self::deposit_event(Event::UnlockDurationSet { currency_id, unlock_duration });
			Ok(())
		}

		/// Set the minimum mint amount for a currency.
		/// Parameters:
		/// - `currency_id`: The currency to set minimum mint amount.
		/// - `minimum_amount`: The minimum mint amount to set.
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::set_minimum_mint())]
		pub fn set_minimum_mint(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			minimum_amount: BalanceOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			MinimumMint::<T>::mutate(currency_id, |old_amount| {
				*old_amount = minimum_amount;
			});

			Self::deposit_event(Event::MinimumMintSet { currency_id, minimum_amount });

			Ok(())
		}

		/// Set the minimum redeem amount for a currency.
		/// Parameters:
		/// - `currency_id`: The currency to set minimum redeem amount.
		/// - `minimum_amount`: The minimum redeem amount to set.
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::set_minimum_redeem())]
		pub fn set_minimum_redeem(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			minimum_amount: BalanceOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			MinimumRedeem::<T>::mutate(currency_id, |old_amount| {
				*old_amount = minimum_amount;
			});

			Self::deposit_event(Event::MinimumRedeemSet { currency_id, minimum_amount });
			Ok(())
		}

		/// Support a token to rebond.
		/// Parameters:
		/// - `currency_id`: The currency to support rebond.
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::add_support_rebond_token())]
		pub fn add_support_rebond_token(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			TokenToRebond::<T>::mutate(currency_id, |maybe_value| -> DispatchResult {
				match maybe_value {
					Some(_) => Err(Error::<T>::InvalidRebondToken.into()),
					None => {
						*maybe_value = Some(BalanceOf::<T>::zero());
						Self::deposit_event(Event::SupportRebondTokenAdded { currency_id });
						Ok(())
					},
				}
			})
		}

		/// Remove the support of a token to rebond.
		/// Parameters:
		/// - `currency_id`: The currency to remove support rebond.
		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::remove_support_rebond_token())]
		pub fn remove_support_rebond_token(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			TokenToRebond::<T>::mutate(currency_id, |maybe_value| -> DispatchResult {
				match maybe_value {
					Some(_) => {
						*maybe_value = None;
						Self::deposit_event(Event::SupportRebondTokenRemoved { currency_id });
						Ok(())
					},
					None => Err(Error::<T>::InvalidRebondToken.into()),
				}
			})
		}

		/// Set the fees for mint and redeem.
		/// Parameters:
		/// - `mint_fee`: The fee for mint.
		/// - `redeem_fee`: The fee for redeem.
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::set_fees())]
		pub fn set_fees(
			origin: OriginFor<T>,
			mint_fee: Permill,
			redeem_fee: Permill,
		) -> DispatchResult {
			ensure_root(origin)?;

			Fees::<T>::mutate(|fees| *fees = (mint_fee, redeem_fee));

			Self::deposit_event(Event::FeeSet { mint_fee, redeem_fee });
			Ok(())
		}

		/// Set the hook iteration limit.
		/// Parameters:
		/// - `limit`: The hook iteration limit.
		#[pallet::call_index(10)]
		#[pallet::weight(T::WeightInfo::set_hook_iteration_limit())]
		pub fn set_hook_iteration_limit(origin: OriginFor<T>, limit: u32) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			HookIterationLimit::<T>::mutate(|old_limit| {
				*old_limit = limit;
			});

			Self::deposit_event(Event::HookIterationLimitSet { limit });
			Ok(())
		}

		/// Set the total amount of tokens that are currently locked for unlocking.
		/// Parameters:
		/// - `currency_id`: The currency to set unlocking total.
		/// - `currency_amount`: The total amount of tokens that are currently locked for unlocking.
		#[pallet::call_index(11)]
		#[pallet::weight(T::WeightInfo::set_unlocking_total())]
		pub fn set_unlocking_total(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			currency_amount: BalanceOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Self::update_unlocking_total(&currency_id, &currency_amount, Operation::Set)?;
			Self::deposit_event(Event::UnlockingTotalSet { currency_id, currency_amount });
			Ok(())
		}

		/// Set the minimum time unit for a currency.
		/// Parameters:
		/// - `currency_id`: The currency to set minimum time unit.
		/// - `time_unit`: The minimum time unit to set.
		#[pallet::call_index(12)]
		#[pallet::weight(T::WeightInfo::set_min_time_unit())]
		pub fn set_min_time_unit(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			time_unit: TimeUnit,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			MinTimeUnit::<T>::mutate(&currency_id, |old_time_unit| {
				*old_time_unit = time_unit.clone()
			});

			Self::deposit_event(Event::MinTimeUnitSet { currency_id, time_unit });
			Ok(())
		}

		/// Set the ongoing time unit for a currency.
		/// Parameters:
		/// - `currency_id`: The currency to set ongoing time unit.
		/// - `time_unit`: The ongoing time unit to set.
		#[pallet::call_index(13)]
		#[pallet::weight(T::WeightInfo::set_ongoing_time_unit())]
		pub fn set_ongoing_time_unit(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			time_unit: TimeUnit,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			OngoingTimeUnit::<T>::mutate(&currency_id, |old_time_unit| {
				*old_time_unit = Some(time_unit.clone())
			});

			Self::deposit_event(Event::SetOngoingTimeUnit { currency_id, time_unit });
			Ok(())
		}

		// mint with lock to get incentive vtoken
		#[pallet::call_index(14)]
		#[pallet::weight(T::WeightInfo::mint_with_lock())]
		pub fn mint_with_lock(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			currency_amount: BalanceOf<T>,
			remark: BoundedVec<u8, ConstU32<32>>,
			channel_id: Option<u32>,
		) -> DispatchResult {
			// Check origin
			let minter = ensure_signed(origin)?;

			// check if the minter has at least currency_amount of currency_id which is transferable
			T::MultiCurrency::ensure_can_withdraw(currency_id, &minter, currency_amount)
				.map_err(|_| Error::<T>::NotEnoughBalance)?;

			// check whether the currency_id is supported
			ensure!(MinimumMint::<T>::contains_key(currency_id), Error::<T>::NotSupportTokenType);

			// check whether the user has veBNC
			let vebnc_balance =
				T::BbBNC::balance_of(&minter, None).map_err(|_| Error::<T>::VeBNCCheckingError)?;
			ensure!(vebnc_balance > BalanceOf::<T>::zero(), Error::<T>::NotEnoughBalance);

			// check whether the vtoken coefficient is set
			let v_currency_id =
				currency_id.to_vtoken().map_err(|_| Error::<T>::NotSupportTokenType)?;

			ensure!(
				VtokenIncentiveCoef::<T>::contains_key(v_currency_id),
				Error::<T>::IncentiveCoefNotFound
			);

			// check whether the pool has balance of v_currency_id
			let incentive_pool_account = &Self::incentive_pool_account();
			let vtoken_pool_balance =
				T::MultiCurrency::free_balance(v_currency_id, &incentive_pool_account);

			ensure!(vtoken_pool_balance > BalanceOf::<T>::zero(), Error::<T>::NotEnoughBalance);

			// mint vtoken
			let vtoken_minted =
				Self::do_mint(minter.clone(), currency_id, currency_amount, remark, channel_id)?;

			// lock vtoken and record the lock
			Self::lock_vtoken_for_incentive_minting(minter.clone(), v_currency_id, vtoken_minted)?;

			// calculate the incentive amount
			let incentive_amount =
				Self::calculate_incentive_vtoken_amount(&minter, v_currency_id, vtoken_minted)?;

			// Since the user has already locked the vtoken, we can directly transfer the incentive
			// vtoken. It won't fail. transfer the incentive amount to the minter
			T::MultiCurrency::transfer(
				v_currency_id,
				incentive_pool_account,
				&minter,
				incentive_amount,
			)
			.map_err(|_| Error::<T>::NotEnoughBalance)?;

			// deposit event
			Self::deposit_event(Event::IncentivizedMinting {
				address: minter,
				currency_id,
				currency_amount,
				locked_vtoken_amount: vtoken_minted,
				incentive_vtoken_amount: incentive_amount,
			});

			Ok(())
		}

		/// Unlock the vtoken minted in an incentive mode
		/// Parameters:
		/// - `v_currency_id`: The v_currency to unlock.
		#[pallet::call_index(15)]
		#[pallet::weight(T::WeightInfo::unlock_incentive_minted_vtoken())]
		pub fn unlock_incentive_minted_vtoken(
			origin: OriginFor<T>,
			v_currency_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			let unlocker = ensure_signed(origin)?;

			// get the user's VtokenLockLedger
			ensure!(
				VtokenLockLedger::<T>::contains_key(&unlocker, v_currency_id),
				Error::<T>::UserUnlockLedgerNotFound
			);

			VtokenLockLedger::<T>::mutate_exists(
				&unlocker,
				v_currency_id,
				|maybe_ledger| -> Result<(), Error<T>> {
					let current_block = frame_system::Pallet::<T>::block_number();

					if let Some(ref mut ledger) = maybe_ledger {
						// check the total locked amount
						let (total_locked, mut lock_records) = ledger.clone();

						// unlock the vtoken
						let mut unlock_amount = BalanceOf::<T>::zero();
						let mut remove_index = 0;

						// enumerate lock_records
						for (index, (locked_amount, due_block_num)) in
							lock_records.iter().enumerate()
						{
							if current_block >= *due_block_num {
								unlock_amount += *locked_amount;
								remove_index = index + 1;
							} else {
								break;
							}
						}

						// remove all the records less than remove_index
						if remove_index > 0 {
							lock_records.drain(0..remove_index);
						}

						// check the unlock amount
						ensure!(unlock_amount > BalanceOf::<T>::zero(), Error::<T>::NoUnlockRecord);

						let remaining_locked_amount = total_locked
							.checked_sub(&unlock_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;

						if remaining_locked_amount == BalanceOf::<T>::zero() {
							T::MultiCurrency::remove_lock(
								INCENTIVE_LOCK_ID,
								v_currency_id,
								&unlocker,
							)
							.map_err(|_| Error::<T>::FailToRemoveLock)?;

							// remove the ledger
							*maybe_ledger = None;
						} else {
							// update the ledger
							*ledger = (remaining_locked_amount, lock_records);

							// reset the locked amount to be remaining_locked_amount
							T::MultiCurrency::set_lock(
								INCENTIVE_LOCK_ID,
								v_currency_id,
								&unlocker,
								remaining_locked_amount,
							)
							.map_err(|_| Error::<T>::Unexpected)?;
						}

						Ok(())
					} else {
						Err(Error::<T>::UserUnlockLedgerNotFound)
					}
				},
			)?;

			Ok(())
		}

		/// Set the incentive coefficient for a vtoken when minted in an incentive mode
		/// Parameters:
		/// - `v_currency_id`: The v_currency to set incentive coefficient.
		/// - `new_coef_op`: The new incentive coefficient to set.
		#[pallet::call_index(16)]
		#[pallet::weight(T::WeightInfo::set_incentive_coef())]
		pub fn set_incentive_coef(
			origin: OriginFor<T>,
			v_currency_id: CurrencyIdOf<T>,
			new_coef_op: Option<u128>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			if let Some(new_coef) = new_coef_op {
				VtokenIncentiveCoef::<T>::insert(v_currency_id, new_coef);
			} else {
				VtokenIncentiveCoef::<T>::remove(v_currency_id);
			}

			Self::deposit_event(Event::VtokenIncentiveCoefSet {
				v_currency_id,
				coefficient: new_coef_op,
			});

			Ok(())
		}

		/// Set the locked blocks for a vtoken when minted in an incentive mode
		/// Parameters:
		/// - `v_currency_id`: The v_currency to set locked blocks.
		/// - `new_blockes_op`: The new locked blocks to set.
		#[pallet::call_index(17)]
		#[pallet::weight(T::WeightInfo::set_vtoken_incentive_lock_blocks())]
		pub fn set_vtoken_incentive_lock_blocks(
			origin: OriginFor<T>,
			v_currency_id: CurrencyIdOf<T>,
			new_blockes_op: Option<BlockNumberFor<T>>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			if let Some(new_blocks) = new_blockes_op {
				MintWithLockBlocks::<T>::insert(v_currency_id, new_blocks);
			} else {
				MintWithLockBlocks::<T>::remove(v_currency_id);
			}

			Self::deposit_event(Event::VtokenIncentiveLockBlocksSet {
				v_currency_id,
				blocks: new_blockes_op,
			});

			Ok(())
		}
	}
}
