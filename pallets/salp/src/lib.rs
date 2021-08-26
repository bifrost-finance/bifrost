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

pub mod migration {
	pub fn migrate() {
		log::info!("salp migration...");
	}
}

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

// Re-export pallet items so that they can be accessed from the crate namespace.
use frame_support::{pallet_prelude::*, transactional};
use node_primitives::{TokenInfo, TokenSymbol};
use orml_traits::MultiCurrency;
pub use pallet::*;
use sp_std::convert::TryFrom;

type TrieIndex = u32;

pub trait WeightInfo {
	fn create() -> Weight;
	fn contribute() -> Weight;
	fn unlock() -> Weight;
	fn withdraw() -> Weight;
	fn refund() -> Weight;
	fn redeem() -> Weight;
	fn dissolve(n: u32) -> Weight;
	fn on_initialize(n: u32) -> Weight;
}

#[allow(type_alias_bounds)]
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[allow(type_alias_bounds)]
type BalanceOf<T: Config> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub enum FundStatus {
	Ongoing,
	Retired,
	Success,
	Failed,
	RefundWithdrew,
	RedeemWithdrew,
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
pub struct FundInfo<Balance, LeasePeriod> {
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

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
#[codec(dumb_trait_bound)]
pub struct ContributionMemoInfo {
	index: TrieIndex,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, Copy)]
pub enum ContributionStatus<BalanceOf> {
	Idle,
	Refunded,
	Unlocked,
	Refunding,
	Contributing(BalanceOf),
}

impl<BalanceOf> ContributionStatus<BalanceOf>
where
	BalanceOf: frame_support::sp_runtime::traits::Zero + Clone + Copy,
{
	pub fn is_contributing(&self) -> bool {
		match self {
			Self::Contributing(_) => true,
			_ => false,
		}
	}

	pub fn contributing(&self) -> BalanceOf {
		match self {
			Self::Contributing(contributing) => *contributing,
			_ => frame_support::sp_runtime::traits::Zero::zero(),
		}
	}
}

impl<BalanceOf> Default for ContributionStatus<BalanceOf> {
	fn default() -> Self {
		Self::Idle
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, Copy)]
pub enum RedeemStatus<BalanceOf> {
	Idle,
	Redeeming(BalanceOf),
}

impl<BalanceOf> RedeemStatus<BalanceOf>
where
	BalanceOf: frame_support::sp_runtime::traits::Zero + Clone + Copy,
{
	pub fn is_redeeming(&self) -> bool {
		match self {
			Self::Redeeming(..) => true,
			_ => false,
		}
	}

	pub fn redeeming(&self) -> BalanceOf {
		match self {
			Self::Redeeming(redeeming) => *redeeming,
			_ => frame_support::sp_runtime::traits::Zero::zero(),
		}
	}
}

impl<BalanceOf> Default for RedeemStatus<BalanceOf> {
	fn default() -> Self {
		Self::Idle
	}
}

#[frame_support::pallet]
pub mod pallet {
	// Import various types used to declare pallet in scope.
	use frame_support::{
		pallet_prelude::{storage::child, *},
		sp_runtime::traits::{AccountIdConversion, CheckedAdd, Hash, Saturating, Zero},
		storage::ChildTriePrefixIterator,
		weights::WeightToFeePolynomial,
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use node_primitives::{
		BancorHandler, CurrencyId, LeasePeriod, MessageId, ParaId, ParachainTransactProxyType,
		ParachainTransactType, TransferOriginType,
	};
	use orml_traits::{currency::TransferAll, MultiCurrency, MultiReservableCurrency, XcmTransfer};
	use polkadot_parachain::primitives::Id as PolkadotParaId;
	use sp_arithmetic::Percent;
	use sp_std::{convert::TryInto, prelude::*};
	use xcm::v0::{
		prelude::{XcmError, XcmResult},
		Junction, MultiLocation,
	};
	use xcm_support::*;

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
		type RelayChainToken: Get<CurrencyId>;

		#[pallet::constant]
		type DepositToken: Get<CurrencyId>;

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

		#[pallet::constant]
		type SlotLength: Get<LeasePeriod>;

		type MultiCurrency: TransferAll<AccountIdOf<Self>>
			+ MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiReservableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type BancorPool: BancorHandler<BalanceOf<Self>>;

		type ExecuteXcmOrigin: EnsureOrigin<
			<Self as frame_system::Config>::Origin,
			Success = MultiLocation,
		>;

		type EnsureConfirmAsMultiSig: EnsureOrigin<<Self as frame_system::Config>::Origin>;

		type BifrostXcmExecutor: BifrostXcmExecutor;

		#[pallet::constant]
		type XcmTransferOrigin: Get<TransferOriginType>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;

		/// Parachain Id
		type SelfParaId: Get<u32>;

		/// Weight to Fee calculator
		type WeightToFee: WeightToFeePolynomial<Balance = BalanceOf<Self>>;

		/// Xcm weight
		#[pallet::constant]
		type BaseXcmWeight: Get<u64>;

		#[pallet::constant]
		type ContributionWeight: Get<u64>;

		#[pallet::constant]
		type WithdrawWeight: Get<u64>;

		#[pallet::constant]
		type AddProxyWeight: Get<u64>;

		#[pallet::constant]
		type RemoveProxyWeight: Get<u64>;

		/// The interface to Cross-chain transfer.
		type XcmTransfer: XcmTransfer<AccountIdOf<Self>, BalanceOf<Self>, CurrencyId>;

		/// The sovereign sub-account for where the staking currencies are sent to.
		#[pallet::constant]
		type SovereignSubAccountLocation: Get<MultiLocation>;

		#[pallet::constant]
		type TransactProxyType: Get<ParachainTransactProxyType>;

		#[pallet::constant]
		type TransactType: Get<ParachainTransactType>;
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
		Contributing(AccountIdOf<T>, ParaId, BalanceOf<T>, MessageId),
		/// Contributed to a crowd sale. [who, fund_index, amount]
		Contributed(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Fail on contribute to crowd sale. [who, fund_index, amount]
		ContributeFailed(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Withdrawing full balance of a contributor. [who, fund_index, amount]
		Withdrawing(ParaId, BalanceOf<T>),
		/// Withdrew full balance of a contributor. [who, fund_index, amount]
		Withdrew(ParaId, BalanceOf<T>),
		/// Fail on withdraw full balance of a contributor. [who, fund_index, amount]
		WithdrawFailed(ParaId, BalanceOf<T>),
		/// Refunding to account. [who, fund_index, amount]
		Refunding(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Refunded to account. [who, fund_index, amount]
		Refunded(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Fail on refund to account. [who,fund_index, amount]
		RefundFailed(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Redeeming to account. [who, fund_index, first_slot, last_slot, value]
		Redeeming(AccountIdOf<T>, ParaId, LeasePeriod, LeasePeriod, BalanceOf<T>),
		/// Redeemed to account. [who, fund_index, first_slot, last_slot, value]
		Redeemed(AccountIdOf<T>, ParaId, LeasePeriod, LeasePeriod, BalanceOf<T>),
		/// Fail on redeem to account. [who, fund_index, first_slot, last_slot, value]
		RedeemFailed(AccountIdOf<T>, ParaId, LeasePeriod, LeasePeriod, BalanceOf<T>),
		/// Fund is dissolved. [fund_index]
		Dissolved(ParaId),
		/// The vsToken/vsBond was be unlocked. [who, fund_index, value]
		Unlocked(AccountIdOf<T>, ParaId, BalanceOf<T>),
		AllUnlocked(ParaId),
		/// Proxy
		ProxyAdded(AccountIdOf<T>),
		ProxyRemoved(AccountIdOf<T>),
		/// Mint
		Minted(AccountIdOf<T>, BalanceOf<T>),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The first slot needs to at least be less than 3 `max_value`.
		FirstSlotTooFarInFuture,
		/// Last slot must be greater than first slot.
		LastSlotBeforeFirstSlot,
		/// The last slot cannot be more then 3 slots after the first slot.
		LastSlotTooFarInFuture,
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
		/// The origin of this call is invalid.
		UnauthorizedAccount,
		/// The fund has been registered.
		FundAlreadyCreated,
		/// Crosschain xcm failed
		XcmFailed,
		/// Don't have enough vsToken/vsBond to refund
		NotEnoughReservedAssetsToRefund,
		/// Don't have enough token to refund by users
		NotEnoughBalanceInRefundPool,
		/// Don't have enough vsToken/vsBond to unlock
		NotEnoughBalanceToUnlock,
		/// The vsBond is expired now
		VSBondExpired,
		/// The vsBond cannot be redeemed by now
		UnRedeemableNow,
		/// Dont have enough vsToken/vsBond to redeem
		NotEnoughFreeAssetsToRedeem,
		/// Dont have enough vsToken/vsBond to unlock when redeem failed
		NotEnoughReservedAssetsToUnlockWhenRedeemFailed,
		/// Don't have enough token to redeem by users
		NotEnoughBalanceInRedeemPool,
		/// Invalid redeem status
		InvalidRedeemStatus,
	}

	/// Tracker for the next available trie index
	#[pallet::storage]
	#[pallet::getter(fn current_trie_index)]
	pub(super) type CurrentTrieIndex<T: Config> = StorageValue<_, TrieIndex, ValueQuery>;

	/// Tracker for the next available trie index
	#[pallet::storage]
	#[pallet::getter(fn current_contribution_index)]
	pub(super) type CurrentContributionIndex<T: Config> = StorageValue<_, TrieIndex, ValueQuery>;

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

	/// The balance can be refunded to users.
	#[pallet::storage]
	#[pallet::getter(fn refund_pool)]
	pub(super) type RefundPool<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// The balance can be redeemed to users.
	#[pallet::storage]
	#[pallet::getter(fn redeem_pool)]
	pub(super) type RedeemPool<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn redeem_status)]
	pub(super) type RedeemExtras<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		Blake2_128Concat,
		(ParaId, LeasePeriod, LeasePeriod),
		RedeemStatus<BalanceOf<T>>,
		ValueQuery,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		pub fn fund_success(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
		) -> DispatchResult {
			T::EnsureConfirmAsMultiSig::ensure_origin(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Ongoing, Error::<T>::InvalidFundStatus);

			let fund_new = FundInfo { status: FundStatus::Success, ..fund };
			Funds::<T>::insert(index, Some(fund_new));

			Ok(())
		}

		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		pub fn fund_fail(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			T::EnsureConfirmAsMultiSig::ensure_origin(origin)?;

			// crownload is failed, so enable the withdrawal function of vsToken/vsBond
			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Ongoing, Error::<T>::InvalidFundStatus);

			let fund_new = FundInfo { status: FundStatus::Failed, ..fund };
			Funds::<T>::insert(index, Some(fund_new));

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
			T::EnsureConfirmAsMultiSig::ensure_origin(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Success, Error::<T>::InvalidFundStatus);

			let fund_new = FundInfo { status: FundStatus::Retired, ..fund };
			Funds::<T>::insert(index, Some(fund_new));

			Ok(())
		}

		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		pub fn fund_end(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			T::EnsureConfirmAsMultiSig::ensure_origin(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(
				fund.status == FundStatus::RefundWithdrew ||
					fund.status == FundStatus::RedeemWithdrew,
				Error::<T>::InvalidFundStatus
			);

			let fund_new = FundInfo { status: FundStatus::End, ..fund };
			Funds::<T>::insert(index, Some(fund_new));

			Ok(())
		}

		/// Unlock the reserved vsToken/vsBond after fund success
		#[pallet::weight(T::WeightInfo::unlock())]
		pub fn unlock(
			_origin: OriginFor<T>,
			who: AccountIdOf<T>,
			#[pallet::compact] index: ParaId,
		) -> DispatchResult {
			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(
				fund.status == FundStatus::Success ||
					fund.status == FundStatus::Retired ||
					fund.status == FundStatus::RedeemWithdrew ||
					fund.status == FundStatus::End,
				Error::<T>::InvalidFundStatus
			);

			let (contributed, status) = Self::contribution(fund.trie_index, &who);
			ensure!(status == ContributionStatus::Idle, Error::<T>::InvalidContributionStatus);

			#[allow(non_snake_case)]
			let (vsToken, vsBond) = Self::vsAssets(index, fund.first_slot, fund.last_slot);

			let balance = T::MultiCurrency::unreserve(vsToken, &who, contributed);
			ensure!(balance == Zero::zero(), Error::<T>::NotEnoughBalanceToUnlock);
			let balance = T::MultiCurrency::unreserve(vsBond, &who, contributed);
			ensure!(balance == Zero::zero(), Error::<T>::NotEnoughBalanceToUnlock);

			Self::put_contribution(
				fund.trie_index,
				&who,
				contributed,
				ContributionStatus::Unlocked,
			);

			Self::deposit_event(Event::<T>::Unlocked(who, index, contributed));

			Ok(())
		}

		/// Unlock the reserved vsToken/vsBond after fund success
		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		pub fn batch_unlock(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
		) -> DispatchResult {
			T::EnsureConfirmAsMultiSig::ensure_origin(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(
				fund.status == FundStatus::Success ||
					fund.status == FundStatus::Retired ||
					fund.status == FundStatus::RedeemWithdrew ||
					fund.status == FundStatus::End,
				Error::<T>::InvalidFundStatus
			);

			let mut unlock_count = 0u32;
			let contributions = Self::contribution_iterator(fund.trie_index);
			// Assume everyone will be refunded.
			let mut all_unlocked = true;

			for (who, (contributed, status)) in contributions {
				if unlock_count >= T::RemoveKeysLimit::get() {
					// Not everyone was able to be refunded this time around.
					all_unlocked = false;
					break;
				}
				if status != ContributionStatus::Unlocked {
					#[allow(non_snake_case)]
					let (vsToken, vsBond) = Self::vsAssets(index, fund.first_slot, fund.last_slot);
					let balance = T::MultiCurrency::unreserve(vsToken, &who, contributed);
					ensure!(balance == Zero::zero(), Error::<T>::NotEnoughBalanceToUnlock);
					let balance = T::MultiCurrency::unreserve(vsBond, &who, contributed);
					ensure!(balance == Zero::zero(), Error::<T>::NotEnoughBalanceToUnlock);

					Self::put_contribution(
						fund.trie_index,
						&who,
						contributed,
						ContributionStatus::Unlocked,
					);
					unlock_count += 1;
				}
			}

			if all_unlocked {
				Self::deposit_event(Event::<T>::AllUnlocked(index));
			}

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
			T::EnsureConfirmAsMultiSig::ensure_origin(origin)?;

			ensure!(!Funds::<T>::contains_key(index), Error::<T>::FundAlreadyCreated);

			ensure!(first_slot <= last_slot, Error::<T>::LastSlotBeforeFirstSlot);

			let last_slot_limit = first_slot
				.checked_add(((T::SlotLength::get() as u32) - 1).into())
				.ok_or(Error::<T>::FirstSlotTooFarInFuture)?;
			ensure!(last_slot <= last_slot_limit, Error::<T>::LastSlotTooFarInFuture);

			let deposit = T::SubmissionDeposit::get();

			Funds::<T>::insert(
				index,
				Some(FundInfo {
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

		/// Contribute to a crowd sale. This will transfer some balance over to fund a parachain
		/// slot. It will be withdrawable in two instances: the parachain becomes retired; or the
		/// slot is unable to be purchased and the timeout expires.
		#[pallet::weight(T::WeightInfo::contribute())]
		#[transactional]
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

			let (contributed, status) = Self::contribution(fund.trie_index, &who);
			ensure!(status == ContributionStatus::Idle, Error::<T>::InvalidContributionStatus);

			if T::TransactType::get() == ParachainTransactType::Xcm &&
				T::XcmTransferOrigin::get() == TransferOriginType::FromRelayChain
			{
				T::MultiCurrency::reserve(T::RelayChainToken::get(), &who, value)?;
			}

			Self::put_contribution(
				fund.trie_index,
				&who,
				contributed,
				ContributionStatus::Contributing(value),
			);

			let contribution_index: MessageId;

			if T::TransactType::get() == ParachainTransactType::Xcm {
				contribution_index = Self::xcm_ump_contribute(origin, index, value)
					.map_err(|_e| Error::<T>::XcmFailed)?;
			} else {
				contribution_index =
					sp_io::hashing::blake2_256(&Self::next_contribution_index()?.encode());
				if T::TransactProxyType::get() == ParachainTransactProxyType::Derived {
					Self::xcm_ump_transfer(who.clone(), value)?;
				}
			}
			Self::deposit_event(Event::Contributing(
				who.clone(),
				index,
				value.clone(),
				contribution_index,
			));
			Ok(())
		}

		/// Confirm contribute
		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		pub fn confirm_contribute(
			origin: OriginFor<T>,
			who: AccountIdOf<T>,
			#[pallet::compact] index: ParaId,
			is_success: bool,
			_contribution_index: MessageId,
		) -> DispatchResult {
			T::EnsureConfirmAsMultiSig::ensure_origin(origin)?;
			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			let can_confirm = fund.status == FundStatus::Ongoing ||
				fund.status == FundStatus::Failed ||
				fund.status == FundStatus::Success;
			ensure!(can_confirm, Error::<T>::InvalidFundStatus);

			let (contributed, status) = Self::contribution(fund.trie_index, &who);
			ensure!(status.is_contributing(), Error::<T>::InvalidContributionStatus);
			let contributing = status.contributing();

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

				if T::TransactType::get() == ParachainTransactType::Xcm &&
					T::XcmTransferOrigin::get() == TransferOriginType::FromRelayChain
				{
					T::MultiCurrency::withdraw(T::RelayChainToken::get(), &who, contributing)?;
				}

				// Update the contribution of who
				let contributed_new = contributed.saturating_add(contributing);
				Self::put_contribution(
					fund.trie_index,
					&who,
					contributed_new,
					ContributionStatus::Idle,
				);

				Self::deposit_event(Event::Contributed(who, index, contributing));
			} else {
				// Update the contribution of who
				Self::put_contribution(
					fund.trie_index,
					&who,
					contributed,
					ContributionStatus::Idle,
				);
				if T::TransactType::get() == ParachainTransactType::Xcm &&
					T::XcmTransferOrigin::get() == TransferOriginType::FromRelayChain
				{
					T::MultiCurrency::unreserve(T::RelayChainToken::get(), &who, contributing);
				}

				Self::deposit_event(Event::ContributeFailed(who, index, contributing));
			}

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
			T::EnsureConfirmAsMultiSig::ensure_origin(origin.clone())?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			let can = fund.status == FundStatus::Failed || fund.status == FundStatus::Retired;
			ensure!(can, Error::<T>::InvalidFundStatus);

			if T::TransactType::get() == ParachainTransactType::Xcm {
				Self::xcm_ump_withdraw(origin, index).map_err(|_| Error::<T>::XcmFailed)?;
			}

			Self::deposit_event(Event::Withdrawing(index, fund.raised));

			Ok(())
		}

		/// Confirm withdraw by fund owner temporarily
		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		pub fn confirm_withdraw(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			is_success: bool,
		) -> DispatchResult {
			T::EnsureConfirmAsMultiSig::ensure_origin(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			let can = fund.status == FundStatus::Failed || fund.status == FundStatus::Retired;
			ensure!(can, Error::<T>::InvalidFundStatus);

			let amount_withdrew = fund.raised;

			if is_success {
				if fund.status == FundStatus::Retired {
					RedeemPool::<T>::set(Self::redeem_pool().saturating_add(amount_withdrew));

					let fund_new = FundInfo { status: FundStatus::RedeemWithdrew, ..fund };
					Funds::<T>::insert(index, Some(fund_new));
				} else if fund.status == FundStatus::Failed {
					RefundPool::<T>::set(Self::refund_pool().saturating_add(amount_withdrew));

					let fund_new = FundInfo { status: FundStatus::RefundWithdrew, ..fund };
					Funds::<T>::insert(index, Some(fund_new));
				}

				Self::deposit_event(Event::Withdrew(index, amount_withdrew));
			} else {
				Self::deposit_event(Event::WithdrawFailed(index, amount_withdrew));
			}

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::refund())]
		#[transactional]
		pub fn refund(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::RefundWithdrew, Error::<T>::InvalidFundStatus);

			let (contributed, status) = Self::contribution(fund.trie_index, &who);
			ensure!(contributed > Zero::zero(), Error::<T>::ZeroContribution);
			ensure!(status == ContributionStatus::Idle, Error::<T>::InvalidContributionStatus);

			ensure!(Self::refund_pool() >= contributed, Error::<T>::NotEnoughBalanceInRefundPool);

			#[allow(non_snake_case)]
			let (vsToken, vsBond) = Self::vsAssets(index, fund.first_slot, fund.last_slot);
			ensure!(
				T::MultiCurrency::reserved_balance(vsToken, &who) >= contributed,
				Error::<T>::NotEnoughReservedAssetsToRefund
			);
			ensure!(
				T::MultiCurrency::reserved_balance(vsBond, &who) >= contributed,
				Error::<T>::NotEnoughReservedAssetsToRefund
			);

			if T::TransactType::get() == ParachainTransactType::Xcm {
				Self::xcm_ump_redeem(origin, index, contributed)
					.map_err(|_| Error::<T>::XcmFailed)?;
			}

			RefundPool::<T>::set(Self::refund_pool().saturating_sub(contributed));

			Self::put_contribution(
				fund.trie_index,
				&who,
				contributed,
				ContributionStatus::Refunding,
			);

			Self::deposit_event(Event::Refunding(who, index, contributed));

			Ok(())
		}

		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		pub fn confirm_refund(
			origin: OriginFor<T>,
			who: AccountIdOf<T>,
			#[pallet::compact] index: ParaId,
			is_success: bool,
		) -> DispatchResult {
			T::EnsureConfirmAsMultiSig::ensure_origin(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::RefundWithdrew, Error::<T>::InvalidFundStatus);

			let (contributed, status) = Self::contribution(fund.trie_index, &who);
			ensure!(status == ContributionStatus::Refunding, Error::<T>::InvalidContributionStatus);

			#[allow(non_snake_case)]
			let (vsToken, vsBond) = Self::vsAssets(index, fund.first_slot, fund.last_slot);

			if is_success {
				// Slash the reserved vsToken/vsBond
				let balance = T::MultiCurrency::slash_reserved(vsToken, &who, contributed);
				ensure!(balance == Zero::zero(), Error::<T>::NotEnoughReservedAssetsToRefund);
				let balance = T::MultiCurrency::slash_reserved(vsBond, &who, contributed);
				ensure!(balance == Zero::zero(), Error::<T>::NotEnoughReservedAssetsToRefund);

				Self::put_contribution(
					fund.trie_index,
					&who,
					contributed,
					ContributionStatus::Refunded,
				);

				Self::deposit_event(Event::Refunded(who, index, contributed));
			} else {
				RefundPool::<T>::set(Self::refund_pool().saturating_add(contributed));

				Self::put_contribution(
					fund.trie_index,
					&who,
					contributed,
					ContributionStatus::Idle,
				);

				Self::deposit_event(Event::RefundFailed(who, index, contributed));
			}

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

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::RedeemWithdrew, Error::<T>::InvalidFundStatus);
			ensure!(Self::redeem_pool() >= value, Error::<T>::NotEnoughBalanceInRedeemPool);
			let cur_block = <frame_system::Pallet<T>>::block_number();
			ensure!(!Self::is_expired(cur_block, fund.last_slot), Error::<T>::VSBondExpired);
			ensure!(Self::can_redeem(cur_block, fund.last_slot), Error::<T>::UnRedeemableNow);
			#[allow(non_snake_case)]
			let (vsToken, vsBond) = Self::vsAssets(index, fund.first_slot, fund.last_slot);

			T::MultiCurrency::reserve(vsToken, &who, value)
				.map_err(|_| Error::<T>::NotEnoughFreeAssetsToRedeem)?;
			T::MultiCurrency::reserve(vsBond, &who, value)
				.map_err(|_| Error::<T>::NotEnoughFreeAssetsToRedeem)?;

			let status = Self::redeem_status(who.clone(), (index, fund.first_slot, fund.last_slot));
			ensure!(status == RedeemStatus::Idle, Error::<T>::InvalidRedeemStatus);

			if T::TransactType::get() == ParachainTransactType::Xcm {
				Self::xcm_ump_redeem(origin.clone(), index, value)
					.map_err(|_| Error::<T>::XcmFailed)?;
			}

			RedeemPool::<T>::set(Self::redeem_pool().saturating_sub(value));

			RedeemExtras::<T>::insert(
				who.clone(),
				(index, fund.first_slot, fund.last_slot),
				RedeemStatus::Redeeming(value),
			);

			Self::deposit_event(Event::Redeeming(
				who,
				index,
				fund.first_slot,
				fund.last_slot,
				value,
			));

			Ok(())
		}

		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		pub fn confirm_redeem(
			origin: OriginFor<T>,
			who: AccountIdOf<T>,
			#[pallet::compact] index: ParaId,
			is_success: bool,
		) -> DispatchResult {
			use RedeemStatus as RS;

			T::EnsureConfirmAsMultiSig::ensure_origin(origin)?;
			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::RedeemWithdrew, Error::<T>::InvalidFundStatus);

			let status = Self::redeem_status(who.clone(), (index, fund.first_slot, fund.last_slot));
			ensure!(status.is_redeeming(), Error::<T>::InvalidRedeemStatus);
			let value = status.redeeming();

			#[allow(non_snake_case)]
			let (vsToken, vsBond) = Self::vsAssets(index, fund.first_slot, fund.last_slot);

			if is_success {
				let balance = T::MultiCurrency::slash_reserved(vsToken, &who, value);
				ensure!(balance == Zero::zero(), Error::<T>::NotEnoughFreeAssetsToRedeem);
				let balance = T::MultiCurrency::slash_reserved(vsBond, &who, value);
				ensure!(balance == Zero::zero(), Error::<T>::NotEnoughFreeAssetsToRedeem);

				RedeemExtras::<T>::insert(
					who.clone(),
					(index, fund.first_slot, fund.last_slot),
					RS::Idle,
				);

				Self::deposit_event(Event::Redeemed(
					who,
					index,
					fund.first_slot,
					fund.last_slot,
					value,
				));
			} else {
				let balance = T::MultiCurrency::unreserve(vsToken, &who, value);
				ensure!(
					balance == Zero::zero(),
					Error::<T>::NotEnoughReservedAssetsToUnlockWhenRedeemFailed
				);
				let balance = T::MultiCurrency::unreserve(vsBond, &who, value);
				ensure!(
					balance == Zero::zero(),
					Error::<T>::NotEnoughReservedAssetsToUnlockWhenRedeemFailed
				);

				RedeemPool::<T>::set(Self::redeem_pool().saturating_add(value));

				RedeemExtras::<T>::insert(
					who.clone(),
					(index, fund.first_slot, fund.last_slot),
					RS::Idle,
				);

				Self::deposit_event(Event::RedeemFailed(
					who,
					index,
					fund.first_slot,
					fund.last_slot,
					value,
				));
			}

			Ok(())
		}

		/// Remove a fund after the retirement period has ended and all funds have been returned.
		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		pub fn dissolve(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			T::EnsureConfirmAsMultiSig::ensure_origin(origin)?;

			let mut fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::End, Error::<T>::InvalidFundStatus);

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
				Self::kill_contribution(fund.trie_index, &who);
				fund.raised = fund.raised.saturating_sub(balance);
				refund_count += 1;
			}

			if all_refunded == true {
				Funds::<T>::remove(index);
				Self::deposit_event(Event::<T>::Dissolved(index));
			}

			Ok(())
		}

		/// Add proxy for parachain account
		/// - `delegate`: The delegate proxy account
		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		#[transactional]
		pub fn add_proxy(origin: OriginFor<T>, delegate: AccountIdOf<T>) -> DispatchResult {
			T::EnsureConfirmAsMultiSig::ensure_origin(origin)?;

			Self::xcm_ump_add_proxy(delegate.clone()).map_err(|_| Error::<T>::XcmFailed)?;

			Self::deposit_event(Event::ProxyAdded(delegate));

			Ok(())
		}

		/// Add proxy for parachain account
		/// - `delegate`: The delegate proxy account
		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		#[transactional]
		pub fn remove_proxy(origin: OriginFor<T>, delegate: AccountIdOf<T>) -> DispatchResult {
			T::EnsureConfirmAsMultiSig::ensure_origin(origin)?;

			Self::xcm_ump_remove_proxy(delegate.clone()).map_err(|_| Error::<T>::XcmFailed)?;

			Self::deposit_event(Event::ProxyRemoved(delegate));

			Ok(())
		}

		/// transfer to parachain salp account
		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		#[transactional]
		pub fn mint(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Self::xcm_ump_transfer(who.clone(), amount.clone())?;

			Self::deposit_event(Event::<T>::Minted(who, amount));

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
						RedeemPool::<T>::set(Self::redeem_pool().saturating_sub(release_amount));

						// Increase the balance of bancor-pool by release-amount
						if let Err(err) =
							T::BancorPool::add_token(T::RelayChainToken::get(), release_amount)
						{
							log::warn!("Bancor: {:?} on bifrost-bancor.", err);
						}
					} else {
						log::warn!("Overflow: The balance of redeem-pool exceeds u128.");
					}
				}
			}
			<T as Config>::WeightInfo::on_initialize(n)

			// TODO: Auto unlock vsToken/vsBond?
		}
	}

	impl<T: Config> Pallet<T> {
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
		) -> Result<BalanceOf<T>, Error<T>> {
			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			let (contributed, _) = Self::contribution(fund.trie_index, who);
			Ok(contributed)
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

		pub(crate) fn next_contribution_index() -> Result<TrieIndex, Error<T>> {
			CurrentContributionIndex::<T>::try_mutate(|ti| {
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
			let token_symbol = TokenSymbol::try_from(tokensymbo_bit).unwrap_or(TokenSymbol::KSM);

			let vsToken = CurrencyId::VSToken(token_symbol);
			let vsBond = CurrencyId::VSBond(token_symbol, index, first_slot, last_slot);

			(vsToken, vsBond)
		}

		/// Check if the vsBond is `past` the redeemable date
		pub(crate) fn is_expired(block: BlockNumberFor<T>, last_slot: LeasePeriod) -> bool {
			let block_begin_redeem = Self::block_end_of_lease_period_index(last_slot);
			let block_end_redeem = block_begin_redeem + T::VSBondValidPeriod::get();

			block >= block_end_redeem
		}

		/// Check if the vsBond is `in` the redeemable date
		pub(crate) fn can_redeem(block: BlockNumberFor<T>, last_slot: LeasePeriod) -> bool {
			let block_begin_redeem = Self::block_end_of_lease_period_index(last_slot);
			let block_end_redeem = block_begin_redeem + T::VSBondValidPeriod::get();

			block >= block_begin_redeem && block < block_end_redeem
		}

		#[allow(unused)]
		pub(crate) fn block_start_of_lease_period_index(slot: LeasePeriod) -> BlockNumberFor<T> {
			slot * T::LeasePeriod::get()
		}

		pub(crate) fn block_end_of_lease_period_index(slot: LeasePeriod) -> BlockNumberFor<T> {
			(slot + 1) * T::LeasePeriod::get()
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

		fn xcm_ump_contribute(
			origin: OriginFor<T>,
			index: ParaId,
			mut value: BalanceOf<T>,
		) -> Result<MessageId, XcmError> {
			let _who = ensure_signed(origin).map_err(|_e| XcmError::BadOrigin)?;

			let fee: BalanceOf<T> = T::WeightToFee::calc(&T::BifrostXcmExecutor::transact_weight(
				T::ContributionWeight::get(),
			));
			value = value.saturating_sub(fee);

			let contribute_call = CrowdloanContributeCall::CrowdloanContribute(
				ContributeCall::Contribute(Contribution { index, value, signature: None }),
			)
			.encode()
			.into();

			T::BifrostXcmExecutor::ump_transact(
				MultiLocation::Null,
				contribute_call,
				T::ContributionWeight::get(),
				false,
			)
		}

		fn xcm_ump_withdraw(_origin: OriginFor<T>, index: ParaId) -> Result<[u8; 32], XcmError> {
			let who: AccountIdOf<T> = PolkadotParaId::from(T::SelfParaId::get()).into_account();

			let withdraw = Withdraw { who, index };
			let call = CrowdloanWithdrawCall::CrowdloanWithdraw(WithdrawCall::Withdraw(withdraw))
				.encode()
				.into();

			T::BifrostXcmExecutor::ump_transact(
				MultiLocation::X1(Junction::Parachain(index)),
				call,
				T::WithdrawWeight::get(),
				false,
			)
		}

		fn xcm_ump_redeem(origin: OriginFor<T>, index: ParaId, value: BalanceOf<T>) -> XcmResult {
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

		fn xcm_ump_add_proxy(delegate: AccountIdOf<T>) -> Result<[u8; 32], XcmError> {
			let call = ProxyAddCall::ProxyAdd(AddProxyCall::Add(AddProxy {
				delegate,
				proxy_type: ProxyType::Any,
				delay: T::BlockNumber::zero(),
			}))
			.encode()
			.into();

			T::BifrostXcmExecutor::ump_transact(
				MultiLocation::Null,
				call,
				T::AddProxyWeight::get(),
				false,
			)
		}

		fn xcm_ump_remove_proxy(delegate: AccountIdOf<T>) -> Result<[u8; 32], XcmError> {
			let call = ProxyRemoveCall::ProxyRemove(RemoveProxyCall::Remove(RemoveProxy {
				delegate,
				proxy_type: ProxyType::Any,
				delay: T::BlockNumber::zero(),
			}))
			.encode()
			.into();

			T::BifrostXcmExecutor::ump_transact(
				MultiLocation::Null,
				call,
				T::AddProxyWeight::get(),
				false,
			)
		}

		fn xcm_ump_transfer(who: AccountIdOf<T>, amount: BalanceOf<T>) -> DispatchResult {
			T::XcmTransfer::transfer(
				who.clone(),
				T::RelayChainToken::get(),
				amount,
				T::SovereignSubAccountLocation::get(),
				3 * T::BaseXcmWeight::get(),
			)
		}
	}
}
