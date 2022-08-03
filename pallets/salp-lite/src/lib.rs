// This file is part of Bifrost.

// Copyright (C) 2019-2022 Liebi Technologies (UK) Ltd.
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
#![allow(deprecated)] // TODO: clear transaction

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;
pub mod weights;
pub use weights::WeightInfo;

// Re-export pallet items so that they can be accessed from the crate namespace.
use frame_support::{pallet_prelude::*, transactional};
use node_primitives::{ContributionStatus, TokenInfo, TokenSymbol, TrieIndex};
use orml_traits::MultiCurrency;
pub use pallet::*;
use scale_info::TypeInfo;

#[allow(type_alias_bounds)]
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[allow(type_alias_bounds)]
type BalanceOf<T: Config> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum FundStatus {
	Ongoing,
	Retired,
	Success,
	Failed,
	RefundWithdrew,
	RedeemWithdrew,
	FailedToContinue,
}

impl Default for FundStatus {
	fn default() -> Self {
		FundStatus::Ongoing
	}
}

/// Information on a funding effort for a pre-existing parachain. We assume that the parachain
/// ID is known as it's used for the key of the storage item for which this is the value
/// (`Funds`).
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
#[codec(dumb_trait_bound)]
pub struct FundInfo<Balance, LeasePeriod> {
	/// The total amount raised.
	raised: Balance,
	/// A hard-cap on the amount that may be contributed.
	cap: Balance,
	/// First slot in range to bid on; it's actually a LeasePeriod, but that's the same type as
	/// BlockNumber.
	first_slot: LeasePeriod,
	/// Last slot in range to bid on; it's actually a LeasePeriod, but that's the same type as
	/// BlockNumber.
	last_slot: LeasePeriod,
	/// Index used for the child trie of this fund
	trie_index: TrieIndex,
	/// Fund status
	status: FundStatus,
}

#[frame_support::pallet]
pub mod pallet {
	// Import various types used to declare pallet in scope.

