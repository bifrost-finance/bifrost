// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

// TODO: Refactor the info returned by `Event`

mod mock;
mod tests;

// Re-export pallet items so that they can be accessed from the crate namespace.
use frame_support::{pallet_prelude::*, sp_runtime::MultiSignature};
use node_primitives::ParaId;
use orml_traits::MultiCurrency;
pub use pallet::*;

type TrieIndex = u32;

#[allow(type_alias_bounds)]
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[allow(type_alias_bounds)]
type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub enum FundStatus {
	Ongoing,
	Retired,
	Success,
	Failed,
	Withdrew,
	End,
}

impl Default for FundStatus {
	fn default() -> Self {
		FundStatus::Ongoing
	}
}

/// Information on a funding effort for a pre-existing parachain. We assume that the parachain
/// ID is known as it's used for the key of the storage item for which this is the value
/// (`Funds`).
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
#[codec(dumb_trait_bound)]
pub struct FundInfo<AccountId, Balance, LeasePeriod> {
	/// The owning account who placed the deposit.
	depositor: AccountId,
	/// The amount of deposit placed.
	deposit: Balance,
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

#[derive(Encode, Decode)]
pub enum CrowdloanContributeCall<BalanceOf> {
	#[codec(index = 73)]
	CrowdloanContribute(ContributeCall<BalanceOf>),
}

#[derive(Encode, Decode)]
pub enum CrowdloanWithdrawCall<AccountIdOf> {
	#[codec(index = 73)]
	CrowdloanWithdraw(WithdrawCall<AccountIdOf>),
}

#[derive(Debug, PartialEq, Encode, Decode)]
pub struct Contribution<BalanceOf> {
	#[codec(compact)]
	index: ParaId,
	#[codec(compact)]
	value: BalanceOf,
	signature: Option<MultiSignature>,
}

#[derive(Encode, Decode)]
pub enum ContributeCall<BalanceOf> {
	#[codec(index = 1)]
	Contribute(Contribution<BalanceOf>),
}

#[derive(Debug, PartialEq, Encode, Decode)]
pub struct Withdraw<AccountIdOf> {
	who: AccountIdOf,
	#[codec(compact)]
	index: ParaId,
}

#[derive(Encode, Decode)]
pub enum WithdrawCall<AccountIdOf> {
	#[codec(index = 2)]
	Withdraw(Withdraw<AccountIdOf>),
}

#[frame_support::pallet]
pub mod pallet {
	// Import various types used to declare pallet in scope.
	use frame_support::{
		pallet_prelude::{storage::child, *},
		sp_runtime::traits::{AccountIdConversion, CheckedAdd, CheckedSub, Hash, Saturating, Zero},
		storage::ChildTriePrefixIterator,
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use node_primitives::{traits::BancorHandler, CurrencyId, LeasePeriod, ParaId, TokenSymbol};
	use orml_traits::{
		currency::TransferAll, LockIdentifier, MultiCurrency, MultiCurrencyExtended,
		MultiLockableCurrency, MultiReservableCurrency,
	};
	use polkadot_parachain::primitives::Id as PolkadotParaId;
	use sp_arithmetic::Percent;
	use sp_std::{convert::TryInto, prelude::*};
	use xcm::v0::{
		prelude::{XcmError, XcmResult},
		Junction, MultiLocation,
	};
	use xcm_support::BifrostXcmExecutor;

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config<BlockNumber = LeasePeriod> {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// ModuleID for the crowdloan module. An appropriate value could be
		/// ```ModuleId(*b"py/cfund")```
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The amount to be held on deposit by the depositor of a crowdloan.
		type SubmissionDeposit: Get<BalanceOf<Self>>;

		/// The minimum amount that may be contributed into a crowdloan. Should almost certainly be
		/// at least ExistentialDeposit.
		#[pallet::constant]
		type MinContribution: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type RelyChainToken: Get<CurrencyId>;

		/// The number of blocks over which a single period lasts.
		#[pallet::constant]
		type LeasePeriod: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type VSBondValidPeriod: Get<BlockNumberFor<Self>>;

		/// The time interval from 1:1 redeem-pool to bancor-pool to release.
		#[pallet::constant]
		type ReleaseCycle: Get<LeasePeriod>;

		/// The release ratio from the 1:1 redeem-pool to the bancor-pool per cycle.
		///
		/// **NOTE: THE RELEASE RATIO MUST BE IN [0, 1].**
		#[pallet::constant]
		type ReleaseRatio: Get<Percent>;

		#[pallet::constant]
		type RemoveKeysLimit: Get<u32>;

		type MultiCurrency: TransferAll<AccountIdOf<Self>>
			+ MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiCurrencyExtended<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiLockableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiReservableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type BancorPool: BancorHandler<BalanceOf<Self>>;

		type ExecuteXcmOrigin: EnsureOrigin<
			<Self as frame_system::Config>::Origin,
			Success = MultiLocation,
		>;

		type BifrostXcmExecutor: BifrostXcmExecutor;

		#[pallet::constant]
		type SlotLength: Get<LeasePeriod>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Create a new crowdloaning campaign. [fund_index]
		Created(ParaId),
		/// Contributing to a crowd sale. [who, fund_index, amount]
		Contributing(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Contributed to a crowd sale. [who, fund_index, amount]
		Contributed(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Fail on contribute to crowd sale. [who, fund_index, amount]
		ContributeFailed(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Withdrawing full balance of a contributor. [who, fund_index, amount]
		Withdrawing(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Withdrew full balance of a contributor. [who, fund_index, amount]
		Withdrew(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Fail on withdraw full balance of a contributor. [who, fund_index, amount]
		WithdrawFailed(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// TODO
		Refunding(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// TODO
		Refunded(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// TODO
		RefundFailed(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Redeeming token(rely-chain) by vsToken/vsBond. [who, fund_index, amount]
		Redeeming(AccountIdOf<T>, BalanceOf<T>),
		/// Redeemed token(rely-chain) by vsToken/vsBond. [who, fund_index, amount]
		Redeemed(AccountIdOf<T>, BalanceOf<T>),
		/// Fail on redeem token(rely-chain) by vsToken/vsBond. [who, fund_index, amount]
		RedeemFailed(AccountIdOf<T>, BalanceOf<T>),
		/// Fund is dissolved. [fund_index]
		Dissolved(ParaId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The first slot needs to at least be less than 3 `max_value`.
		FirstSlotTooFarInFuture,
		/// Last slot must be greater than first slot.
		LastSlotBeforeFirstSlot,
		/// The last slot cannot be more then 3 slots after the first slot.
		LastSlotTooFarInFuture,
		/// The campaign ends before the current block number. The end must be in the future.
		CannotEndInPast,
		/// There was an overflow.
		Overflow,
		/// The contribution was below the minimum, `MinContribution`.
		ContributionTooSmall,
		/// Invalid fund index.
		InvalidParaId,
		/// Contributions exceed maximum amount.
		CapExceeded,
		/// The contribution period has already ended.
		ContributionPeriodOver,
		/// The origin of this call is invalid.
		UnauthorizedAccount,
		/// This crowdloan does not correspond to a parachain.
		NotParachain,
		/// This parachain lease is still active and retirement cannot yet begin.
		LeaseActive,
		/// This parachain's bid or lease is still active and withdraw cannot yet begin.
		BidOrLeaseActive,
		/// Funds have not yet been returned.
		FundsNotReturned,
		/// Fund has not yet retired.
		FundNotRetired,
		/// Fund has not withdrew.
		FundNotWithdrew,
		/// The crowdloan has not yet ended.
		FundNotEnded,
		/// The fund has been registered.
		FundExisted,
		/// Fund has been expired.
		VSBondExpired,
		/// There are no contributions stored in this crowdloan.
		NoContributions,
		/// This crowdloan has an active parachain and cannot be dissolved.
		HasActiveParachain,
		/// The crowdloan is not ready to dissolve. Potentially still has a slot or in retirement
		/// period.
		NotReadyToDissolve,
		/// Invalid signature.
		InvalidSignature,
		/// Invalid fund status.
		InvalidFundStatus,
		/// Insufficient Balance.
		InsufficientBalance,
		/// Crosschain xcm failed
		XcmFailed,
		/// TODO
		ForbidDoubleContributing,
		/// TODO
		NotEnoughCurrencyToSlash,
	}

	/// Tracker for the next available trie index
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
		Option<FundInfo<AccountIdOf<T>, BalanceOf<T>, LeasePeriod>>,
		ValueQuery,
	>;

	/// TODO: docs
	#[pallet::storage]
	#[pallet::getter(fn refund_pool)]
	pub(super) type RefundPool<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// The balance of the token(rely-chain) can be redeemed.
	#[pallet::storage]
	#[pallet::getter(fn redeem_pool)]
	pub(super) type RedeemPool<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn fund_success(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
		) -> DispatchResult {
			Self::check_fund_owner(origin.clone(), index)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Ongoing, Error::<T>::InvalidFundStatus);
			Funds::<T>::mutate(index, |fund| {
				if let Some(fund) = fund {
					fund.status = FundStatus::Success;
				}
			});

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn fund_fail(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			Self::check_fund_owner(origin.clone(), index)?;

			// crownload is failed, so enable the withdrawal function of vsToken/vsBond
			let mut fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Ongoing, Error::<T>::InvalidFundStatus);
			fund.status = FundStatus::Failed;
			Funds::<T>::insert(index, Some(fund.clone()));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn fund_retire(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
		) -> DispatchResult {
			Self::check_fund_owner(origin.clone(), index)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Success, Error::<T>::InvalidFundStatus);
			Funds::<T>::mutate(index, |fund| {
				if let Some(fund) = fund {
					fund.status = FundStatus::Retired;
				}
			});

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn fund_end(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			Self::check_fund_owner(origin.clone(), index)?;

			let mut fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Withdrew, Error::<T>::InvalidFundStatus);
			fund.status = FundStatus::End;
			Funds::<T>::insert(index, Some(fund.clone()));

			Ok(())
		}

		/// TODO: Refactor the docs.
		/// Create a new crowdloaning campaign for a parachain slot deposit for the current auction.
		#[pallet::weight(0)]
		pub fn create(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] cap: BalanceOf<T>,
			#[pallet::compact] first_slot: LeasePeriod,
			#[pallet::compact] last_slot: LeasePeriod,
		) -> DispatchResult {
			let depositor = ensure_signed(origin)?;

			ensure!(!Funds::<T>::contains_key(index), Error::<T>::FundExisted);

			ensure!(first_slot <= last_slot, Error::<T>::LastSlotBeforeFirstSlot);

			let last_slot_limit = first_slot
				.checked_add(((T::SlotLength::get() as u32) - 1).into())
				.ok_or(Error::<T>::FirstSlotTooFarInFuture)?;
			ensure!(last_slot <= last_slot_limit, Error::<T>::LastSlotTooFarInFuture);

			let deposit = T::SubmissionDeposit::get();

			T::MultiCurrency::reserve(Self::token(), &depositor, deposit)?;

			Funds::<T>::insert(
				index,
				Some(FundInfo {
					depositor,
					deposit,
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

		/// TODO: Refactor the docs.
		/// Contribute to a crowd sale. This will transfer some balance over to fund a parachain
		/// slot. It will be withdrawable in two instances: the parachain becomes retired; or the
		/// slot is unable to be purchased and the timeout expires.
		#[pallet::weight(0)]
		pub fn contribute(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] value: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Ongoing, Error::<T>::InvalidFundStatus);

			ensure!(value >= T::MinContribution::get(), Error::<T>::ContributionTooSmall);

			let raised = fund.raised.checked_add(&value).ok_or(Error::<T>::Overflow)?;
			ensure!(raised <= fund.cap, Error::<T>::CapExceeded);

			let (contributed, contributing) = Self::contribution_get(fund.trie_index, &who);
			ensure!(contributing == 0, Error::<T>::ForbidDoubleContributing);
			Self::contribution_put(fund.trie_index, &who, contributed, value);

			Self::xcm_ump_contribute(origin, index, value).map_err(|_e| Error::<T>::XcmFailed)?;

			Self::deposit_event(Event::Contributing(who, index, value));

			Ok(())
		}

		/// Confirm contribute
		#[pallet::weight(0)]
		pub fn confirm_contribute(
			origin: OriginFor<T>,
			#[pallet::compact] who: AccountIdOf<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] is_success: bool,
		) -> DispatchResult {
			let depositor = ensure_signed(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			// TODO: Needed?
			// ensure!(fund.status == FundStatus::Ongoing, Error::<T>::InvalidFundStatus);

			ensure!(depositor == fund.depositor, Error::<T>::UnauthorizedAccount);

			let (contributed, contributing) = Self::contribution_get(fund.trie_index, &who);
			if contributing == Zero::zero() {
				return Ok(());
			}

			#[allow(non_snake_case)]
			let (vsToken, vsBond) = Self::vsAssets(index, fund.first_slot, fund.last_slot);

			if is_success {
				// Issue reserved vsToken/vsBond to contributor
				T::MultiCurrency::deposit(vsToken, &who, contributing)?;
				T::MultiCurrency::reserve(vsToken, &who, contributing)?;
				T::MultiCurrency::deposit(vsBond, &who, contributing)?;
				T::MultiCurrency::reserve(vsBond, &who, contributing)?;

				// Update the raised of fund
				let fund_new =
					FundInfo { raised: fund.raised.saturating_add(contributing), ..fund };
				Funds::<T>::insert(index, Some(fund_new));

				// Update the contribution of who
				let contributed_new = contributed.saturating_add(contributing);
				let contributing_new = Zero::zero();
				Self::contribution_put(fund.trie_index, &who, contributed_new, contributing_new);

				Self::deposit_event(Event::Contributed(who, index, contributing));
			} else {
				// Update the contribution of who
				Self::contribution_put(fund.trie_index, &who, contributed, Zero::zero());

				Self::deposit_event(Event::ContributeFailed(who, index, contributing));
			}

			Ok(())
		}

		/// TODO: Refactor the docs.
		/// Withdraw full balance of the parachain. this function may need to be called multiple
		/// times
		/// - `index`: The parachain to whose crowdloan the contribution was made.
		#[pallet::weight(0)]
		pub fn withdraw(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			let depositor = ensure_signed(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			let can = fund.status == FundStatus::Failed || fund.status == FundStatus::Retired;
			ensure!(can, Error::<T>::InvalidFundStatus);

			ensure!(depositor == fund.depositor, Error::<T>::UnauthorizedAccount);

			Self::xcm_ump_withdraw(origin, index).map_err(|_| Error::<T>::XcmFailed)?;

			Self::deposit_event(Event::Withdrawing(owner, index, fund.raised));

			Ok(())
		}

		/// TODO: Refactor the docs.
		/// Confirm withdraw by fund owner temporarily
		#[pallet::weight(0)]
		pub fn confirm_withdraw(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] is_success: bool,
		) -> DispatchResult {
			let depositor = ensure_signed(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			let can = fund.status == FundStatus::Failed || fund.status == FundStatus::Retired;
			ensure!(can, Error::<T>::InvalidFundStatus);

			ensure!(depositor == fund.depositor, Error::<T>::UnauthorizedAccount);

			let amount_withdrew = fund.raised;

			if is_success {
				if fund.status == FundStatus::Retired {
					// TODO: Remove Redeem-Pool from SALP
					RedeemPool::<T>::set(Self::redeem_pool().saturating_add(amount_withdrew));
				} else if fund.status == FundStatus::Failed {
					RefundPool::<T>::set(Self::redeem_pool().saturating_add(amount_withdrew));
				}

				let fund_new = FundInfo { status: FundStatus::Withdrew, ..fund };
				Funds::<T>::insert(index, Some(fund_new));

				Self::deposit_event(Event::Withdrew(who, index, amount_withdrew));
			} else {
				Self::deposit_event(Event::WithdrawFailed(who, index, amount_withdrew));
			}

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn refund(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Withdrew, Error::<T>::InvalidFundStatus);

			let (contributed, _) = Self::contribution_get(fund.trie_index, &who);

			Self::xcm_ump_refund(origin, index, contributed).map_err(|_| Error::<T>::XcmFailed)?;

			Self::deposit_event(Event::Refunding(who, index, contributed));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn confirm_refund(
			origin: OriginFor<T>,
			#[pallet::compact] who: AccountIdOf<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] is_success: bool,
		) -> DispatchResult {
			let depositor = ensure_signed(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Withdrew, Error::<T>::InvalidFundStatus);

			ensure!(depositor == fund.depositor, Error::<T>::UnauthorizedAccount);

			let (contributed, contributing) = Self::contribution_get(fund.trie_index, &who);
			if contributed == Zero::zero() {
				return Ok(());
			}

			#[allow(non_snake_case)]
			let (vsToken, vsBond) = Self::vsAssets(index, fund.first_slot, fund.last_slot);

			if is_success {
				// Slash the reserved vsToken/vsBond
				let balance = T::MultiCurrency::slash_reserved(vsToken, &who, contributed);
				ensure!(balance != Zero::zero(), Error::<T>::NotEnoughCurrencyToSlash);
				let balance = T::MultiCurrency::slash_reserved(vsBond, &who, contributed);
				ensure!(balance != Zero::zero(), Error::<T>::NotEnoughCurrencyToSlash);

				// Update the contribution of who
				Self::contribution_put(fund.trie_index, &who, Zero::zero(), contributing);

				Self::deposit_event(Event::Refunded(who, index, contributed));
			} else {
				Self::deposit_event(Event::RefundFailed(who, index, contributed));
			}

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn redeem(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			value: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;

			// TODO: Look like the following code be needless
			ensure!(fund.status == FundStatus::Withdrew, Error::<T>::FundNotWithdrew);

			// TODO: Temp solution, move the check to `Assets` later.
			let cur_block = <frame_system::Pallet<T>>::block_number();
			let end_block = (fund.last_slot + 1) * T::LeasePeriod::get();
			ensure!(
				cur_block < end_block || (cur_block - end_block) <= T::VSBondValidPeriod::get(),
				Error::<T>::VSBondExpired
			);

			let new_redeem_balance =
				Self::redeem_pool().checked_sub(&value).ok_or(Error::<T>::InsufficientBalance)?;

			let (_, status) = Self::contribution_get(fund.trie_index, &who);

			// TODO: The people who do `redeem` dont have to be contributors
			// 	Remove it
			ensure!(
				status == ContributionStatus::Contributed || status == ContributionStatus::Redeemed,
				Error::<T>::ContributionInvalid
			);

			let vstoken = Self::vsToken();
			let vsbond = Self::vsbond(index, fund.first_slot, fund.last_slot);
			Self::check_balance(index, &who, value)?;

			// TODO: Fix the bug
			// 	It's no way to know the amount of vsToken/vsBond under a specific lock
			// 	So the bug cannot be fixed by now
			// Lock the vsToken/vsBond.
			T::MultiCurrency::extend_lock(REDEEM_LOCK, vstoken, &who, value)?;
			T::MultiCurrency::extend_lock(REDEEM_LOCK, vsbond, &who, value)?;

			Self::xcm_ump_redeem(origin, index, value).map_err(|_e| Error::<T>::XcmFailed)?;

			RedeemPool::<T>::put(new_redeem_balance);

			let _balance = Self::update_contribution(
				index,
				who.clone(),
				Zero::zero(),
				ContributionStatus::Redeeming,
			)?;

			Self::deposit_event(Event::Redeeming(who, value));

			Ok(())
		}

		/// Confirm redeem by fund owner temporarily
		#[pallet::weight(0)]
		pub fn confirm_redeem(
			origin: OriginFor<T>,
			who: AccountIdOf<T>,
			index: ParaId,
			value: BalanceOf<T>,
			is_success: bool,
		) -> DispatchResult {
			Self::check_fund_owner(origin, index)?;
			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Withdrew, Error::<T>::FundNotWithdrew);
			let (_, status) = Self::contribution_get(fund.trie_index, &who);
			ensure!(status == ContributionStatus::Redeeming, Error::<T>::ContributionInvalid);
			Self::redeem_callback(who, index, value, is_success)
		}

		/// Remove a fund after the retirement period has ended and all funds have been returned.
		#[pallet::weight(0)]
		pub fn dissolve(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			let owner = Self::check_fund_owner(origin, index)?;

			let mut fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::End, Error::<T>::FundNotEnded);

			let mut refund_count = 0u32;
			// Try killing the crowdloan child trie and Assume everyone will be refunded.
			let contributions = Self::contribution_iterator(fund.trie_index);
			let mut all_refunded = true;
			for (who, (balance, _)) in contributions {
				if refund_count >= T::RemoveKeysLimit::get() {
					// Not everyone was able to be refunded this time around.
					all_refunded = false;
					break;
				}
				Self::contribution_kill(fund.trie_index, &who);
				fund.raised = fund.raised.saturating_sub(balance);
				refund_count += 1;
			}

			if all_refunded == true {
				T::MultiCurrency::unreserve(Self::token(), &owner, fund.deposit);
				Funds::<T>::remove(index);
				Self::deposit_event(Event::<T>::Dissolved(index));
			}
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(n: BlockNumberFor<T>) {
			// Release x% KSM/DOT from 1:1 redeem-pool to bancor-pool per cycle.
			if (n % T::ReleaseCycle::get()) == 0 {
				if let Ok(redeem_pool_balance) = TryInto::<u128>::try_into(Self::redeem_pool()) {
					// Calculate the release amount by `(redeem_pool_balance *
					// T::ReleaseRatio).main_part()`.
					let release_amount = T::ReleaseRatio::get() * redeem_pool_balance;

					// Must be ok.
					if let Ok(release_amount) = TryInto::<BalanceOf<T>>::try_into(release_amount) {
						// Decrease the balance of redeem-pool by release amount.
						RedeemPool::<T>::mutate(|b| {
							*b = b.saturating_sub(release_amount);
						});

						// Increase the balance of bancor-pool by release amount.
						if let Err(err) = T::BancorPool::add_token(
							T::RelyChainToken::get().into(),
							release_amount,
						) {
							log::warn!("Bancor: {:?} on bifrost-bancor.", err);
						}
					}
				} else {
					log::warn!("Overflow: The balance of redeem-pool exceeds u128.");
				}
			}

			// TODO: check & lock if vsBond if expired ???
		}

		fn on_initialize(_n: BlockNumberFor<T>) -> frame_support::weights::Weight {
			// TODO estimate weight
			Zero::zero()
		}
	}

	impl<T: Config> Pallet<T> {
		/// The account ID of the fund pot.
		///
		/// This actually does computation. If you need to keep using it, then make sure you cache
		/// the value and only call this once.
		pub fn fund_account_id(index: ParaId) -> AccountIdOf<T> {
			T::PalletId::get().into_sub_account(index)
		}

		pub fn id_from_index(index: TrieIndex) -> child::ChildInfo {
			let mut buf = Vec::new();
			buf.extend_from_slice(&(T::PalletId::get().0));
			buf.extend_from_slice(&index.encode()[..]);
			child::ChildInfo::new_default(T::Hashing::hash(&buf[..]).as_ref())
		}

		pub fn contribution_put(
			index: TrieIndex,
			who: &AccountIdOf<T>,
			contributed: BalanceOf<T>,
			contributing: BalanceOf<T>,
		) {
			who.using_encoded(|b| {
				child::put(&Self::id_from_index(index), b, &(contributed, contributing))
			});
		}

		pub fn contribution_get(
			index: TrieIndex,
			who: &AccountIdOf<T>,
		) -> (BalanceOf<T>, BalanceOf<T>) {
			who.using_encoded(|b| {
				child::get_or_default::<(BalanceOf<T>, BalanceOf<T>)>(
					&Self::id_from_index(index),
					b,
				)
			})
		}

		pub fn contribution_kill(index: TrieIndex, who: &AccountIdOf<T>) {
			who.using_encoded(|b| child::kill(&Self::id_from_index(index), b));
		}

		pub fn contribution_iterator(
			index: TrieIndex,
		) -> ChildTriePrefixIterator<(AccountIdOf<T>, (BalanceOf<T>, BalanceOf<T>))> {
			ChildTriePrefixIterator::<_>::with_prefix_over_key::<Identity>(
				&Self::id_from_index(index),
				&[],
			)
		}

		pub fn crowdloan_kill(index: TrieIndex) -> child::KillChildStorageResult {
			child::kill_storage(&Self::id_from_index(index), Some(T::RemoveKeysLimit::get()))
		}

		// TODO: Combine the xcm function

		pub fn xcm_ump_contribute(
			origin: OriginFor<T>,
			index: ParaId,
			value: BalanceOf<T>,
		) -> XcmResult {
			let origin_location: MultiLocation =
				T::ExecuteXcmOrigin::ensure_origin(origin).map_err(|_e| XcmError::BadOrigin)?;

			let contribution = Contribution { index, value: value.clone(), signature: None };

			let call = CrowdloanContributeCall::CrowdloanContribute(ContributeCall::Contribute(
				contribution,
			))
			.encode()
			.into();

			let amount = TryInto::<u128>::try_into(value).map_err(|_| XcmError::Unimplemented)?;

			let _result = T::BifrostXcmExecutor::ump_transfer_asset(
				origin_location.clone(),
				MultiLocation::X1(Junction::Parachain(index)),
				amount,
				true,
			)?;

			T::BifrostXcmExecutor::ump_transact(origin_location, call)
		}

		pub fn xcm_ump_withdraw(origin: OriginFor<T>, index: ParaId) -> XcmResult {
			let origin_location: MultiLocation =
				T::ExecuteXcmOrigin::ensure_origin(origin).map_err(|_e| XcmError::BadOrigin)?;

			let who: AccountIdOf<T> = PolkadotParaId::from(index).into_account();

			let withdraw = Withdraw { who, index };
			let call = CrowdloanWithdrawCall::CrowdloanWithdraw(WithdrawCall::Withdraw(withdraw))
				.encode()
				.into();
			T::BifrostXcmExecutor::ump_transact(origin_location, call)
		}

		pub fn xcm_ump_refund(
			origin: OriginFor<T>,
			index: ParaId,
			value: BalanceOf<T>,
		) -> XcmResult {
			let origin_location: MultiLocation =
				T::ExecuteXcmOrigin::ensure_origin(origin).map_err(|_e| XcmError::BadOrigin)?;

			let amount = TryInto::<u128>::try_into(value).map_err(|_| XcmError::Unimplemented)?;

			T::BifrostXcmExecutor::ump_transfer_asset(
				MultiLocation::X1(Junction::Parachain(index)),
				origin_location,
				amount,
				false,
			)
		}

		pub fn xcm_ump_redeem(
			origin: OriginFor<T>,
			index: ParaId,
			value: BalanceOf<T>,
		) -> XcmResult {
			let origin_location: MultiLocation =
				T::ExecuteXcmOrigin::ensure_origin(origin).map_err(|_e| XcmError::BadOrigin)?;

			let amount = TryInto::<u128>::try_into(value).map_err(|_| XcmError::Unimplemented)?;

			T::BifrostXcmExecutor::ump_transfer_asset(
				MultiLocation::X1(Junction::Parachain(index)),
				origin_location,
				amount,
				false,
			)
		}

		pub(crate) fn next_trie_index() -> Result<TrieIndex, Error<T>> {
			CurrentTrieIndex::<T>::try_mutate(|ti| {
				*ti = ti.checked_add(1).ok_or(Error::<T>::Overflow)?;
				Ok(*ti - 1)
			})
		}

		pub fn check_balance(
			para_id: ParaId,
			who: &AccountIdOf<T>,
			value: BalanceOf<T>,
		) -> Result<(), Error<T>> {
			let fund = Self::funds(para_id).ok_or(Error::<T>::InvalidParaId)?;
			T::MultiCurrency::ensure_can_withdraw(Self::vsToken(), who, value)
				.map_err(|_e| Error::<T>::InsufficientBalance)?;
			T::MultiCurrency::ensure_can_withdraw(
				Self::vsbond(para_id, fund.first_slot, fund.last_slot),
				&who,
				value,
			)
			.map_err(|_e| Error::<T>::InsufficientBalance)?;
			Ok(())
		}

		fn token() -> CurrencyId {
			#[cfg(feature = "with-asgard-runtime")]
			return CurrencyId::Token(TokenSymbol::ASG);
			#[cfg(not(feature = "with-asgard-runtime"))]
			return CurrencyId::Token(TokenSymbol::BNC);
		}

		#[allow(non_snake_case)]
		pub(crate) fn vsAssets(
			index: ParaId,
			first_slot: LeasePeriod,
			last_slot: LeasePeriod,
		) -> (CurrencyId, CurrencyId) {
			let token_symbol = *T::RelyChainToken::get();

			let vsToken = CurrencyId::VSToken(token_symbol);
			let vsBond = CurrencyId::VSBond(token_symbol, index, first_slot, last_slot);

			(vsToken, vsBond)
		}

		fn redeem_callback(
			who: AccountIdOf<T>,
			index: ParaId,
			value: BalanceOf<T>,
			is_success: bool,
		) -> DispatchResult {
			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;

			let vstoken = Self::vsToken();
			let vsbond = Self::vsbond(index, fund.first_slot, fund.last_slot);

			T::MultiCurrency::remove_lock(REDEEM_LOCK, vstoken, &who)?;
			T::MultiCurrency::remove_lock(REDEEM_LOCK, vsbond, &who)?;

			if is_success {
				// Burn the vsToken/vsBond.
				T::MultiCurrency::withdraw(vstoken, &who, value)?;
				T::MultiCurrency::withdraw(vsbond, &who, value)?;

				// Update contribution trie
				let _balance = Self::update_contribution(
					index,
					who.clone(),
					value,
					ContributionStatus::Redeemed,
				)?;

				Self::deposit_event(Event::Redeemed(who, value));
			} else {
				// Revoke the redeem pool.
				let new_redeem_balance = Self::redeem_pool().saturating_add(value);
				RedeemPool::<T>::put(new_redeem_balance);

				Self::deposit_event(Event::RedeemFailed(who, value));
			}

			Ok(())
		}
	}

	pub const fn vslock(index: ParaId) -> LockIdentifier {
		(index as u64).to_be_bytes()
	}

	const REDEEM_LOCK: LockIdentifier = *b"REDEEMLC";
}
