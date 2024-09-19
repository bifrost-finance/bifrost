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

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;
pub mod weights;
pub use weights::WeightInfo;

// Re-export pallet items so that they can be accessed from the crate namespace.
use bifrost_primitives::{
	ContributionStatus, CurrencyIdConversion, CurrencyIdRegister, TrieIndex, TryConvertFrom,
	VtokenMintingInterface,
};
use bifrost_stable_pool::{traits::StablePoolHandler, StableAssetPoolId};
use bifrost_xcm_interface::ChainId;
use cumulus_primitives_core::{QueryId, Response};
use frame_support::{pallet_prelude::*, sp_runtime::SaturatedConversion, traits::LockIdentifier};
use orml_traits::MultiCurrency;
pub use pallet::*;
use pallet_xcm::ensure_response;
use scale_info::TypeInfo;
use sp_runtime::traits::One;
use zenlink_protocol::{AssetId, ExportZenlink};

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum FundStatus {
	Ongoing,
	Retired,
	Success,
	Failed,
	RefundWithdrew,
	RedeemWithdrew,
	FailedToContinue,
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
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
#[codec(dumb_trait_bound)]
pub struct FundInfo<Balance, LeasePeriod> {
	/// The total amount raised.
	pub raised: Balance,
	/// A hard-cap on the amount that may be contributed.
	pub cap: Balance,
	/// First slot in range to bid on; it's actually a LeasePeriod, but that's the same type as
	/// BlockNumber.
	pub first_slot: LeasePeriod,
	/// Last slot in range to bid on; it's actually a LeasePeriod, but that's the same type as
	/// BlockNumber.
	pub last_slot: LeasePeriod,
	/// Index used for the child trie of this fund
	pub trie_index: TrieIndex,
	/// Fund status
	pub status: FundStatus,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct ReserveInfo<Balance> {
	value: Balance,
	if_mint: bool,
}

#[frame_support::pallet]
pub mod pallet {
	// Import various types used to declare pallet in scope.
	use bifrost_primitives::{CurrencyId, LeasePeriod, MessageId, Nonce, ParaId};
	use bifrost_xcm_interface::traits::XcmHelper;
	use frame_support::{
		pallet_prelude::{storage::child, *},
		sp_runtime::traits::{AccountIdConversion, CheckedAdd, Hash, Saturating, Zero},
		storage::ChildTriePrefixIterator,
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use orml_traits::{
		currency::TransferAll, MultiCurrency, MultiLockableCurrency, MultiReservableCurrency,
	};
	use sp_arithmetic::Percent;
	use sp_std::{convert::TryInto, prelude::*};
	use xcm::v3::MaybeErrorCode;

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type RuntimeOrigin: IsType<<Self as frame_system::Config>::RuntimeOrigin>
			+ Into<Result<pallet_xcm::Origin, <Self as Config>::RuntimeOrigin>>;

		type RuntimeCall: Parameter + From<Call<Self>>;

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

		#[pallet::constant]
		type VSBondValidPeriod: Get<BlockNumberFor<Self>>;

		/// The time interval from 1:1 redeem-pool to bancor-pool to release.
		#[pallet::constant]
		type ReleaseCycle: Get<BlockNumberFor<Self>>;

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
			+ MultiReservableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiLockableCurrency<AccountIdOf<Self>>;

		type EnsureConfirmAsGovernance: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		type WeightInfo: WeightInfo;

		/// The XcmInterface to manage the staking of sub-account on relaychain.
		type XcmInterface: XcmHelper<AccountIdOf<Self>, BalanceOf<Self>>;

		#[pallet::constant]
		type TreasuryAccount: Get<Self::AccountId>;

		#[pallet::constant]
		type BuybackPalletId: Get<PalletId>;

		type DexOperator: ExportZenlink<Self::AccountId, AssetId>;

		type CurrencyIdConversion: CurrencyIdConversion<CurrencyId>;

		type CurrencyIdRegister: CurrencyIdRegister<CurrencyId>;

		type ParachainId: Get<cumulus_primitives_core::ParaId>;

		type StablePool: StablePoolHandler<Balance = BalanceOf<Self>, AccountId = Self::AccountId>;

		type VtokenMinting: VtokenMintingInterface<Self::AccountId, CurrencyId, BalanceOf<Self>>;

		#[pallet::constant]
		type LockId: Get<LockIdentifier>;

		#[pallet::constant]
		type BatchLimit: Get<u32>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
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
		/// Withdrew full balance of a contributor. [who, fund_index, amount]
		Withdrew(ParaId, BalanceOf<T>),
		/// refund to account. [who, fund_index,value]
		Refunded(AccountIdOf<T>, ParaId, LeasePeriod, LeasePeriod, BalanceOf<T>),
		/// all refund
		AllRefunded(ParaId),
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
		End(ParaId),
		Continued(ParaId, LeasePeriod, LeasePeriod),
		RefundedDissolved(ParaId, LeasePeriod, LeasePeriod),
		Buyback(BalanceOf<T>),
		VstokenUnlocked(AccountIdOf<T>),
		BuybackByStablePool {
			pool_id: StableAssetPoolId,
			currency_id_in: CurrencyId,
			value: BalanceOf<T>,
		},
		Reserved {
			who: AccountIdOf<T>,
			para_id: ParaId,
			value: BalanceOf<T>,
			if_mint: bool,
		},
		ReservationCancelled {
			who: AccountIdOf<T>,
			para_id: ParaId,
		},
		ReservationFullyHandled {
			para_id: ParaId,
		},
		ReservationHandled {
			para_id: ParaId,
		},
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
		/// Don't have enough token to redeem by users
		NotEnoughBalanceInRedeemPool,
		NotEnoughBalanceInFund,
		InvalidFundSameSlot,
		InvalidFundNotExist,
		InvalidRefund,
		NotEnoughBalanceToContribute,
		NotSupportTokenType,
		/// Responder is not a relay chain
		ResponderNotRelayChain,
		/// No contribution record found
		NotFindContributionValue,
		ArgumentsError,
	}

	/// Multisig confirm account
	#[pallet::storage]
	pub type MultisigConfirmAccount<T: Config> = StorageValue<_, AccountIdOf<T>, OptionQuery>;

	/// Tracker for the next available fund index
	#[pallet::storage]
	pub(super) type CurrentTrieIndex<T: Config> = StorageValue<_, TrieIndex, ValueQuery>;

	/// Tracker for the next nonce index
	#[pallet::storage]
	pub(super) type CurrentNonce<T: Config> =
		StorageMap<_, Blake2_128Concat, ParaId, Nonce, ValueQuery>;

	/// Record contribution
	#[pallet::storage]
	pub type QueryIdContributionInfo<T: Config> =
		StorageMap<_, Blake2_128Concat, QueryId, (ParaId, AccountIdOf<T>, BalanceOf<T>)>;

	/// Info on all of the funds.
	#[pallet::storage]
	pub(super) type Funds<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		ParaId,
		Option<FundInfo<BalanceOf<T>, LeasePeriod>>,
		ValueQuery,
	>;

	/// The balance can be redeemed to users.
	#[pallet::storage]
	pub type RedeemPool<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
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

	#[pallet::storage]
	pub type ReserveInfos<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		ParaId,
		Twox64Concat,
		T::AccountId,
		ReserveInfo<BalanceOf<T>>,
		ValueQuery,
	>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub initial_multisig_account: Option<AccountIdOf<T>>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			if let Some(ref key) = self.initial_multisig_account {
				MultisigConfirmAccount::<T>::put(key)
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::fund_retire())]
		pub fn fund_retire(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
		) -> DispatchResult {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?;

			let fund = Funds::<T>::get(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Success, Error::<T>::InvalidFundStatus);

			let fund_new = FundInfo { status: FundStatus::Retired, ..fund };
			Funds::<T>::insert(index, Some(fund_new));
			Self::deposit_event(Event::<T>::Retired(index));

			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::fund_end())]
		pub fn fund_end(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?;

			let fund = Funds::<T>::get(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(
				fund.status == FundStatus::RefundWithdrew ||
					fund.status == FundStatus::RedeemWithdrew,
				Error::<T>::InvalidFundStatus
			);

			let fund_new = FundInfo { status: FundStatus::End, ..fund };
			Funds::<T>::insert(index, Some(fund_new));
			Self::deposit_event(Event::<T>::End(index));

			Ok(())
		}

		/// Edit the configuration for an in-progress crowdloan.
		///
		/// Can only be called by Root origin.
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::edit())]
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

			let fund = Funds::<T>::get(index).ok_or(Error::<T>::InvalidParaId)?;

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

		/// Withdraw full balance of the parachain.
		/// - `index`: The parachain to whose crowdloan the contribution was made.
		#[pallet::call_index(14)]
		#[pallet::weight(T::WeightInfo::withdraw())]
		pub fn withdraw(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			T::EnsureConfirmAsGovernance::ensure_origin(origin.clone())?;

			let fund = Funds::<T>::get(index).ok_or(Error::<T>::InvalidParaId)?;
			let can = fund.status == FundStatus::Failed || fund.status == FundStatus::Retired;
			ensure!(can, Error::<T>::InvalidFundStatus);

			let amount_withdrew = fund.raised;
			let total = RedeemPool::<T>::get()
				.checked_add(&amount_withdrew)
				.ok_or(Error::<T>::Overflow)?;
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

		#[pallet::call_index(15)]
		#[pallet::weight(T::WeightInfo::refund())]
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
			ensure!(RedeemPool::<T>::get() >= value, Error::<T>::NotEnoughBalanceInRefundPool);

			let vs_token = T::CurrencyIdConversion::convert_to_vstoken(T::RelayChainToken::get())
				.map_err(|_| Error::<T>::NotSupportTokenType)?;
			let vs_bond = T::CurrencyIdConversion::convert_to_vsbond(
				T::RelayChainToken::get(),
				index,
				fund.first_slot,
				fund.last_slot,
			)
			.map_err(|_| Error::<T>::NotSupportTokenType)?;
			T::MultiCurrency::ensure_can_withdraw(vs_token, &who, value)
				.map_err(|_e| Error::<T>::NotEnoughFreeAssetsToRedeem)?;
			T::MultiCurrency::ensure_can_withdraw(vs_bond, &who, value)
				.map_err(|_e| Error::<T>::NotEnoughFreeAssetsToRedeem)?;

			T::MultiCurrency::withdraw(vs_token, &who, value)?;
			T::MultiCurrency::withdraw(vs_bond, &who, value)?;

			RedeemPool::<T>::set(RedeemPool::<T>::get().saturating_sub(value));
			let mut fund_new = Funds::<T>::get(index).ok_or(Error::<T>::InvalidParaId)?;
			fund_new.raised = fund_new.raised.saturating_sub(value);
			Funds::<T>::insert(index, Some(fund_new));
			if fund.status == FundStatus::FailedToContinue {
				fund.raised = fund.raised.saturating_sub(value);
				FailedFundsToRefund::<T>::insert(
					(index, first_slot, last_slot),
					Some(fund.clone()),
				);
			}

			T::MultiCurrency::transfer(
				T::RelayChainToken::get(),
				&Self::fund_account_id(index),
				&who,
				value,
			)?;

			Self::deposit_event(Event::Refunded(
				who,
				index,
				fund.first_slot,
				fund.last_slot,
				value,
			));

			Ok(())
		}

		#[pallet::call_index(16)]
		#[pallet::weight(T::WeightInfo::redeem())]
		pub fn redeem(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] value: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			let mut fund = Funds::<T>::get(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::RedeemWithdrew, Error::<T>::InvalidFundStatus);
			ensure!(fund.raised >= value, Error::<T>::NotEnoughBalanceInRedeemPool);

			let vs_token = T::CurrencyIdConversion::convert_to_vstoken(T::RelayChainToken::get())
				.map_err(|_| Error::<T>::NotSupportTokenType)?;
			let vs_bond = T::CurrencyIdConversion::convert_to_vsbond(
				T::RelayChainToken::get(),
				index,
				fund.first_slot,
				fund.last_slot,
			)
			.map_err(|_| Error::<T>::NotSupportTokenType)?;

			ensure!(RedeemPool::<T>::get() >= value, Error::<T>::NotEnoughBalanceInRedeemPool);
			let cur_block = <frame_system::Pallet<T>>::block_number();
			let expired = Self::is_expired(cur_block, fund.last_slot)?;
			ensure!(!expired, Error::<T>::VSBondExpired);
			T::MultiCurrency::ensure_can_withdraw(vs_token, &who, value)
				.map_err(|_e| Error::<T>::NotEnoughFreeAssetsToRedeem)?;
			T::MultiCurrency::ensure_can_withdraw(vs_bond, &who, value)
				.map_err(|_e| Error::<T>::NotEnoughFreeAssetsToRedeem)?;

			T::MultiCurrency::withdraw(vs_token, &who, value)?;
			T::MultiCurrency::withdraw(vs_bond, &who, value)?;
			RedeemPool::<T>::set(RedeemPool::<T>::get().saturating_sub(value));

			fund.raised = fund.raised.saturating_sub(value);
			Funds::<T>::insert(index, Some(fund.clone()));

			T::MultiCurrency::transfer(
				T::RelayChainToken::get(),
				&Self::fund_account_id(index),
				&who,
				value,
			)?;
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
		#[pallet::call_index(17)]
		#[pallet::weight(T::WeightInfo::dissolve_refunded())]
		pub fn dissolve_refunded(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] first_slot: LeasePeriod,
			#[pallet::compact] last_slot: LeasePeriod,
		) -> DispatchResult {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?;

			let fund = FailedFundsToRefund::<T>::get((index, first_slot, last_slot))
				.ok_or(Error::<T>::InvalidRefund)?;

			ensure!(fund.status == FundStatus::FailedToContinue, Error::<T>::InvalidFundStatus);

			FailedFundsToRefund::<T>::remove((index, first_slot, last_slot));

			Self::deposit_event(Event::<T>::RefundedDissolved(index, first_slot, last_slot));

			Ok(())
		}

		/// Remove a fund after the retirement period has ended and all funds have been returned.
		#[pallet::call_index(18)]
		#[pallet::weight(T::WeightInfo::dissolve())]
		pub fn dissolve(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?;

			let mut fund = Funds::<T>::get(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::End, Error::<T>::InvalidFundStatus);

			let mut refund_count = 0u32;
			// Try killing the crowdloan child trie and Assume everyone will be refunded.
			let contributions = Self::contribution_iterator(fund.trie_index);
			let mut all_refunded = true;
			#[allow(clippy::explicit_counter_loop)]
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

			if all_refunded {
				let from = &Self::fund_account_id(index);
				let relay_currency_id = T::RelayChainToken::get();
				let fund_account_balance = T::MultiCurrency::free_balance(relay_currency_id, from);
				T::MultiCurrency::transfer(
					relay_currency_id,
					from,
					&T::TreasuryAccount::get(),
					Percent::from_percent(25) * fund_account_balance,
				)?;
				T::MultiCurrency::transfer(
					relay_currency_id,
					from,
					&T::BuybackPalletId::get().into_account_truncating(),
					Percent::from_percent(75) * fund_account_balance,
				)?;
				Funds::<T>::remove(index);
				Self::deposit_event(Event::<T>::Dissolved(index));
			}

			Ok(())
		}

		// unused but xcm-interface
		#[pallet::call_index(20)]
		#[pallet::weight(T::WeightInfo::confirm_contribute())]
		pub fn confirm_contribution(
			origin: OriginFor<T>,
			query_id: QueryId,
			response: Response,
		) -> DispatchResult {
			let responder = ensure_response(<T as Config>::RuntimeOrigin::from(origin))?;
			ensure!(responder == xcm::v4::Location::parent(), Error::<T>::ResponderNotRelayChain);

			let (index, contributer, _amount) = QueryIdContributionInfo::<T>::get(query_id)
				.ok_or(Error::<T>::NotFindContributionValue)?;

			let fund = Funds::<T>::get(index).ok_or(Error::<T>::InvalidParaId)?;
			let can_confirm = fund.status == FundStatus::Ongoing ||
				fund.status == FundStatus::Failed ||
				fund.status == FundStatus::Success;
			ensure!(can_confirm, Error::<T>::InvalidFundStatus);

			let (contributed, status) = Self::contribution(fund.trie_index, &contributer);
			ensure!(status.is_contributing(), Error::<T>::InvalidContributionStatus);
			let contributing = status.contributing();

			let vs_token = T::CurrencyIdConversion::convert_to_vstoken(T::RelayChainToken::get())
				.map_err(|_| Error::<T>::NotSupportTokenType)?;
			let vs_bond = T::CurrencyIdConversion::convert_to_vsbond(
				T::RelayChainToken::get(),
				index,
				fund.first_slot,
				fund.last_slot,
			)
			.map_err(|_| Error::<T>::NotSupportTokenType)?;

			if let Response::DispatchResult(MaybeErrorCode::Success) = response {
				// Issue reserved vsToken/vsBond to contributor
				T::MultiCurrency::deposit(vs_token, &contributer, contributing)?;
				T::MultiCurrency::deposit(vs_bond, &contributer, contributing)?;

				// Update the raised of fund
				let fund_new =
					FundInfo { raised: fund.raised.saturating_add(contributing), ..fund };
				Funds::<T>::insert(index, Some(fund_new));

				T::MultiCurrency::unreserve(T::RelayChainToken::get(), &contributer, contributing);
				T::MultiCurrency::transfer(
					T::RelayChainToken::get(),
					&contributer,
					&Self::fund_account_id(index),
					contributing,
				)?;

				// Update the contribution of contributer
				let contributed_new = contributed.saturating_add(contributing);
				Self::put_contribution(
					fund.trie_index,
					&contributer,
					contributed_new,
					ContributionStatus::Idle,
				);
				Self::deposit_event(Event::Contributed(contributer, index, contributing));
			} else {
				// Update the contribution of contributer
				Self::put_contribution(
					fund.trie_index,
					&contributer,
					contributed,
					ContributionStatus::Idle,
				);
				T::MultiCurrency::unreserve(T::RelayChainToken::get(), &contributer, contributing);
				Self::deposit_event(Event::ContributeFailed(contributer, index, contributing));
			}
			QueryIdContributionInfo::<T>::remove(query_id);
			Ok(())
		}

		#[pallet::call_index(21)]
		#[pallet::weight(T::WeightInfo::buyback_vstoken_by_stable_pool())]
		pub fn buyback_vstoken_by_stable_pool(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			currency_id_in: CurrencyId,
			value: BalanceOf<T>,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			let relay_currency_id = T::RelayChainToken::get();
			let relay_vtoken_id = T::CurrencyIdConversion::convert_to_vtoken(relay_currency_id)
				.map_err(|_| Error::<T>::NotSupportTokenType)?;
			let relay_vstoken_id = T::CurrencyIdConversion::convert_to_vstoken(relay_currency_id)
				.map_err(|_| Error::<T>::NotSupportTokenType)?;

			match currency_id_in {
				cid if cid == relay_currency_id => {
					T::StablePool::swap(
						&T::BuybackPalletId::get().into_account_truncating(),
						pool_id,
						T::StablePool::get_pool_token_index(pool_id, relay_currency_id)
							.ok_or(Error::<T>::ArgumentsError)?,
						T::StablePool::get_pool_token_index(pool_id, relay_vstoken_id)
							.ok_or(Error::<T>::ArgumentsError)?,
						value.saturated_into(),
						Percent::from_percent(50).saturating_reciprocal_mul(value).saturated_into(),
					)?;
				},
				cid if cid == relay_vtoken_id => {
					let token_value = T::VtokenMinting::vtoken_to_token(
						relay_currency_id,
						relay_vtoken_id,
						value,
					)?;
					T::StablePool::swap(
						&T::BuybackPalletId::get().into_account_truncating(),
						pool_id,
						T::StablePool::get_pool_token_index(pool_id, relay_vtoken_id)
							.ok_or(Error::<T>::ArgumentsError)?,
						T::StablePool::get_pool_token_index(pool_id, relay_vstoken_id)
							.ok_or(Error::<T>::ArgumentsError)?,
						value.saturated_into(),
						Percent::from_percent(50)
							.saturating_reciprocal_mul(token_value)
							.saturated_into(),
					)?;
				},
				_ => return Err(Error::<T>::ArgumentsError.into()),
			}

			Self::deposit_event(Event::<T>::BuybackByStablePool { pool_id, currency_id_in, value });
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			// Release x% KSM/DOT from redeem-pool to bancor-pool per cycle
			if n != Zero::zero() && (n % T::ReleaseCycle::get()) == Zero::zero() {
				if let Ok(rp_balance) = TryInto::<u128>::try_into(RedeemPool::<T>::get()) {
					// Calculate the release amount
					let release_amount = T::ReleaseRatio::get() * rp_balance;

					// Must be ok
					if let Err(_) = TryInto::<BalanceOf<T>>::try_into(release_amount) {
						log::warn!("Overflow: The balance of redeem-pool exceeds u128.");
					}
				}
			}
			T::DbWeight::get().reads(1)
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn fund_success(origin: OriginFor<T>, index: ParaId) -> DispatchResult {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?;

			let fund = Funds::<T>::get(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Ongoing, Error::<T>::InvalidFundStatus);

			let fund_new = FundInfo { status: FundStatus::Success, ..fund };
			Funds::<T>::insert(index, Some(fund_new));
			Self::deposit_event(Event::<T>::Success(index));

			Ok(())
		}

		pub fn fund_fail(origin: OriginFor<T>, index: ParaId) -> DispatchResult {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?;

			// crownload is failed, so enable the withdrawal function of vsToken/vsBond
			let fund = crate::pallet::Funds::<T>::get(index)
				.ok_or(crate::pallet::Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Ongoing, Error::<T>::InvalidFundStatus);

			let fund_new = crate::FundInfo { status: crate::FundStatus::Failed, ..fund };
			crate::pallet::Funds::<T>::insert(index, Some(fund_new));
			Self::deposit_event(crate::pallet::Event::<T>::Failed(index));

			Ok(())
		}

		pub fn continue_fund(
			origin: OriginFor<T>,
			index: ParaId,
			first_slot: LeasePeriod,
			last_slot: LeasePeriod,
		) -> DispatchResult {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?;

			// crownload is failed, so enable the withdrawal function of vsToken/vsBond
			let fund = Funds::<T>::get(index).ok_or(Error::<T>::InvalidParaId)?;
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

			match T::RelayChainToken::get() {
				CurrencyId::Token(token_symbol) =>
					if !T::CurrencyIdRegister::check_vsbond_registered(
						token_symbol,
						index,
						first_slot,
						last_slot,
					) {
						T::CurrencyIdRegister::register_vsbond_metadata(
							token_symbol,
							index,
							first_slot,
							last_slot,
						)?;
					},
				CurrencyId::Token2(token_id) => {
					if !T::CurrencyIdRegister::check_vsbond2_registered(
						token_id, index, first_slot, last_slot,
					) {
						T::CurrencyIdRegister::register_vsbond2_metadata(
							token_id, index, first_slot, last_slot,
						)?;
					}
				},
				_ => (),
			}

			Self::deposit_event(Event::<T>::Continued(
				index,
				fund_old.first_slot,
				fund_old.last_slot,
			));

			Ok(())
		}

		/// Create a new crowdloaning campaign for a parachain slot deposit for the current auction.
		pub fn create(
			origin: OriginFor<T>,
			index: ParaId,
			cap: BalanceOf<T>,
			first_slot: LeasePeriod,
			last_slot: LeasePeriod,
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

			match T::RelayChainToken::get() {
				CurrencyId::Token(token_symbol) =>
					if !T::CurrencyIdRegister::check_vsbond_registered(
						token_symbol,
						index,
						first_slot,
						last_slot,
					) {
						T::CurrencyIdRegister::register_vsbond_metadata(
							token_symbol,
							index,
							first_slot,
							last_slot,
						)?;
					},
				CurrencyId::Token2(token_id) => {
					if !T::CurrencyIdRegister::check_vsbond2_registered(
						token_id, index, first_slot, last_slot,
					) {
						T::CurrencyIdRegister::register_vsbond2_metadata(
							token_id, index, first_slot, last_slot,
						)?;
					}
				},
				_ => (),
			}

			Self::deposit_event(Event::<T>::Created(index));

			Ok(())
		}

		/// Contribute to a crowd sale. This will transfer some balance over to fund a parachain
		/// slot. It will be withdrawable in two instances: the parachain becomes retired; or the
		/// slot is unable to be purchased and the timeout expires.
		pub fn contribute(
			origin: OriginFor<T>,
			index: ParaId,
			value: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			let fund = Funds::<T>::get(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Ongoing, Error::<T>::InvalidFundStatus);

			ensure!(value >= T::MinContribution::get(), Error::<T>::ContributionTooSmall);

			let raised = fund.raised.checked_add(&value).ok_or(Error::<T>::Overflow)?;
			ensure!(raised <= fund.cap, Error::<T>::CapExceeded);

			let (contributed, status) = Self::contribution(fund.trie_index, &who);
			ensure!(
				status == ContributionStatus::Idle ||
					status == ContributionStatus::Refunded ||
					status == ContributionStatus::Redeemed ||
					status == ContributionStatus::Unlocked,
				Error::<T>::InvalidContributionStatus
			);

			ensure!(
				T::MultiCurrency::can_reserve(T::RelayChainToken::get(), &who, value),
				Error::<T>::NotEnoughBalanceToContribute
			);

			T::MultiCurrency::reserve(T::RelayChainToken::get(), &who, value)?;

			Self::put_contribution(
				fund.trie_index,
				&who,
				contributed,
				ContributionStatus::Contributing(value),
			);

			let message_id = T::XcmInterface::contribute(who.clone(), index, value)?;

			Self::deposit_event(Event::Contributing(who, index, value, message_id));
			Ok(())
		}

		/// Confirm contribute
		pub fn confirm_contribute(
			origin: OriginFor<T>,
			query_id: QueryId,
			is_success: bool,
		) -> DispatchResult {
			let confirmor = ensure_signed(origin.clone())?;
			if Some(confirmor) != MultisigConfirmAccount::<T>::get() {
				return Err(DispatchError::BadOrigin.into());
			}

			let (index, contributer, _amount) = QueryIdContributionInfo::<T>::get(query_id)
				.ok_or(Error::<T>::NotFindContributionValue)?;

			let fund = Funds::<T>::get(index).ok_or(Error::<T>::InvalidParaId)?;
			let can_confirm = fund.status == FundStatus::Ongoing ||
				fund.status == FundStatus::Failed ||
				fund.status == FundStatus::Success;
			ensure!(can_confirm, Error::<T>::InvalidFundStatus);

			let (contributed, status) = Self::contribution(fund.trie_index, &contributer);
			ensure!(status.is_contributing(), Error::<T>::InvalidContributionStatus);
			let contributing = status.contributing();

			let vs_token = T::CurrencyIdConversion::convert_to_vstoken(T::RelayChainToken::get())
				.map_err(|_| Error::<T>::NotSupportTokenType)?;
			let vs_bond = T::CurrencyIdConversion::convert_to_vsbond(
				T::RelayChainToken::get(),
				index,
				fund.first_slot,
				fund.last_slot,
			)
			.map_err(|_| Error::<T>::NotSupportTokenType)?;

			if is_success {
				// Issue reserved vsToken/vsBond to contributor
				T::MultiCurrency::deposit(vs_token, &contributer, contributing)?;
				T::MultiCurrency::deposit(vs_bond, &contributer, contributing)?;

				// Update the raised of fund
				let fund_new =
					FundInfo { raised: fund.raised.saturating_add(contributing), ..fund };
				Funds::<T>::insert(index, Some(fund_new));

				T::MultiCurrency::unreserve(T::RelayChainToken::get(), &contributer, contributing);
				T::MultiCurrency::transfer(
					T::RelayChainToken::get(),
					&contributer,
					&Self::fund_account_id(index),
					contributing,
				)?;

				// Update the contribution of contributer
				let contributed_new = contributed.saturating_add(contributing);
				Self::put_contribution(
					fund.trie_index,
					&contributer,
					contributed_new,
					ContributionStatus::Idle,
				);
				Self::deposit_event(Event::Contributed(contributer, index, contributing));
			} else {
				// Update the contribution of contributer
				Self::put_contribution(
					fund.trie_index,
					&contributer,
					contributed,
					ContributionStatus::Idle,
				);
				T::MultiCurrency::unreserve(T::RelayChainToken::get(), &contributer, contributing);
				Self::deposit_event(Event::ContributeFailed(contributer, index, contributing));
			}

			QueryIdContributionInfo::<T>::remove(query_id);

			Ok(())
		}

		/// Unlock the reserved vsToken/vsBond after fund success
		pub fn unlock(origin: OriginFor<T>, who: AccountIdOf<T>, index: ParaId) -> DispatchResult {
			ensure_signed(origin)?;
			let fund = Funds::<T>::get(index).ok_or(Error::<T>::InvalidParaId)?;

			let (contributed, _) = Self::contribution(fund.trie_index, &who);

			let vs_token = T::CurrencyIdConversion::convert_to_vstoken(T::RelayChainToken::get())
				.map_err(|_| Error::<T>::NotSupportTokenType)?;
			let vs_bond = T::CurrencyIdConversion::convert_to_vsbond(
				T::RelayChainToken::get(),
				index,
				fund.first_slot,
				fund.last_slot,
			)
			.map_err(|_| Error::<T>::NotSupportTokenType)?;

			T::MultiCurrency::unreserve(vs_token, &who, contributed);
			T::MultiCurrency::unreserve(vs_bond, &who, contributed);

			Self::deposit_event(Event::<T>::Unlocked(who, index, contributed));

			Ok(())
		}

		pub fn buyback(origin: OriginFor<T>, value: crate::BalanceOf<T>) -> DispatchResult {
			let _who = ensure_signed(origin.clone())?;

			let relay_currency_id = T::RelayChainToken::get();
			let relay_vstoken_id = T::CurrencyIdConversion::convert_to_vstoken(relay_currency_id)
				.map_err(|_| crate::pallet::Error::<T>::NotSupportTokenType)?;
			let relay_asset_id: AssetId =
				AssetId::try_convert_from(relay_currency_id, T::ParachainId::get().into())
					.map_err(|_| DispatchError::Other("Conversion Error."))?;
			let relay_vstoken_asset_id: AssetId =
				AssetId::try_convert_from(relay_vstoken_id, T::ParachainId::get().into())
					.map_err(|_| DispatchError::Other("Conversion Error."))?;
			let path = vec![relay_asset_id, relay_vstoken_asset_id];

			T::DexOperator::inner_swap_exact_assets_for_assets(
				&T::BuybackPalletId::get().into_account_truncating(),
				value.saturated_into(),
				Percent::from_percent(50).saturating_reciprocal_mul(value).saturated_into(),
				&path,
				&T::TreasuryAccount::get(),
			)?;

			Self::deposit_event(crate::pallet::Event::<T>::Buyback(value));

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// set multisig account
		pub fn set_multisig_account(account: AccountIdOf<T>) {
			MultisigConfirmAccount::<T>::put(account);
		}
		/// Check if the vsBond is `past` the redeemable date
		pub(crate) fn is_expired(
			block: BlockNumberFor<T>,
			last_slot: LeasePeriod,
		) -> Result<bool, Error<T>> {
			let block_begin_redeem = Self::block_end_of_lease_period_index(last_slot);
			let block_end_redeem = block_begin_redeem.saturating_add(T::VSBondValidPeriod::get());

			Ok(block >= block_end_redeem)
		}

		/// Check if the vsBond is `in` the redeemable date
		#[allow(dead_code)]
		pub(crate) fn can_redeem(
			block: BlockNumberFor<T>,
			last_slot: LeasePeriod,
		) -> Result<bool, Error<T>> {
			let block_begin_redeem = Self::block_end_of_lease_period_index(last_slot);
			let block_end_redeem = block_begin_redeem.saturating_add(T::VSBondValidPeriod::get());

			Ok(block >= block_begin_redeem && block < block_end_redeem)
		}

		pub(crate) fn block_end_of_lease_period_index(slot: LeasePeriod) -> BlockNumberFor<T> {
			(BlockNumberFor::<T>::from(slot) + One::one()).saturating_mul(T::LeasePeriod::get())
		}

		pub fn find_fund(
			index: ParaId,
			first_slot: LeasePeriod,
			last_slot: LeasePeriod,
		) -> Result<FundInfo<BalanceOf<T>, LeasePeriod>, Error<T>> {
			return match FailedFundsToRefund::<T>::get((index, first_slot, last_slot)) {
				Some(fund) => Ok(fund),
				_ => match Funds::<T>::get(index) {
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

		pub fn contribution(
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
			let fund = Funds::<T>::get(index).ok_or(Error::<T>::InvalidParaId)?;
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

		pub fn redeem_for_reserve(
			who: AccountIdOf<T>,
			index: ParaId,
			value: BalanceOf<T>,
			fund: &mut FundInfo<BalanceOf<T>, LeasePeriod>,
			vs_token: CurrencyId,
			vs_bond: CurrencyId,
		) -> DispatchResult {
			ensure!(fund.raised >= value, Error::<T>::NotEnoughBalanceInRedeemPool);

			ensure!(RedeemPool::<T>::get() >= value, Error::<T>::NotEnoughBalanceInRedeemPool);
			let cur_block = <frame_system::Pallet<T>>::block_number();
			let expired = Self::is_expired(cur_block, fund.last_slot)?;
			ensure!(!expired, Error::<T>::VSBondExpired);
			T::MultiCurrency::ensure_can_withdraw(vs_token, &who, value)
				.map_err(|_e| Error::<T>::NotEnoughFreeAssetsToRedeem)?;
			T::MultiCurrency::ensure_can_withdraw(vs_bond, &who, value)
				.map_err(|_e| Error::<T>::NotEnoughFreeAssetsToRedeem)?;

			T::MultiCurrency::withdraw(vs_token, &who, value)?;
			T::MultiCurrency::withdraw(vs_bond, &who, value)?;
			RedeemPool::<T>::set(RedeemPool::<T>::get().saturating_sub(value));

			fund.raised = fund.raised.saturating_sub(value);
			Funds::<T>::insert(index, Some(fund.clone()));

			T::MultiCurrency::transfer(
				T::RelayChainToken::get(),
				&Self::fund_account_id(index),
				&who,
				value,
			)?;
			Self::deposit_event(Event::Redeemed(
				who,
				index,
				fund.first_slot,
				fund.last_slot,
				value,
			));

			Ok(())
		}

		pub fn refund_for_reserve(
			who: AccountIdOf<T>,
			index: ParaId,
			first_slot: LeasePeriod,
			last_slot: LeasePeriod,
			value: BalanceOf<T>,
			vs_token: CurrencyId,
			vs_bond: CurrencyId,
		) -> DispatchResult {
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
			ensure!(RedeemPool::<T>::get() >= value, Error::<T>::NotEnoughBalanceInRefundPool);

			T::MultiCurrency::ensure_can_withdraw(vs_token, &who, value)
				.map_err(|_e| Error::<T>::NotEnoughFreeAssetsToRedeem)?;
			T::MultiCurrency::ensure_can_withdraw(vs_bond, &who, value)
				.map_err(|_e| Error::<T>::NotEnoughFreeAssetsToRedeem)?;

			T::MultiCurrency::withdraw(vs_token, &who, value)?;
			T::MultiCurrency::withdraw(vs_bond, &who, value)?;

			RedeemPool::<T>::set(RedeemPool::<T>::get().saturating_sub(value));
			let mut fund_new = Funds::<T>::get(index).ok_or(Error::<T>::InvalidParaId)?;
			fund_new.raised = fund_new.raised.saturating_sub(value);
			Funds::<T>::insert(index, Some(fund_new));
			if fund.status == FundStatus::FailedToContinue {
				fund.raised = fund.raised.saturating_sub(value);
				FailedFundsToRefund::<T>::insert(
					(index, first_slot, last_slot),
					Some(fund.clone()),
				);
			}

			T::MultiCurrency::transfer(
				T::RelayChainToken::get(),
				&Self::fund_account_id(index),
				&who,
				value,
			)?;

			Self::deposit_event(Event::Refunded(
				who,
				index,
				fund.first_slot,
				fund.last_slot,
				value,
			));

			Ok(())
		}
	}
}

impl<T: Config>
	bifrost_xcm_interface::SalpHelper<AccountIdOf<T>, <T as Config>::RuntimeCall, BalanceOf<T>>
	for Pallet<T>
{
	fn confirm_contribute_call() -> <T as Config>::RuntimeCall {
		let call = Call::<T>::confirm_contribution { query_id: 0, response: Default::default() };
		<T as Config>::RuntimeCall::from(call)
	}

	fn bind_query_id_and_contribution(
		query_id: QueryId,
		index: ChainId,
		contributer: AccountIdOf<T>,
		amount: BalanceOf<T>,
	) {
		QueryIdContributionInfo::<T>::insert(query_id, (index, contributer, amount));
	}
}