	use frame_support::{
		pallet_prelude::{storage::child, *},
		sp_runtime::traits::{AccountIdConversion, CheckedAdd, Hash, Saturating, Zero},
		sp_std::convert::TryInto,
		storage::ChildTriePrefixIterator,
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use node_primitives::{BancorHandler, CurrencyId, LeasePeriod, MessageId, ParaId};
	use orml_traits::{currency::TransferAll, MultiCurrency, MultiReservableCurrency};
	use sp_arithmetic::Percent;
	use sp_std::prelude::*;

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config<BlockNumber = LeasePeriod> {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// ModuleID for the crowdloan module. An appropriate value could be
		/// ```ModuleId(*b"py/cfund")```
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The minimum amount that may be contributed into a crowdloan. Should almost certainly be
		/// at least ExistentialDeposit.
		#[pallet::constant]
		type MinContribution: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type RelayChainToken: Get<CurrencyId>;

		/// The number of blocks over which a single period lasts.
		#[pallet::constant]
		type LeasePeriod: Get<BlockNumberFor<Self>>;

		/// The time interval from 1:1 redeem-pool to bancor-pool to release.
		#[pallet::constant]
		type ReleaseCycle: Get<BlockNumberFor<Self>>;

		/// The release ratio from the 1:1 redeem-pool to the bancor-pool per cycle.
		#[pallet::constant]
		type ReleaseRatio: Get<Percent>;

		#[pallet::constant]
		type BatchKeysLimit: Get<u32>;

		#[pallet::constant]
		type SlotLength: Get<LeasePeriod>;

		type MultiCurrency: TransferAll<AccountIdOf<Self>>
			+ MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiReservableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type BancorPool: BancorHandler<BalanceOf<Self>>;

		type EnsureConfirmAsGovernance: EnsureOrigin<<Self as frame_system::Config>::Origin>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Create a new crowdloaning campaign. [fund_index]
		Created(ParaId),
		/// Contributed to a crowd sale. [who, fund_index, amount]
		Issued(AccountIdOf<T>, ParaId, BalanceOf<T>, MessageId),
		/// Withdrew full balance of a contributor. [who, fund_index, amount]
		Withdrew(ParaId, BalanceOf<T>),
		/// refund to account. [who, fund_index,value]
		Refunded(AccountIdOf<T>, ParaId, LeasePeriod, LeasePeriod, BalanceOf<T>),
		/// redeem to account. [who, fund_index, first_slot, last_slot, value]
		Redeemed(AccountIdOf<T>, ParaId, LeasePeriod, LeasePeriod, BalanceOf<T>),
		/// Fund is edited. [fund_index]
		Edited(ParaId),
		/// Fund is dissolved. [fund_index]
		Dissolved(ParaId),
		/// The vsToken/vsBond was be unlocked. [who, fund_index, value]
		Unlocked(AccountIdOf<T>, ParaId, BalanceOf<T>),
		AllUnlocked(ParaId),
		/// Fund status change
		Failed(ParaId),
		Success(ParaId),
		Retired(ParaId),
		Continued(ParaId, LeasePeriod, LeasePeriod),
		RefundedDissolved(ParaId, LeasePeriod, LeasePeriod),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The first slot needs to at least be less than 3 `max_value`.
		FirstSlotTooFarInFuture,
		/// Last slot must be greater than first slot.
		LastSlotBeforeFirstSlot,
		/// The last slot cannot be more then 3 slots after the first slot.
		LastSlotTooFarInFuture,
		/// Migrate slot must be greater than first slot
		MigrateSlotBeforeFirstSlot,
		/// There was an overflow.
		Overflow,
		/// The contribution was below the minimum, `MinContribution`.
		ContributionTooSmall,
		/// The account doesn't have any contribution to the fund.
		ZeroContribution,
		/// Invalid fund index.
		InvalidParaId,
		/// Invalid fund status.
		InvalidFundStatus,
		/// Invalid contribution status.
		InvalidContributionStatus,
		/// Contributions exceed maximum amount.
		CapExceeded,
		/// The fund has been registered.
		FundAlreadyCreated,
		/// Don't have enough vsToken/vsBond to refund
		NotEnoughReservedAssetsToRefund,
		/// Don't have enough token to refund by users
		NotEnoughBalanceInRefundPool,
		/// Don't have enough vsToken/vsBond to unlock
		NotEnoughBalanceToUnlock,
		/// Dont have enough vsToken/vsBond to redeem
		NotEnoughFreeAssetsToRedeem,
		/// Don't have enough token to redeem by users
		NotEnoughBalanceInRedeemPool,
		/// Invalid Fund when refund/redeem
		NotEnoughBalanceInFund,
		InvalidFundSameSlot,
		InvalidFundNotExist,
		InvalidRefund,
	}

	/// Multisig confirm account
	#[pallet::storage]
	#[pallet::getter(fn multisig_confirm_account)]
	pub type MultisigConfirmAccount<T: Config> = StorageValue<_, AccountIdOf<T>, OptionQuery>;

	/// Tracker for the next available fund index
	#[pallet::storage]
	#[pallet::getter(fn current_trie_index)]
	pub(super) type CurrentTrieIndex<T: Config> = StorageValue<_, TrieIndex, ValueQuery>;

	/// Info on all of the funds.
	#[pallet::storage]
	#[pallet::getter(fn funds)]
	pub(super) type Funds<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		ParaId,
		Option<FundInfo<BalanceOf<T>, LeasePeriod>>,
		ValueQuery,
	>;

	/// Info on all of the fail-to-continue funds.
	#[pallet::storage]
	#[pallet::getter(fn failed_funds_to_refund)]
	pub(super) type FailedFundsToRefund<T: Config> = StorageNMap<
		_,
		(
			NMapKey<Blake2_128Concat, ParaId>,
			NMapKey<Blake2_128Concat, LeasePeriod>,
			NMapKey<Blake2_128Concat, LeasePeriod>,
		),
		Option<FundInfo<BalanceOf<T>, LeasePeriod>>,
		ValueQuery,
	>;

	/// The balance can be redeemed to users.
	#[pallet::storage]
	#[pallet::getter(fn redeem_pool)]
	pub(super) type RedeemPool<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub initial_multisig_account: Option<AccountIdOf<T>>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { initial_multisig_account: None }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			if let Some(ref key) = self.initial_multisig_account {
				MultisigConfirmAccount::<T>::put(key)
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		pub fn set_multisig_confirm_account(
			origin: OriginFor<T>,
			account: AccountIdOf<T>,
		) -> DispatchResult {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?;

			Self::set_multisig_account(account);

			Ok(())
		}

		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		pub fn fund_success(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
		) -> DispatchResult {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Ongoing, Error::<T>::InvalidFundStatus);

			let fund_new = FundInfo { status: FundStatus::Success, ..fund };
			Funds::<T>::insert(index, Some(fund_new));
			Self::deposit_event(Event::<T>::Success(index));

			Ok(())
		}

		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		pub fn fund_fail(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?;

			// crownload is failed, so enable the withdrawal function of vsToken/vsBond
			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Ongoing, Error::<T>::InvalidFundStatus);

			let fund_new = FundInfo { status: FundStatus::Failed, ..fund };
			Funds::<T>::insert(index, Some(fund_new));

			Self::deposit_event(Event::<T>::Failed(index));

			Ok(())
		}

		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		pub fn fund_retire(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
		) -> DispatchResult {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Success, Error::<T>::InvalidFundStatus);

