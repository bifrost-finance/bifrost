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

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	// Import various types used to declare pallet in scope.
	use frame_support::{
		log,
		pallet_prelude::{storage::child, *},
		sp_runtime::{
			traits::{AccountIdConversion, CheckedAdd, Hash, Saturating, Zero},
			MultiSignature,
		},
		storage::ChildTriePrefixIterator,
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use node_primitives::{CurrencyId, TokenSymbol, LeasePeriod};
	use orml_traits::{
		currency::TransferAll, MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency,
		MultiReservableCurrency,
	};
	use sp_std::prelude::*;
	use xcm::v0::{Junction, MultiLocation, OriginKind, SendXcm, Xcm};

	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
	pub enum FundStatus {
		Ongoing,
		Retired,
		Success,
		Failed,
	}

	impl Default for FundStatus {
		fn default() -> Self {
			FundStatus::Ongoing
		}
	}

	/// Information on a funding effort for a pre-existing parachain. We assume that the parachain ID
	/// is known as it's used for the key of the storage item for which this is the value (`Funds`).
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
	#[codec(dumb_trait_bound)]
	pub struct FundInfo<AccountId, Balance, BlockNumber, LeasePeriod> {
		/// The owning account who placed the deposit.
		depositor: AccountId,
		/// The amount of deposit placed.
		deposit: Balance,
		/// The total amount raised.
		raised: Balance,
		/// Block number after which the funding must have succeeded. If not successful at this number
		/// then everyone may withdraw their funds.
		end: BlockNumber,
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

	type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

	type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

	type ParaId = u32;

	type TrieIndex = u32;

	#[derive(Encode, Decode)]
	pub enum CrowdloanPalletCall<BalanceOf> {
		#[codec(index = 27)]
		// the index should match the position of the module in `construct_runtime!`
		CrowdloanContribute(ContributeCall<BalanceOf>),
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
		// the index should match the position of the dispatchable in the target pallet
		Contribute(Contribution<BalanceOf>),
	}

	/// Error type for something that went wrong with xcm communication.
	#[derive(Debug)]
	pub enum XcmError {
		/// Convert origin error
		ConvertOriginFailed,
		/// Unable to send contribute
		ContributeSentFailed,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// ModuleID for the crowdloan module. An appropriate value could be ```ModuleId(*b"py/cfund")```
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The amount to be held on deposit by the depositor of a crowdloan.
		type SubmissionDeposit: Get<BalanceOf<Self>>;

		/// The minimum amount that may be contributed into a crowdloan. Should almost certainly be at
		/// least ExistentialDeposit.
		#[pallet::constant]
		type MinContribution: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type TokenType: Get<TokenSymbol>;

		type MultiCurrency: TransferAll<Self::AccountId>
			+ MultiCurrency<Self::AccountId, CurrencyId = CurrencyId>
			+ MultiCurrencyExtended<Self::AccountId, CurrencyId = CurrencyId>
			+ MultiLockableCurrency<Self::AccountId, CurrencyId = CurrencyId>
			+ MultiReservableCurrency<Self::AccountId, CurrencyId = CurrencyId>;

		#[pallet::constant]
		type RemoveKeysLimit: Get<u32>;

		type XcmSender: SendXcm;

		type SendXcmOrigin: EnsureOrigin<
			<Self as frame_system::Config>::Origin,
			Success = MultiLocation,
		>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Create a new crowdloaning campaign. [fund_index]
		Created(ParaId),
		/// Contributed to a crowd sale. [who, fund_index, amount]
		Contributed(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Withdrew full balance of a contributor. [who, fund_index, amount]
		Withdrew(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Redeemed full balance of a contributor. [who, fund_index, amount]
		Redeemed(AccountIdOf<T>, ParaId, BalanceOf<T>),
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
		InvalidOrigin,
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
		/// The crowdloan has not yet ended.
		FundNotEnded,
		/// There are no contributions stored in this crowdloan.
		NoContributions,
		/// This crowdloan has an active parachain and cannot be dissolved.
		HasActiveParachain,
		/// The crowdloan is not ready to dissolve. Potentially still has a slot or in retirement period.
		NotReadyToDissolve,
		/// Invalid signature.
		InvalidSignature,
		/// Invalid fund status.
		InvalidFundStatus,
		/// Insufficient Balance.
		InsufficientBalance,
		/// Crosschain xcm failed
		XcmFailed,
	}

	#[pallet::storage]
	#[pallet::getter(fn validators)]
	pub(super) type Validators<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountIdOf<T>, bool, ValueQuery>;

	/// Tracker for the next available trie index
	#[pallet::storage]
	#[pallet::getter(fn next_trie_index)]
	pub(super) type NextTrieIndex<T: Config> = StorageValue<_, TrieIndex, ValueQuery>;

	/// Info on all of the funds.
	#[pallet::storage]
	#[pallet::getter(fn funds)]
	pub(super) type Funds<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		ParaId,
		Option<FundInfo<AccountIdOf<T>, BalanceOf<T>, BlockNumberFor<T>, LeasePeriod>>,
		ValueQuery,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub(super) fn fund_success(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let mut fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(
				fund.status == FundStatus::Ongoing,
				Error::<T>::InvalidFundStatus
			);
			fund.status = FundStatus::Success;
			Funds::<T>::insert(index, Some(fund));

			// TODO enable vsToken/vsBond transfer
			// T::AssetHandler::unlockToken(paraId)
			// T::AssetHandler::unlockVsBond(paraId)

			Ok(().into())
		}

		#[pallet::weight(0)]
		pub(super) fn fund_fail(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			// crownload is failed, so enable the withdrawal function of vsToken/vsBond
			let mut fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(
				fund.status == FundStatus::Ongoing,
				Error::<T>::InvalidFundStatus
			);
			fund.status = FundStatus::Failed;
			Funds::<T>::insert(index, Some(fund));
			// TODO enable vsToken/vsBond transfer
			// T::AssetHandler::unlockToken(paraId)
			// T::AssetHandler::unlockVsBond(paraId)

			Ok(().into())
		}

		#[pallet::weight(0)]
		pub(super) fn fund_retire(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let mut fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(
				fund.status == FundStatus::Success,
				Error::<T>::InvalidFundStatus
			);
			fund.status = FundStatus::Retired;
			Funds::<T>::insert(index, Some(fund));

			Ok(().into())
		}

		/// Create a new crowdloaning campaign for a parachain slot deposit for the current auction.
		#[pallet::weight(0)]
		pub(super) fn create(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] cap: BalanceOf<T>,
			#[pallet::compact] first_slot: LeasePeriod,
			#[pallet::compact] last_slot: LeasePeriod,
			#[pallet::compact] end: BlockNumberFor<T>,
		) -> DispatchResultWithPostInfo {
			let depositor = ensure_signed(origin)?;

			ensure!(first_slot <= last_slot, Error::<T>::LastSlotBeforeFirstSlot);
			let last_slot_limit = first_slot
				.checked_add(3u32.into())
				.ok_or(Error::<T>::FirstSlotTooFarInFuture)?;
			ensure!(
				last_slot <= last_slot_limit,
				Error::<T>::LastSlotTooFarInFuture
			);
			ensure!(
				end > frame_system::Pallet::<T>::block_number(),
				Error::<T>::CannotEndInPast
			);

			// There should not be an existing fund.
			ensure!(!Funds::<T>::contains_key(index), Error::<T>::FundNotEnded);

			let trie_index = Self::next_trie_index();
			let new_trie_index = trie_index.checked_add(1).ok_or(Error::<T>::Overflow)?;

			let deposit = T::SubmissionDeposit::get();
			// T::Currency::reserve(&depositor, deposit)?;

			Funds::<T>::insert(
				index,
				Some(FundInfo {
					depositor,
					deposit,
					raised: Zero::zero(),
					end,
					cap,
					first_slot,
					last_slot,
					trie_index,
					status: FundStatus::Ongoing,
				}),
			);

			NextTrieIndex::<T>::put(new_trie_index);

			Self::deposit_event(Event::<T>::Created(index));

			Ok(().into())
		}

		/// Contribute to a crowd sale. This will transfer some balance over to fund a parachain
		/// slot. It will be withdrawable in two instances: the parachain becomes retired; or the
		/// slot is unable to be purchased and the timeout expires.
		#[pallet::weight(0)]
		pub(super) fn contribute(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] value: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin.clone())?;

			ensure!(
				value >= T::MinContribution::get(),
				Error::<T>::ContributionTooSmall
			);
			let mut fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(
				fund.status == FundStatus::Ongoing,
				Error::<T>::InvalidFundStatus
			);
			fund.raised = fund
				.raised
				.checked_add(&value)
				.ok_or(Error::<T>::Overflow)?;
			ensure!(fund.raised <= fund.cap, Error::<T>::CapExceeded);

			Self::xcm_ump_contribute_to_parachain(origin, index, value)
				.map_err(|_e| Error::<T>::XcmFailed)?;

			let old_balance = Self::contribution_get(fund.trie_index, &who);

			let balance = old_balance.saturating_add(value);
			Self::contribution_put(fund.trie_index, &who, &balance);

			// TODO
			// deposit KSM/DOT to fund_account
			// issue vsToken/vsBond to sender

			Funds::<T>::insert(index, Some(fund));

			Self::deposit_event(Event::Contributed(who, index, value));

			Ok(().into())
		}

		/// Contribute to a crowd sale. This will transfer some balance over to fund a parachain
		/// slot. It will be withdrawable in two instances: the parachain becomes retired; or the
		/// slot is unable to be purchased and the timeout expires.
		#[pallet::weight(0)]
		pub(super) fn partially_withdraw(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			contributor: AccountIdOf<T>,
			#[pallet::compact] value: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let _origin_location: MultiLocation = T::SendXcmOrigin::ensure_origin(origin)?;

			let who = contributor.clone();

			ensure!(
				value >= T::MinContribution::get(),
				Error::<T>::ContributionTooSmall
			);
			let mut fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(
				fund.status == FundStatus::Ongoing,
				Error::<T>::InvalidFundStatus
			);

			let old_balance = Self::contribution_get(fund.trie_index, &who);
			ensure!(
				fund.status == FundStatus::Ongoing,
				Error::<T>::InvalidFundStatus
			);
			ensure!(old_balance >= value, Error::<T>::InsufficientBalance);
			let balance = old_balance.saturating_sub(value);
			Self::contribution_put(fund.trie_index, &who, &balance);

			ensure!(fund.raised >= value, Error::<T>::InsufficientBalance);
			fund.raised = fund.raised.saturating_sub(value);
			// fund.raised = fund.raised.checked_sub(&value).ok_or(Error::<T>::InsufficientBalance)?;
			// TODO withdraw vsToken/vsBond from sender and withdraw KSM/DOT to fund_account
			let _fund_account = Self::fund_account_id(index);
			// T::Currency::withdraw(vsToken,contributor,value);
			// T::Currency::withdraw(vsBond,contributor,value);
			// T::Currency::withdraw(vsKsm,fund_account,value);

			Funds::<T>::insert(index, Some(fund));

			Self::deposit_event(Event::Contributed(who, index, value));

			Ok(().into())
		}

		/// Withdraw full balance of a contributor.
		///
		/// Origin must be signed.
		///
		/// The fund must be either in, or ready for, retirement. For a fund to be *in* retirement, then the retirement
		/// flag must be set. For a fund to be ready for retirement, then:
		/// - it must not already be in retirement;
		/// - the amount of raised funds must be bigger than the _free_ balance of the account;
		/// - and either:
		///   - the block number must be at least `end`; or
		///   - the current lease period must be greater than the fund's `last_slot`.
		///
		/// In this case, the fund's retirement flag is set and its `end` is reset to the current block
		/// number.
		///
		/// - `who`: The account whose contribution should be withdrawn.
		/// - `index`: The parachain to whose crowdloan the contribution was made.
		// #[weight = T::WeightInfo::withdraw()]
		#[pallet::weight(0)]
		pub(super) fn withdraw(
			origin: OriginFor<T>,
			who: AccountIdOf<T>,
			#[pallet::compact] index: ParaId,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;

			let mut fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(
				fund.status == FundStatus::Failed,
				Error::<T>::InvalidFundStatus
			);

			let balance = Self::contribution_get(fund.trie_index, &who);
			ensure!(balance > Zero::zero(), Error::<T>::NoContributions);

			Self::contribution_kill(fund.trie_index, &who);
			fund.raised = fund.raised.saturating_sub(balance);

			Funds::<T>::insert(index, Some(fund));

			// TODO destroy vsToken/vsBond from sender& withdraw KSM/DOT to fund_account
			let _fund_account = Self::fund_account_id(index);
			// T::Currency::destroy(vsToken,who);
			// T::Currency::destroy(vsBond,who);
			// T::Currency::withdraw(token,balance);

			Self::deposit_event(Event::Withdrew(who, index, balance));

			Ok(().into())
		}

		#[pallet::weight(0)]
		pub(super) fn redeem_from_bancor_pool(
			_origin: OriginFor<T>,
			who: AccountIdOf<T>,
			#[pallet::compact] index: ParaId,
			amount: Option<BalanceOf<T>>,
		) -> DispatchResultWithPostInfo {
			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(
				fund.status == FundStatus::Retired,
				Error::<T>::InvalidFundStatus
			);

			// TODO call Bancor function
			// T::Bancor::swap();

			Self::deposit_event(Event::Redeemed(who, index, amount.unwrap_or(Zero::zero())));

			Ok(().into())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_n: BlockNumberFor<T>) {
			// TODO check & release x% KSM/DOT to Bancor pool
			// TODO check & lock if vsBond if expired
		}

		fn on_initialize(_n: BlockNumberFor<T>) -> frame_support::weights::Weight {
			// TODO estimate weight
			Zero::zero()
		}
	}

	impl<T: Config> Pallet<T> {
		/// The account ID of the fund pot.
		///
		/// This actually does computation. If you need to keep using it, then make sure you cache the
		/// value and only call this once.
		pub fn fund_account_id(index: ParaId) -> AccountIdOf<T> {
			T::PalletId::get().into_sub_account(index)
		}

		pub fn id_from_index(index: TrieIndex) -> child::ChildInfo {
			let mut buf = Vec::new();
			buf.extend_from_slice(&(T::PalletId::get().0));
			buf.extend_from_slice(&index.encode()[..]);
			child::ChildInfo::new_default(T::Hashing::hash(&buf[..]).as_ref())
		}

		pub fn contribution_put(index: TrieIndex, who: &AccountIdOf<T>, balance: &BalanceOf<T>) {
			who.using_encoded(|b| child::put(&Self::id_from_index(index), b, &(balance)));
		}

		pub fn contribution_get(index: TrieIndex, who: &AccountIdOf<T>) -> BalanceOf<T> {
			who.using_encoded(|b| {
				child::get_or_default::<BalanceOf<T>>(&Self::id_from_index(index), b)
			})
		}

		pub fn contribution_kill(index: TrieIndex, who: &AccountIdOf<T>) {
			who.using_encoded(|b| child::kill(&Self::id_from_index(index), b));
		}

		pub fn crowdloan_kill(index: TrieIndex) -> child::KillChildStorageResult {
			child::kill_storage(&Self::id_from_index(index), Some(T::RemoveKeysLimit::get()))
		}

		pub fn contribution_iterator(
			index: TrieIndex,
		) -> ChildTriePrefixIterator<(AccountIdOf<T>, (BalanceOf<T>, Vec<u8>))> {
			ChildTriePrefixIterator::<_>::with_prefix_over_key::<Identity>(
				&Self::id_from_index(index),
				&[],
			)
		}

		pub fn xcm_ump_contribute_to_parachain(
			origin: OriginFor<T>,
			para_id: ParaId,
			value: BalanceOf<T>,
		) -> Result<(), XcmError> {
			let origin_location: MultiLocation = T::SendXcmOrigin::ensure_origin(origin)
				.map_err(|_e| XcmError::ConvertOriginFailed)?;

			let contribution = Contribution {
				index: para_id,
				value,
				signature: None,
			};

			let call =
				CrowdloanPalletCall::CrowdloanContribute(ContributeCall::Contribute(contribution))
					.encode();

			let msg = Xcm::Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: u64::MAX,
				call: call.into(),
			};

			let message = Xcm::<()>::RelayedFrom {
				who: origin_location,
				message: Box::new(msg),
			};

			match T::XcmSender::send_xcm(MultiLocation::X1(Junction::Parent), message.clone()) {
				Ok(()) => {
					log::info!(
						target: "salp",
						"crowdloan transact sent success message as {:?}",
						message,
					);
				}
				Err(e) => {
					log::error!(
						target: "salp",
						"crowdloan transact sent failed error as {:?}",
						e,
					);
					return Err(XcmError::ContributeSentFailed);
				}
			}
			Ok(())
		}
	}
}