			let fund_new = FundInfo { status: FundStatus::Retired, ..fund };
			Funds::<T>::insert(index, Some(fund_new));
			Self::deposit_event(Event::<T>::Retired(index));

			Ok(())
		}

		/// Create a new crowdloaning campaign for a parachain slot deposit for the current auction.
		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		pub fn create(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] cap: BalanceOf<T>,
			#[pallet::compact] first_slot: LeasePeriod,
			#[pallet::compact] last_slot: LeasePeriod,
		) -> DispatchResult {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?;

			ensure!(!Funds::<T>::contains_key(index), Error::<T>::FundAlreadyCreated);

			ensure!(first_slot <= last_slot, Error::<T>::LastSlotBeforeFirstSlot);

			let last_slot_limit = first_slot
				.checked_add(((T::SlotLength::get() as u32) - 1).into())
				.ok_or(Error::<T>::FirstSlotTooFarInFuture)?;
			ensure!(last_slot <= last_slot_limit, Error::<T>::LastSlotTooFarInFuture);

			Funds::<T>::insert(
				index,
				Some(FundInfo {
					raised: Zero::zero(),
					cap,
					first_slot,
					last_slot,
					trie_index: Self::next_trie_index()?,
					status: FundStatus::Ongoing,
				}),
			);

			Self::deposit_event(Event::<T>::Created(index));

			Ok(())
		}

		/// Contribute to a crowd sale. This will transfer some balance over to fund a parachain
		/// slot. It will be withdrawable in two instances: the parachain becomes retired; or the
		/// slot is unable to be purchased and the timeout expires.
		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		#[transactional]
		pub fn issue(
			origin: OriginFor<T>,
			who: AccountIdOf<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] value: BalanceOf<T>,
			message_id: MessageId,
		) -> DispatchResult {
			let issuer = ensure_signed(origin.clone())?;
			if Some(issuer) != MultisigConfirmAccount::<T>::get() {
				return Err(DispatchError::BadOrigin.into());
			}

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(
				fund.status == FundStatus::Ongoing || fund.status == FundStatus::Success,
				Error::<T>::InvalidFundStatus
			);

			ensure!(value >= T::MinContribution::get(), Error::<T>::ContributionTooSmall);

			let raised = fund.raised.checked_add(&value).ok_or(Error::<T>::Overflow)?;
			ensure!(raised <= fund.cap, Error::<T>::CapExceeded);

			let (contributed, _) = Self::contribution(fund.trie_index, &who);

			let (vs_token, vs_bond) = Self::vsAssets(index, fund.first_slot, fund.last_slot);

			// Issue vsToken/vsBond to contributor
			T::MultiCurrency::deposit(vs_token, &who, value)?;
			T::MultiCurrency::deposit(vs_bond, &who, value)?;

			// Update the raised of fund
			let fund_new = FundInfo { raised: fund.raised.saturating_add(value), ..fund };
			Funds::<T>::insert(index, Some(fund_new));

			// Update the contribution of who
			let contributed_new = contributed.saturating_add(value);
			Self::put_contribution(
				fund.trie_index,
				&who,
				contributed_new,
				ContributionStatus::Idle,
			);
			Self::deposit_event(Event::Issued(who, index, value, message_id));

			Ok(())
		}

		/// Withdraw full balance of the parachain.
		/// - `index`: The parachain to whose crowdloan the contribution was made.
		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		#[transactional]
		pub fn withdraw(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			T::EnsureConfirmAsGovernance::ensure_origin(origin.clone())?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			let can = fund.status == FundStatus::Failed || fund.status == FundStatus::Retired;
			ensure!(can, Error::<T>::InvalidFundStatus);

			let amount_withdrew = fund.raised;
			let total =
				Self::redeem_pool().checked_add(&amount_withdrew).ok_or(Error::<T>::Overflow)?;
			RedeemPool::<T>::set(total);

			if fund.status == FundStatus::Retired {
				let fund_new = FundInfo { status: FundStatus::RedeemWithdrew, ..fund };
				Funds::<T>::insert(index, Some(fund_new));
			} else if fund.status == FundStatus::Failed {
				let fund_new = FundInfo { status: FundStatus::RefundWithdrew, ..fund };
				Funds::<T>::insert(index, Some(fund_new));
			}
			Self::deposit_event(Event::Withdrew(index, amount_withdrew));

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::redeem())]
		#[transactional]
		pub fn redeem(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] value: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			let mut fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::RedeemWithdrew, Error::<T>::InvalidFundStatus);
			ensure!(fund.raised >= value, Error::<T>::NotEnoughBalanceInFund);
			ensure!(Self::redeem_pool() >= value, Error::<T>::NotEnoughBalanceInRedeemPool);

			#[allow(non_snake_case)]
			let (vsToken, vsBond) = Self::vsAssets(index, fund.first_slot, fund.last_slot);

			T::MultiCurrency::ensure_can_withdraw(vsToken, &who, value)
				.map_err(|_e| Error::<T>::NotEnoughFreeAssetsToRedeem)?;
			T::MultiCurrency::ensure_can_withdraw(vsBond, &who, value)
				.map_err(|_e| Error::<T>::NotEnoughFreeAssetsToRedeem)?;

			T::MultiCurrency::withdraw(vsToken, &who, value)?;
			T::MultiCurrency::withdraw(vsBond, &who, value)?;
			RedeemPool::<T>::set(Self::redeem_pool().saturating_sub(value));
			fund.raised = fund.raised.saturating_sub(value);
			Funds::<T>::insert(index, Some(fund.clone()));

			Self::deposit_event(Event::Redeemed(
				who,
				index,
				fund.first_slot,
				fund.last_slot,
				value,
			));

			Ok(())
		}

		/// Remove a fund after the retirement period has ended and all funds have been returned.
		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		#[transactional]
		pub fn dissolve(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?;

			let mut fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(
				fund.status == FundStatus::RedeemWithdrew ||
					fund.status == FundStatus::RefundWithdrew,
				Error::<T>::InvalidFundStatus
			);

			let mut refund_count = 0u32;
			// Try killing the crowdloan child trie and Assume everyone will be refunded.
			let contributions = Self::contribution_iterator(fund.trie_index);
			let mut all_refunded = true;
			#[allow(clippy::explicit_counter_loop)]
			for (who, (balance, _)) in contributions {
				if refund_count >= T::BatchKeysLimit::get() {
					// Not everyone was able to be refunded this time around.
					all_refunded = false;
					break;
				}
				Self::kill_contribution(fund.trie_index, &who);
				fund.raised = fund.raised.saturating_sub(balance);
				refund_count += 1;
			}

			if all_refunded {
				Funds::<T>::remove(index);
				Self::deposit_event(Event::<T>::Dissolved(index));
			}

			Ok(())
		}

		#[pallet::weight((
			0,
			DispatchClass::Normal,
			Pays::No
			))]
		pub fn continue_fund(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] first_slot: LeasePeriod,
			#[pallet::compact] last_slot: LeasePeriod,
		) -> DispatchResult {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?;

			// crownload is failed, so enable the withdrawal function of vsToken/vsBond
			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::RefundWithdrew, Error::<T>::InvalidFundStatus);
			ensure!(
				fund.first_slot != first_slot || fund.last_slot != last_slot,
				Error::<T>::InvalidFundSameSlot
			);

			let fund_old = FundInfo { status: FundStatus::FailedToContinue, ..fund };
			FailedFundsToRefund::<T>::insert(
				(index, fund.first_slot, fund.last_slot),
				Some(fund_old.clone()),
			);
			let fund_new = FundInfo { status: FundStatus::Ongoing, first_slot, last_slot, ..fund };
			Funds::<T>::insert(index, Some(fund_new));

			Self::deposit_event(Event::<T>::Continued(
				index,
				fund_old.first_slot,
				fund_old.last_slot,
			));

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::refund())]
		#[transactional]
		pub fn refund(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] first_slot: LeasePeriod,
			#[pallet::compact] last_slot: LeasePeriod,
			#[pallet::compact] value: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			let mut fund = Self::find_fund(index, first_slot, last_slot)
				.map_err(|_| Error::<T>::InvalidFundNotExist)?;
			ensure!(
				fund.status == FundStatus::FailedToContinue ||
					fund.status == FundStatus::RefundWithdrew,
				Error::<T>::InvalidRefund
			);
			ensure!(
				fund.first_slot == first_slot && fund.last_slot == last_slot,
				Error::<T>::InvalidRefund
			);
			ensure!(fund.raised >= value, Error::<T>::NotEnoughBalanceInFund);
			ensure!(Self::redeem_pool() >= value, Error::<T>::NotEnoughBalanceInRefundPool);

			#[allow(non_snake_case)]
			let (vsToken, vsBond) = Self::vsAssets(index, fund.first_slot, fund.last_slot);
			T::MultiCurrency::ensure_can_withdraw(vsToken, &who, value)
				.map_err(|_e| Error::<T>::NotEnoughFreeAssetsToRedeem)?;
			T::MultiCurrency::ensure_can_withdraw(vsBond, &who, value)
				.map_err(|_e| Error::<T>::NotEnoughFreeAssetsToRedeem)?;

			T::MultiCurrency::withdraw(vsToken, &who, value)?;
			T::MultiCurrency::withdraw(vsBond, &who, value)?;

			RedeemPool::<T>::set(Self::redeem_pool().saturating_sub(value));
			let mut fund_new = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			fund_new.raised = fund_new.raised.saturating_sub(value);
			Funds::<T>::insert(index, Some(fund_new));
			if fund.status == FundStatus::FailedToContinue {
				fund.raised = fund.raised.saturating_sub(value);
				FailedFundsToRefund::<T>::insert(
					(index, first_slot, last_slot),
					Some(fund.clone()),
				);
			}

			Self::deposit_event(Event::Refunded(
				who,
				index,
				fund.first_slot,
				fund.last_slot,
				value,
			));

			Ok(())
		}

		/// Remove a fund after the retirement period has ended and all funds have been returned.
		#[pallet::weight((
			0,
			DispatchClass::Normal,
			Pays::No
			))]
		#[transactional]
		pub fn dissolve_refunded(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] first_slot: LeasePeriod,
			#[pallet::compact] last_slot: LeasePeriod,
		) -> DispatchResult {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?;

			let fund = Self::failed_funds_to_refund((index, first_slot, last_slot))
				.ok_or(Error::<T>::InvalidRefund)?;

			ensure!(fund.status == FundStatus::FailedToContinue, Error::<T>::InvalidFundStatus);

			FailedFundsToRefund::<T>::remove((index, first_slot, last_slot));

			Self::deposit_event(Event::<T>::RefundedDissolved(index, first_slot, last_slot));

			Ok(())
		}

		/// Edit the configuration for an in-progress crowdloan.
		///
		/// Can only be called by Root origin.
		#[pallet::weight((
			0,
			DispatchClass::Normal,
			Pays::No
			))]
		pub fn edit(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] cap: BalanceOf<T>,
			#[pallet::compact] raised: BalanceOf<T>,
			#[pallet::compact] first_slot: LeasePeriod,
			#[pallet::compact] last_slot: LeasePeriod,
			fund_status: Option<FundStatus>,
		) -> DispatchResult {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;

			let status = match fund_status {
				None => fund.status,
				Some(status) => status,
			};
			Funds::<T>::insert(
				index,
				Some(FundInfo {
					cap,
					first_slot,
					last_slot,
					status,
					raised,
					trie_index: fund.trie_index,
				}),
			);
			Self::deposit_event(Event::<T>::Edited(index));
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			// Release x% KSM/DOT from redeem-pool to bancor-pool per cycle
			if n != 0 && (n % T::ReleaseCycle::get()) == 0 {
				if let Ok(rp_balance) = TryInto::<u128>::try_into(Self::redeem_pool()) {
					// Calculate the release amount
					let release_amount = T::ReleaseRatio::get() * rp_balance;

					// Must be ok
					if let Ok(release_amount) = TryInto::<BalanceOf<T>>::try_into(release_amount) {
						// Increase the balance of bancor-pool by release-amount
						if let Ok(()) =
							T::BancorPool::add_token(T::RelayChainToken::get(), release_amount)
						{
							RedeemPool::<T>::set(
								Self::redeem_pool().saturating_sub(release_amount),
							);
						}
					} else {
						log::warn!("Overflow: The balance of redeem-pool exceeds u128.");
					}
				}
			}
			T::DbWeight::get().reads(1)
		}
	}

	impl<T: Config> Pallet<T> {
		/// set multisig account
		pub fn set_multisig_account(account: AccountIdOf<T>) {
			MultisigConfirmAccount::<T>::put(account);
		}

		pub fn find_fund(
			index: ParaId,
			first_slot: LeasePeriod,
			last_slot: LeasePeriod,
		) -> Result<FundInfo<BalanceOf<T>, LeasePeriod>, Error<T>> {
			return match Self::failed_funds_to_refund((index, first_slot, last_slot)) {
				Some(fund) => Ok(fund),
				_ => match Self::funds(index) {
					Some(fund) => Ok(fund),
					_ => Err(Error::<T>::InvalidFundNotExist),
				},
			};
		}
		pub fn fund_account_id(index: ParaId) -> T::AccountId {
			T::PalletId::get().into_sub_account_truncating(index)
		}

		pub(crate) fn id_from_index(index: TrieIndex) -> child::ChildInfo {
			let mut buf = Vec::new();
			buf.extend_from_slice(&(T::PalletId::get().0));
			buf.extend_from_slice(&index.encode()[..]);
			child::ChildInfo::new_default(T::Hashing::hash(&buf[..]).as_ref())
		}

		pub(crate) fn contribution(
			index: TrieIndex,
			who: &AccountIdOf<T>,
		) -> (BalanceOf<T>, ContributionStatus<BalanceOf<T>>) {
			who.using_encoded(|b| {
				child::get_or_default::<(BalanceOf<T>, ContributionStatus<BalanceOf<T>>)>(
					&Self::id_from_index(index),
					b,
				)
			})
		}

		pub fn contribution_by_fund(
			index: ParaId,
			who: &AccountIdOf<T>,
		) -> Result<(BalanceOf<T>, ContributionStatus<BalanceOf<T>>), Error<T>> {
			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			let (contributed, status) = Self::contribution(fund.trie_index, who);
			Ok((contributed, status))
		}

		pub(crate) fn contribution_iterator(
			index: TrieIndex,
		) -> ChildTriePrefixIterator<(
			AccountIdOf<T>,
			(BalanceOf<T>, ContributionStatus<BalanceOf<T>>),
		)> {
			ChildTriePrefixIterator::<_>::with_prefix_over_key::<Identity>(
				&Self::id_from_index(index),
				&[],
			)
		}

		pub(crate) fn next_trie_index() -> Result<TrieIndex, Error<T>> {
			CurrentTrieIndex::<T>::try_mutate(|ti| {
				*ti = ti.checked_add(1).ok_or(Error::<T>::Overflow)?;
				Ok(*ti - 1)
			})
		}

		#[allow(non_snake_case)]
		pub(crate) fn vsAssets(
			index: ParaId,
			first_slot: LeasePeriod,
			last_slot: LeasePeriod,
		) -> (CurrencyId, CurrencyId) {
			let currency_id_u64: u64 = T::RelayChainToken::get().currency_id();
			let tokensymbo_bit = (currency_id_u64 & 0x0000_0000_0000_00ff) as u8;
			let token_symbol = TokenSymbol::try_from(tokensymbo_bit).unwrap_or(TokenSymbol::DOT);
			CurrencyId::vsAssets(token_symbol, index, first_slot, last_slot)
		}

		fn put_contribution(
			index: TrieIndex,
			who: &AccountIdOf<T>,
			contributed: BalanceOf<T>,
			status: ContributionStatus<BalanceOf<T>>,
		) {
			who.using_encoded(|b| {
				child::put(&Self::id_from_index(index), b, &(contributed, status))
			});
		}

		fn kill_contribution(index: TrieIndex, who: &AccountIdOf<T>) {
			who.using_encoded(|b| child::kill(&Self::id_from_index(index), b));
		}

		#[allow(dead_code)]
		pub(crate) fn set_balance(who: &AccountIdOf<T>, value: BalanceOf<T>) -> DispatchResult {
			T::MultiCurrency::deposit(T::RelayChainToken::get(), who, value)
		}
	}
}
