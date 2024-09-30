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

pub mod migration;
pub mod traits;
pub mod weights;
pub use weights::WeightInfo;

use bb_bnc::traits::BbBNCInterface;
use bifrost_asset_registry::AssetMetadata;
use bifrost_primitives::{
	CurrencyId, CurrencyIdConversion, CurrencyIdExt, CurrencyIdMapping, CurrencyIdRegister,
	RedeemType, SlpOperator, SlpxOperator, TimeUnit, VTokenMintRedeemProvider,
	VTokenSupplyProvider, VtokenMintingInterface, VtokenMintingOperator,
};
use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{
			AccountIdConversion, CheckedAdd, CheckedSub, Saturating, UniqueSaturatedInto, Zero,
		},
		ArithmeticError, DispatchError, FixedU128, Permill, SaturatedConversion,
	},
	traits::LockIdentifier,
	transactional, BoundedVec, PalletId,
};
use frame_system::pallet_prelude::*;
use log;
use orml_traits::{MultiCurrency, MultiLockableCurrency};
pub use pallet::*;
use sp_core::U256;
use sp_std::{vec, vec::Vec};
pub use traits::*;
use xcm::v3::MultiLocation;

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
	use bifrost_primitives::{
		currency::BNC, AstarChainId, HydrationChainId, InterlayChainId, MantaChainId, FIL,
	};
	use frame_support::pallet_prelude::DispatchResultWithPostInfo;
	use orml_traits::XcmTransfer;
	use xcm::{prelude::*, v4::Location};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiLockableCurrency<AccountIdOf<Self>>;

		/// The only origin that can edit token issuer list
		type ControlOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Handler to notify the runtime when redeem success
		/// If you don't need it, you can specify the type `()`.
		type OnRedeemSuccess: OnRedeemSuccess<
			AccountIdOf<Self>,
			CurrencyIdOf<Self>,
			BalanceOf<Self>,
		>;

		/// xtokens xcm transfer interface
		type XcmTransfer: XcmTransfer<AccountIdOf<Self>, BalanceOf<Self>, CurrencyIdOf<Self>>;

		/// The amount of mint
		#[pallet::constant]
		type MaximumUnlockIdOfUser: Get<u32>;

		#[pallet::constant]
		type MaximumUnlockIdOfTimeUnit: Get<u32>;

		// maximum unlocked vtoken records minted in an incentive mode
		#[pallet::constant]
		type MaxLockRecords: Get<u32>;

		#[pallet::constant]
		type EntranceAccount: Get<PalletId>;

		#[pallet::constant]
		type ExitAccount: Get<PalletId>;

		#[pallet::constant]
		type FeeAccount: Get<Self::AccountId>;

		#[pallet::constant]
		type RedeemFeeAccount: Get<Self::AccountId>;

		#[pallet::constant]
		type IncentivePoolAccount: Get<PalletId>;

		#[pallet::constant]
		type RelayChainToken: Get<CurrencyId>;

		#[pallet::constant]
		type MoonbeamChainId: Get<u32>;

		type BifrostSlp: SlpOperator<CurrencyId>;

		type BifrostSlpx: SlpxOperator<BalanceOf<Self>>;

		// bbBNC interface
		type BbBNC: BbBNCInterface<
			AccountIdOf<Self>,
			CurrencyIdOf<Self>,
			BalanceOf<Self>,
			BlockNumberFor<Self>,
		>;

		type CurrencyIdConversion: CurrencyIdConversion<CurrencyId>;

		type CurrencyIdRegister: CurrencyIdRegister<CurrencyId>;

		type ChannelCommission: VTokenMintRedeemProvider<CurrencyId, BalanceOf<Self>>;

		type AssetIdMaps: CurrencyIdMapping<
			CurrencyId,
			MultiLocation,
			AssetMetadata<BalanceOf<Self>>,
		>;

		/// Set default weight.
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Minted {
			address: AccountIdOf<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
			vtoken_amount: BalanceOf<T>,
			fee: BalanceOf<T>,
			remark: BoundedVec<u8, ConstU32<32>>,
			channel_id: Option<u32>,
		},
		Redeemed {
			address: AccountIdOf<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
			vtoken_amount: BalanceOf<T>,
			fee: BalanceOf<T>,
			unlock_id: UnlockId,
		},
		RedeemSuccess {
			unlock_id: UnlockId,
			token_id: CurrencyIdOf<T>,
			to: RedeemTo<AccountIdOf<T>>,
			token_amount: BalanceOf<T>,
		},
		Rebonded {
			address: AccountIdOf<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
			vtoken_amount: BalanceOf<T>,
			fee: BalanceOf<T>,
		},
		RebondedByUnlockId {
			address: AccountIdOf<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
			vtoken_amount: BalanceOf<T>,
			fee: BalanceOf<T>,
			unlock_id: UnlockId,
		},
		UnlockDurationSet {
			token_id: CurrencyIdOf<T>,
			unlock_duration: TimeUnit,
		},
		MinimumMintSet {
			token_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		},
		MinimumRedeemSet {
			token_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		},
		SupportRebondTokenAdded {
			token_id: CurrencyIdOf<T>,
		},
		SupportRebondTokenRemoved {
			token_id: CurrencyIdOf<T>,
		},
		/// Several fees has been set.
		FeeSet {
			mint_fee: Permill,
			redeem_fee: Permill,
			// hosting_fee: BalanceOf<T>,
		},
		HookIterationLimitSet {
			limit: u32,
		},
		UnlockingTotalSet {
			token_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		},
		MinTimeUnitSet {
			token_id: CurrencyIdOf<T>,
			time_unit: TimeUnit,
		},
		FastRedeemFailed {
			err: DispatchError,
		},
		CurrencyTimeUnitRecreated {
			token_id: CurrencyIdOf<T>,
			time_unit: TimeUnit,
		},
		IncentivizedMinting {
			address: AccountIdOf<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
			locked_vtoken_amount: BalanceOf<T>,
			incentive_vtoken_amount: BalanceOf<T>,
		},
		VtokenIncentiveCoefSet {
			vtoken_id: CurrencyIdOf<T>,
			coefficient: Option<u128>,
		},
		VtokenIncentiveLockBlocksSet {
			vtoken_id: CurrencyIdOf<T>,
			blocks: Option<BlockNumberFor<T>>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		BelowMinimumMint,
		BelowMinimumRedeem,
		/// Invalid token to rebond.
		InvalidRebondToken,
		/// Token type not support.
		NotSupportTokenType,
		NotEnoughBalanceToUnlock,
		TokenToRebondNotZero,
		OngoingTimeUnitNotSet,
		TokenUnlockLedgerNotFound,
		UserUnlockLedgerNotFound,
		TimeUnitUnlockLedgerNotFound,
		UnlockDurationNotFound,
		Unexpected,
		CalculationOverflow,
		ExceedMaximumUnlockId,
		TooManyRedeems,
		CanNotRedeem,
		CanNotRebond,
		NotEnoughBalance,
		VeBNCCheckingError,
		IncentiveCoefNotFound,
		TooManyLocks,
		ConvertError,
		NoUnlockRecord,
		FailToRemoveLock,
		BalanceZero,
		IncentiveLockBlocksNotSet,
	}

	#[pallet::storage]
	pub type Fees<T: Config> = StorageValue<_, (Permill, Permill), ValueQuery>;

	#[pallet::storage]
	pub type TokenPool<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	pub type UnlockDuration<T: Config> = StorageMap<_, Twox64Concat, CurrencyIdOf<T>, TimeUnit>;

	#[pallet::storage]
	pub type OngoingTimeUnit<T: Config> = StorageMap<_, Twox64Concat, CurrencyIdOf<T>, TimeUnit>;

	#[pallet::storage]
	pub type MinimumMint<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	pub type MinimumRedeem<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	pub type TokenUnlockNextId<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, u32, ValueQuery>;

	#[pallet::storage]
	pub type TokenUnlockLedger<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		Blake2_128Concat,
		UnlockId,
		(T::AccountId, BalanceOf<T>, TimeUnit, RedeemType<AccountIdOf<T>>),
		OptionQuery,
	>;

	#[pallet::storage]
	pub type UserUnlockLedger<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		(BalanceOf<T>, BoundedVec<UnlockId, T::MaximumUnlockIdOfUser>),
		OptionQuery,
	>;

	#[pallet::storage]
	pub type TimeUnitUnlockLedger<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		TimeUnit,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		(BalanceOf<T>, BoundedVec<UnlockId, T::MaximumUnlockIdOfTimeUnit>, CurrencyIdOf<T>),
		OptionQuery,
	>;

	#[pallet::storage]
	pub type TokenToRebond<T: Config> = StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>>;

	#[pallet::storage]
	pub type MinTimeUnit<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, TimeUnit, ValueQuery>;

	#[pallet::storage]
	pub type UnlockingTotal<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

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
			Self::handle_on_initialize()
				.map_err(|err| {
					Self::deposit_event(Event::FastRedeemFailed { err });
					log::error!(
						target: "runtime::vtoken-minting",
						"Received invalid justification for {:?}",
						err,
					);
					err
				})
				.ok();

			T::WeightInfo::on_initialize()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::mint())]
		pub fn mint(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
			remark: BoundedVec<u8, ConstU32<32>>,
			channel_id: Option<u32>,
		) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;
			Self::mint_inner(exchanger, token_id, token_amount, remark, channel_id).map(|_| ())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::redeem())]
		pub fn redeem(
			origin: OriginFor<T>,
			vtoken_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let exchanger = ensure_signed(origin)?;
			Self::redeem_inner(exchanger, vtoken_id, vtoken_amount, RedeemType::Native)
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::rebond())]
		pub fn rebond(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			let vtoken_id = T::CurrencyIdConversion::convert_to_vtoken(token_id)
				.map_err(|_| Error::<T>::NotSupportTokenType)?;
			let _token_amount_to_rebond =
				TokenToRebond::<T>::get(token_id).ok_or(Error::<T>::InvalidRebondToken)?;
			if let Some((user_unlock_amount, mut ledger_list)) =
				UserUnlockLedger::<T>::get(&exchanger, token_id)
			{
				ensure!(user_unlock_amount >= token_amount, Error::<T>::NotEnoughBalanceToUnlock);
				let mut tmp_amount = token_amount;
				let ledger_list_rev: Vec<UnlockId> = ledger_list.into_iter().rev().collect();
				ledger_list =
					BoundedVec::<UnlockId, T::MaximumUnlockIdOfUser>::try_from(ledger_list_rev)
						.map_err(|_| Error::<T>::ExceedMaximumUnlockId)?;
				let mut tmp = ledger_list
					.iter()
					.map(|&index| -> Result<(UnlockId, bool), Error<T>> {
						if let Some((_, unlock_amount, time_unit, _)) =
							TokenUnlockLedger::<T>::get(token_id, index)
						{
							if tmp_amount >= unlock_amount {
								if let Some((_, _, time_unit, _)) =
									TokenUnlockLedger::<T>::take(&token_id, &index)
								{
									TimeUnitUnlockLedger::<T>::mutate_exists(
										&time_unit,
										&token_id,
										|value| -> Result<(), Error<T>> {
											if let Some((
												total_locked_origin,
												ledger_list_origin,
												_,
											)) = value
											{
												if total_locked_origin == &unlock_amount {
													*value = None;
													return Ok(());
												}
												*total_locked_origin = total_locked_origin
													.checked_sub(&unlock_amount)
													.ok_or(Error::<T>::CalculationOverflow)?;
												ledger_list_origin.retain(|&x| x != index);
											} else {
												return Err(
													Error::<T>::TimeUnitUnlockLedgerNotFound,
												);
											}
											Ok(())
										},
									)?;
									tmp_amount = tmp_amount.saturating_sub(unlock_amount);
								} else {
									return Err(Error::<T>::TokenUnlockLedgerNotFound.into());
								}
								Ok((index, false))
							} else {
								TokenUnlockLedger::<T>::mutate_exists(
									&token_id,
									&index,
									|value| -> Result<(), Error<T>> {
										if let Some((_, total_locked_origin, _, _)) = value {
											if total_locked_origin == &tmp_amount {
												*value = None;
												return Ok(());
											}
											*total_locked_origin = total_locked_origin
												.checked_sub(&tmp_amount)
												.ok_or(Error::<T>::CalculationOverflow)?;
										} else {
											return Err(Error::<T>::TokenUnlockLedgerNotFound);
										}
										Ok(())
									},
								)?;
								TimeUnitUnlockLedger::<T>::mutate_exists(
									&time_unit,
									&token_id,
									|value| -> Result<(), Error<T>> {
										if let Some((total_locked_origin, _, _)) = value {
											if total_locked_origin == &tmp_amount {
												*value = None;
												return Ok(());
											}
											*total_locked_origin = total_locked_origin
												.checked_sub(&tmp_amount)
												.ok_or(Error::<T>::CalculationOverflow)?;
										} else {
											return Err(Error::<T>::TimeUnitUnlockLedgerNotFound);
										}
										Ok(())
									},
								)?;
								Ok((index, true))
							}
						} else {
							Ok((index, true))
						}
					})
					.collect::<Result<Vec<(UnlockId, bool)>, Error<T>>>()?;
				tmp.retain(|(_index, result)| *result);

				let ledger_list_tmp: Vec<UnlockId> =
					tmp.into_iter().map(|(index, _)| index).rev().collect();

				ledger_list =
					BoundedVec::<UnlockId, T::MaximumUnlockIdOfUser>::try_from(ledger_list_tmp)
						.map_err(|_| Error::<T>::ExceedMaximumUnlockId)?;

				UnlockingTotal::<T>::mutate(&token_id, |pool| -> Result<(), Error<T>> {
					*pool =
						pool.checked_sub(&token_amount).ok_or(Error::<T>::CalculationOverflow)?;
					Ok(())
				})?;
				UserUnlockLedger::<T>::mutate_exists(
					&exchanger,
					&token_id,
					|value| -> Result<(), Error<T>> {
						if let Some((total_locked_origin, ledger_list_origin)) = value {
							if total_locked_origin == &token_amount {
								*value = None;
								return Ok(());
							}
							*ledger_list_origin = ledger_list;
							*total_locked_origin = total_locked_origin
								.checked_sub(&token_amount)
								.ok_or(Error::<T>::CalculationOverflow)?;
						} else {
							return Err(Error::<T>::UserUnlockLedgerNotFound);
						}
						Ok(())
					},
				)?;
			} else {
				return Err(Error::<T>::UserUnlockLedgerNotFound.into());
			}

			let (_, vtoken_amount, fee) =
				Self::mint_without_tranfer(&exchanger, vtoken_id, token_id, token_amount)?;

			TokenToRebond::<T>::mutate(&token_id, |value| -> Result<(), Error<T>> {
				if let Some(value_info) = value {
					*value_info = value_info
						.checked_add(&token_amount)
						.ok_or(Error::<T>::CalculationOverflow)?;
				} else {
					return Err(Error::<T>::InvalidRebondToken);
				}
				Ok(())
			})?;

			Self::deposit_event(Event::Rebonded {
				address: exchanger,
				token_id,
				token_amount,
				vtoken_amount,
				fee,
			});
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::rebond_by_unlock_id())]
		pub fn rebond_by_unlock_id(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			unlock_id: UnlockId,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;

			let vtoken_id = T::CurrencyIdConversion::convert_to_vtoken(token_id)
				.map_err(|_| Error::<T>::NotSupportTokenType)?;
			let _token_amount_to_rebond =
				TokenToRebond::<T>::get(token_id).ok_or(Error::<T>::InvalidRebondToken)?;

			let unlock_amount = match TokenUnlockLedger::<T>::get(token_id, unlock_id) {
				Some((who, unlock_amount, time_unit, _)) => {
					ensure!(who == exchanger, Error::<T>::CanNotRebond);
					TimeUnitUnlockLedger::<T>::mutate_exists(
						&time_unit,
						&token_id,
						|value| -> Result<(), Error<T>> {
							if let Some((total_locked_origin, ledger_list_origin, _)) = value {
								if total_locked_origin == &unlock_amount {
									*value = None;
									return Ok(());
								}
								*total_locked_origin = total_locked_origin
									.checked_sub(&unlock_amount)
									.ok_or(Error::<T>::CalculationOverflow)?;
								ledger_list_origin.retain(|&x| x != unlock_id);
							} else {
								return Err(Error::<T>::TimeUnitUnlockLedgerNotFound);
							}
							Ok(())
						},
					)?;

					UserUnlockLedger::<T>::mutate_exists(
						&who,
						&token_id,
						|value| -> Result<(), Error<T>> {
							if let Some((total_locked_origin, ledger_list_origin)) = value {
								if total_locked_origin == &unlock_amount {
									*value = None;
									return Ok(());
								}
								*total_locked_origin = total_locked_origin
									.checked_sub(&unlock_amount)
									.ok_or(Error::<T>::CalculationOverflow)?;
								ledger_list_origin.retain(|&x| x != unlock_id);
							} else {
								return Err(Error::<T>::UserUnlockLedgerNotFound);
							}
							Ok(())
						},
					)?;
					UnlockingTotal::<T>::mutate(&token_id, |pool| -> Result<(), Error<T>> {
						*pool = pool
							.checked_sub(&unlock_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;
						Ok(())
					})?;

					TokenUnlockLedger::<T>::remove(&token_id, &unlock_id);
					unlock_amount
				},
				_ => return Err(Error::<T>::TokenUnlockLedgerNotFound.into()),
			};

			let (token_amount, vtoken_amount, fee) =
				Self::mint_without_tranfer(&exchanger, vtoken_id, token_id, unlock_amount)?;

			TokenToRebond::<T>::mutate(&token_id, |value| -> Result<(), Error<T>> {
				if let Some(value_info) = value {
					*value_info = value_info
						.checked_add(&token_amount)
						.ok_or(Error::<T>::CalculationOverflow)?;
				} else {
					return Err(Error::<T>::InvalidRebondToken);
				}
				Ok(())
			})?;

			Self::deposit_event(Event::RebondedByUnlockId {
				address: exchanger,
				token_id,
				token_amount: unlock_amount,
				vtoken_amount,
				fee,
				unlock_id,
			});
			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::set_unlock_duration())]
		pub fn set_unlock_duration(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			unlock_duration: TimeUnit,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			UnlockDuration::<T>::mutate(token_id, |old_unlock_duration| {
				*old_unlock_duration = Some(unlock_duration.clone());
			});

			Self::deposit_event(Event::UnlockDurationSet { token_id, unlock_duration });

			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::set_minimum_mint())]
		pub fn set_minimum_mint(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			if !MinimumMint::<T>::contains_key(token_id) {
				// mutate_exists
				MinimumMint::<T>::insert(token_id, amount);
			} else {
				MinimumMint::<T>::mutate(token_id, |old_amount| {
					*old_amount = amount;
				});
			}

			match token_id {
				CurrencyId::Token(token_symbol) =>
					if !T::CurrencyIdRegister::check_vtoken_registered(token_symbol) {
						T::CurrencyIdRegister::register_vtoken_metadata(token_symbol)?;
					},
				CurrencyId::Token2(token_id) => {
					if !T::CurrencyIdRegister::check_vtoken2_registered(token_id) {
						T::CurrencyIdRegister::register_vtoken2_metadata(token_id)?;
					}
				},
				_ => (),
			}

			Self::deposit_event(Event::MinimumMintSet { token_id, amount });

			Ok(())
		}

		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::set_minimum_redeem())]
		pub fn set_minimum_redeem(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			MinimumRedeem::<T>::mutate(token_id, |old_amount| {
				*old_amount = amount;
			});

			Self::deposit_event(Event::MinimumRedeemSet { token_id, amount });
			Ok(())
		}

		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::add_support_rebond_token())]
		pub fn add_support_rebond_token(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			if !TokenToRebond::<T>::contains_key(token_id) {
				TokenToRebond::<T>::insert(token_id, BalanceOf::<T>::zero());
				Self::deposit_event(Event::SupportRebondTokenAdded { token_id });
			}

			Ok(())
		}

		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::remove_support_rebond_token())]
		pub fn remove_support_rebond_token(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			if TokenToRebond::<T>::contains_key(token_id) {
				let token_amount_to_rebond =
					TokenToRebond::<T>::get(token_id).ok_or(Error::<T>::InvalidRebondToken)?;
				ensure!(
					token_amount_to_rebond == BalanceOf::<T>::zero(),
					Error::<T>::TokenToRebondNotZero
				);

				TokenToRebond::<T>::remove(token_id);
				Self::deposit_event(Event::SupportRebondTokenRemoved { token_id });
			}
			Ok(())
		}

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

		#[pallet::call_index(11)]
		#[pallet::weight(T::WeightInfo::set_unlocking_total())]
		pub fn set_unlocking_total(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			UnlockingTotal::<T>::mutate(&token_id, |unlocking_total| *unlocking_total = amount);

			Self::deposit_event(Event::UnlockingTotalSet { token_id, amount });
			Ok(())
		}

		#[pallet::call_index(12)]
		#[pallet::weight(T::WeightInfo::set_min_time_unit())]
		pub fn set_min_time_unit(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			time_unit: TimeUnit,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			MinTimeUnit::<T>::mutate(&token_id, |old_time_unit| *old_time_unit = time_unit.clone());

			Self::deposit_event(Event::MinTimeUnitSet { token_id, time_unit });
			Ok(())
		}

		#[pallet::call_index(13)]
		#[pallet::weight(T::WeightInfo::recreate_currency_ongoing_time_unit())]
		pub fn recreate_currency_ongoing_time_unit(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			time_unit: TimeUnit,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			OngoingTimeUnit::<T>::mutate(&token_id, |old_time_unit| {
				*old_time_unit = Some(time_unit.clone())
			});

			Self::deposit_event(Event::CurrencyTimeUnitRecreated { token_id, time_unit });
			Ok(())
		}

		// mint with lock to get incentive vtoken
		#[pallet::call_index(14)]
		#[pallet::weight(T::WeightInfo::mint_with_lock())]
		pub fn mint_with_lock(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
			remark: BoundedVec<u8, ConstU32<32>>,
			channel_id: Option<u32>,
		) -> DispatchResult {
			// Check origin
			let minter = ensure_signed(origin)?;

			// check if the minter has at least token_amount of token_id which is transferable
			T::MultiCurrency::ensure_can_withdraw(token_id, &minter, token_amount)
				.map_err(|_| Error::<T>::NotEnoughBalance)?;

			// check whether the token_id is supported
			ensure!(MinimumMint::<T>::contains_key(token_id), Error::<T>::NotSupportTokenType);

			// check whether the user has veBNC
			let vebnc_balance =
				T::BbBNC::balance_of(&minter, None).map_err(|_| Error::<T>::VeBNCCheckingError)?;
			ensure!(vebnc_balance > BalanceOf::<T>::zero(), Error::<T>::NotEnoughBalance);

			// check whether the vtoken coefficient is set
			let vtoken_id = T::CurrencyIdConversion::convert_to_vtoken(token_id)
				.map_err(|_| Error::<T>::NotSupportTokenType)?;

			ensure!(
				VtokenIncentiveCoef::<T>::contains_key(vtoken_id),
				Error::<T>::IncentiveCoefNotFound
			);

			// check whether the pool has balance of vtoken_id
			let incentive_pool_account = &Self::incentive_pool_account();
			let vtoken_pool_balance =
				T::MultiCurrency::free_balance(vtoken_id, &incentive_pool_account);

			ensure!(vtoken_pool_balance > BalanceOf::<T>::zero(), Error::<T>::NotEnoughBalance);

			// mint vtoken
			let vtoken_minted =
				Self::mint_inner(minter.clone(), token_id, token_amount, remark, channel_id)?;

			// lock vtoken and record the lock
			Self::lock_vtoken_for_incentive_minting(minter.clone(), vtoken_id, vtoken_minted)?;

			// calculate the incentive amount
			let incentive_amount =
				Self::calculate_incentive_vtoken_amount(&minter, vtoken_id, vtoken_minted)?;

			// Since the user has already locked the vtoken, we can directly transfer the incentive
			// vtoken. It won't fail. transfer the incentive amount to the minter
			T::MultiCurrency::transfer(
				vtoken_id,
				incentive_pool_account,
				&minter,
				incentive_amount,
			)
			.map_err(|_| Error::<T>::NotEnoughBalance)?;

			// deposit event
			Self::deposit_event(Event::IncentivizedMinting {
				address: minter,
				token_id,
				token_amount,
				locked_vtoken_amount: vtoken_minted,
				incentive_vtoken_amount: incentive_amount,
			});

			Ok(())
		}

		#[pallet::call_index(15)]
		#[pallet::weight(T::WeightInfo::unlock_incentive_minted_vtoken())]
		pub fn unlock_incentive_minted_vtoken(
			origin: OriginFor<T>,
			vtoken_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			let unlocker = ensure_signed(origin)?;

			// get the user's VtokenLockLedger
			ensure!(
				VtokenLockLedger::<T>::contains_key(&unlocker, vtoken_id),
				Error::<T>::UserUnlockLedgerNotFound
			);

			VtokenLockLedger::<T>::mutate_exists(
				&unlocker,
				vtoken_id,
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
							T::MultiCurrency::remove_lock(INCENTIVE_LOCK_ID, vtoken_id, &unlocker)
								.map_err(|_| Error::<T>::FailToRemoveLock)?;

							// remove the ledger
							*maybe_ledger = None;
						} else {
							// update the ledger
							*ledger = (remaining_locked_amount, lock_records);

							// reset the locked amount to be remaining_locked_amount
							T::MultiCurrency::set_lock(
								INCENTIVE_LOCK_ID,
								vtoken_id,
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

		#[pallet::call_index(16)]
		#[pallet::weight(T::WeightInfo::set_incentive_coef())]
		pub fn set_incentive_coef(
			origin: OriginFor<T>,
			vtoken_id: CurrencyIdOf<T>,
			new_coef_op: Option<u128>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			if let Some(new_coef) = new_coef_op {
				VtokenIncentiveCoef::<T>::insert(vtoken_id, new_coef);
			} else {
				VtokenIncentiveCoef::<T>::remove(vtoken_id);
			}

			Self::deposit_event(Event::VtokenIncentiveCoefSet {
				vtoken_id,
				coefficient: new_coef_op,
			});

			Ok(())
		}

		#[pallet::call_index(17)]
		#[pallet::weight(T::WeightInfo::set_vtoken_incentive_lock_blocks())]
		pub fn set_vtoken_incentive_lock_blocks(
			origin: OriginFor<T>,
			vtoken_id: CurrencyIdOf<T>,
			new_blockes_op: Option<BlockNumberFor<T>>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			if let Some(new_blocks) = new_blockes_op {
				MintWithLockBlocks::<T>::insert(vtoken_id, new_blocks);
			} else {
				MintWithLockBlocks::<T>::remove(vtoken_id);
			}

			Self::deposit_event(Event::VtokenIncentiveLockBlocksSet {
				vtoken_id,
				blocks: new_blockes_op,
			});

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		#[transactional]
		pub fn add_time_unit(a: TimeUnit, b: TimeUnit) -> Result<TimeUnit, DispatchError> {
			let result = match a {
				TimeUnit::Era(era_a) => match b {
					TimeUnit::Era(era_b) => TimeUnit::Era(
						era_a.checked_add(era_b).ok_or(Error::<T>::CalculationOverflow)?,
					),
					_ => return Err(Error::<T>::Unexpected.into()),
				},
				TimeUnit::Round(round_a) => match b {
					TimeUnit::Round(round_b) => TimeUnit::Round(
						round_a.checked_add(round_b).ok_or(Error::<T>::CalculationOverflow)?,
					),
					_ => return Err(Error::<T>::Unexpected.into()),
				},
				TimeUnit::SlashingSpan(slashing_span_a) => match b {
					TimeUnit::SlashingSpan(slashing_span_b) => TimeUnit::SlashingSpan(
						slashing_span_a
							.checked_add(slashing_span_b)
							.ok_or(Error::<T>::CalculationOverflow)?,
					),
					_ => return Err(Error::<T>::Unexpected.into()),
				},
				TimeUnit::Kblock(kblock_a) => match b {
					TimeUnit::Kblock(kblock_b) => TimeUnit::Kblock(
						kblock_a.checked_add(kblock_b).ok_or(Error::<T>::CalculationOverflow)?,
					),
					_ => return Err(Error::<T>::Unexpected.into()),
				},
				TimeUnit::Hour(hour_a) => match b {
					TimeUnit::Hour(hour_b) => TimeUnit::Hour(
						hour_a.checked_add(hour_b).ok_or(Error::<T>::CalculationOverflow)?,
					),
					_ => return Err(Error::<T>::Unexpected.into()),
				},
				// _ => return Err(Error::<T>::Unexpected.into()),
			};

			Ok(result)
		}

		#[transactional]
		pub fn mint_without_tranfer(
			exchanger: &AccountIdOf<T>,
			vtoken_id: CurrencyId,
			token_id: CurrencyId,
			token_amount: BalanceOf<T>,
		) -> Result<(BalanceOf<T>, BalanceOf<T>, BalanceOf<T>), DispatchError> {
			let token_pool_amount = TokenPool::<T>::get(token_id);
			let vtoken_total_issuance = T::MultiCurrency::total_issuance(vtoken_id);
			let (mint_rate, _redeem_rate) = Fees::<T>::get();
			let mint_fee = mint_rate * token_amount;
			let token_amount_excluding_fee =
				token_amount.checked_sub(&mint_fee).ok_or(Error::<T>::CalculationOverflow)?;
			let mut vtoken_amount = token_amount_excluding_fee;
			if token_pool_amount != BalanceOf::<T>::zero() {
				vtoken_amount = U256::from(token_amount_excluding_fee.saturated_into::<u128>())
					.saturating_mul(vtoken_total_issuance.saturated_into::<u128>().into())
					.checked_div(token_pool_amount.saturated_into::<u128>().into())
					.map(|x| u128::try_from(x))
					.ok_or(Error::<T>::CalculationOverflow)?
					.map_err(|_| Error::<T>::CalculationOverflow)?
					.unique_saturated_into();
			}

			// Charging fees
			T::MultiCurrency::transfer(token_id, exchanger, &T::FeeAccount::get(), mint_fee)?;
			// Issue the corresponding vtoken to the user's account.
			T::MultiCurrency::deposit(vtoken_id, exchanger, vtoken_amount)?;
			TokenPool::<T>::mutate(&token_id, |pool| -> Result<(), Error<T>> {
				*pool = pool
					.checked_add(&token_amount_excluding_fee)
					.ok_or(Error::<T>::CalculationOverflow)?;
				Ok(())
			})?;
			Ok((token_amount_excluding_fee, vtoken_amount, mint_fee))
		}

		#[transactional]
		fn on_initialize_update_ledger(
			token_id: CurrencyId,
			account: AccountIdOf<T>,
			index: &UnlockId,
			mut unlock_amount: BalanceOf<T>,
			entrance_account_balance: BalanceOf<T>,
			time_unit: TimeUnit,
			redeem_type: RedeemType<AccountIdOf<T>>,
		) -> DispatchResult {
			let ed = T::MultiCurrency::minimum_balance(token_id);
			let mut account_to_send = account.clone();
			let mut redeem_to = RedeemTo::Native(account_to_send.clone());

			if unlock_amount < ed {
				let receiver_balance = T::MultiCurrency::total_balance(token_id, &account);

				let receiver_balance_after = receiver_balance
					.checked_add(&unlock_amount)
					.ok_or(ArithmeticError::Overflow)?;
				if receiver_balance_after < ed {
					account_to_send = T::FeeAccount::get();
					redeem_to = RedeemTo::Native(T::FeeAccount::get());
				}
			}
			if entrance_account_balance >= unlock_amount {
				T::MultiCurrency::transfer(
					token_id,
					&T::EntranceAccount::get().into_account_truncating(),
					&account_to_send,
					unlock_amount,
				)?;
				TokenUnlockLedger::<T>::remove(&token_id, &index);

				TimeUnitUnlockLedger::<T>::mutate_exists(
					&time_unit,
					&token_id,
					|value| -> Result<(), Error<T>> {
						if let Some((total_locked_origin, ledger_list_origin, _)) = value {
							if total_locked_origin == &unlock_amount {
								*value = None;
								return Ok(());
							}
							*total_locked_origin = total_locked_origin
								.checked_sub(&unlock_amount)
								.ok_or(Error::<T>::CalculationOverflow)?;
							ledger_list_origin.retain(|x| x != index);
						} else {
							return Err(Error::<T>::TimeUnitUnlockLedgerNotFound);
						}
						Ok(())
					},
				)?;

				UserUnlockLedger::<T>::mutate_exists(
					&account,
					&token_id,
					|value| -> Result<(), Error<T>> {
						if let Some((total_locked_origin, ledger_list_origin)) = value {
							if total_locked_origin == &unlock_amount {
								*value = None;
								return Ok(());
							}
							ledger_list_origin.retain(|x| x != index);
							*total_locked_origin = total_locked_origin
								.checked_sub(&unlock_amount)
								.ok_or(Error::<T>::CalculationOverflow)?;
						} else {
							return Err(Error::<T>::UserUnlockLedgerNotFound);
						}
						Ok(())
					},
				)?;
				match redeem_type {
					RedeemType::Native => {},
					RedeemType::Astar(receiver) => {
						let dest = Location::new(
							1,
							[
								Parachain(AstarChainId::get()),
								AccountId32 {
									network: None,
									id: receiver.encode().try_into().unwrap(),
								},
							],
						);
						T::XcmTransfer::transfer(
							account.clone(),
							token_id,
							unlock_amount,
							dest,
							Unlimited,
						)?;
						redeem_to = RedeemTo::Astar(receiver);
					},
					RedeemType::Hydradx(receiver) => {
						let dest = Location::new(
							1,
							[
								Parachain(HydrationChainId::get()),
								AccountId32 {
									network: None,
									id: receiver.encode().try_into().unwrap(),
								},
							],
						);
						T::XcmTransfer::transfer(
							account.clone(),
							token_id,
							unlock_amount,
							dest,
							Unlimited,
						)?;
						redeem_to = RedeemTo::Hydradx(receiver);
					},
					RedeemType::Interlay(receiver) => {
						let dest = Location::new(
							1,
							[
								Parachain(InterlayChainId::get()),
								AccountId32 {
									network: None,
									id: receiver.encode().try_into().unwrap(),
								},
							],
						);
						T::XcmTransfer::transfer(
							account.clone(),
							token_id,
							unlock_amount,
							dest,
							Unlimited,
						)?;
						redeem_to = RedeemTo::Interlay(receiver);
					},
					RedeemType::Manta(receiver) => {
						let dest = Location::new(
							1,
							[
								Parachain(MantaChainId::get()),
								AccountId32 {
									network: None,
									id: receiver.encode().try_into().unwrap(),
								},
							],
						);
						T::XcmTransfer::transfer(
							account.clone(),
							token_id,
							unlock_amount,
							dest,
							Unlimited,
						)?;
						redeem_to = RedeemTo::Manta(receiver);
					},
					RedeemType::Moonbeam(receiver) => {
						let dest = Location::new(
							1,
							[
								Parachain(T::MoonbeamChainId::get()),
								AccountKey20 { network: None, key: receiver.to_fixed_bytes() },
							],
						);
						if token_id == FIL {
							let assets = vec![
								(token_id, unlock_amount),
								(BNC, T::BifrostSlpx::get_moonbeam_transfer_to_fee()),
							];

							T::XcmTransfer::transfer_multicurrencies(
								account.clone(),
								assets,
								1,
								dest,
								Unlimited,
							)?;
						} else {
							T::XcmTransfer::transfer(
								account.clone(),
								token_id,
								unlock_amount,
								dest,
								Unlimited,
							)?;
						}
						redeem_to = RedeemTo::Moonbeam(receiver);
					},
				};
			} else {
				match redeem_type {
					RedeemType::Astar(_) |
					RedeemType::Moonbeam(_) |
					RedeemType::Hydradx(_) |
					RedeemType::Manta(_) |
					RedeemType::Interlay(_) => {
						return Ok(());
					},
					RedeemType::Native => {},
				};
				unlock_amount = entrance_account_balance;
				T::MultiCurrency::transfer(
					token_id,
					&T::EntranceAccount::get().into_account_truncating(),
					&account_to_send,
					unlock_amount,
				)?;
				TokenUnlockLedger::<T>::mutate_exists(
					&token_id,
					&index,
					|value| -> Result<(), Error<T>> {
						if let Some((_, total_locked_origin, _, _)) = value {
							if total_locked_origin == &unlock_amount {
								*value = None;
								return Ok(());
							}
							*total_locked_origin = total_locked_origin
								.checked_sub(&unlock_amount)
								.ok_or(Error::<T>::CalculationOverflow)?;
						} else {
							return Err(Error::<T>::TokenUnlockLedgerNotFound);
						}
						Ok(())
					},
				)?;

				TimeUnitUnlockLedger::<T>::mutate_exists(
					&time_unit,
					&token_id,
					|value| -> Result<(), Error<T>> {
						if let Some((total_locked_origin, _ledger_list_origin, _)) = value {
							if total_locked_origin == &unlock_amount {
								*value = None;
								return Ok(());
							}
							*total_locked_origin = total_locked_origin
								.checked_sub(&unlock_amount)
								.ok_or(Error::<T>::CalculationOverflow)?;
						} else {
							return Err(Error::<T>::TimeUnitUnlockLedgerNotFound);
						}
						Ok(())
					},
				)?;

				UserUnlockLedger::<T>::mutate_exists(
					&account,
					&token_id,
					|value| -> Result<(), Error<T>> {
						if let Some((total_locked_origin, _ledger_list_origin)) = value {
							if total_locked_origin == &unlock_amount {
								*value = None;
								return Ok(());
							}

							*total_locked_origin = total_locked_origin
								.checked_sub(&unlock_amount)
								.ok_or(Error::<T>::CalculationOverflow)?;
						} else {
							return Err(Error::<T>::UserUnlockLedgerNotFound);
						}
						Ok(())
					},
				)?;
			}

			entrance_account_balance
				.checked_sub(&unlock_amount)
				.ok_or(Error::<T>::CalculationOverflow)?;

			UnlockingTotal::<T>::mutate(&token_id, |pool| -> Result<(), Error<T>> {
				*pool = pool.checked_sub(&unlock_amount).ok_or(Error::<T>::CalculationOverflow)?;
				Ok(())
			})?;

			T::OnRedeemSuccess::on_redeem_success(token_id, account.clone(), unlock_amount);

			Self::deposit_event(Event::RedeemSuccess {
				unlock_id: *index,
				token_id,
				to: redeem_to,
				token_amount: unlock_amount,
			});
			Ok(())
		}

		#[transactional]
		fn handle_on_initialize() -> DispatchResult {
			for currency in OngoingTimeUnit::<T>::iter_keys() {
				Self::handle_ledger_by_currency(currency)?;
			}
			Ok(())
		}

		fn handle_ledger_by_currency(currency: CurrencyId) -> DispatchResult {
			let time_unit = MinTimeUnit::<T>::get(currency);
			let unlock_duration_elem = match UnlockDuration::<T>::get(currency) {
				Some(TimeUnit::Era(unlock_duration_era)) => unlock_duration_era,
				Some(TimeUnit::Round(unlock_duration_round)) => unlock_duration_round,
				Some(TimeUnit::Kblock(unlock_duration_kblock)) => unlock_duration_kblock,
				Some(TimeUnit::Hour(unlock_duration_hour)) => unlock_duration_hour,
				_ => 0,
			};
			let ongoing_elem = match OngoingTimeUnit::<T>::get(currency) {
				Some(TimeUnit::Era(ongoing_era)) => ongoing_era,
				Some(TimeUnit::Round(ongoing_round)) => ongoing_round,
				Some(TimeUnit::Kblock(ongoing_kblock)) => ongoing_kblock,
				Some(TimeUnit::Hour(ongoing_hour)) => ongoing_hour,
				_ => 0,
			};
			if let Some((_total_locked, ledger_list, token_id)) =
				TimeUnitUnlockLedger::<T>::get(time_unit.clone(), currency)
			{
				for index in ledger_list.iter().take(HookIterationLimit::<T>::get() as usize) {
					if let Some((account, unlock_amount, time_unit, redeem_type)) =
						TokenUnlockLedger::<T>::get(token_id, index)
					{
						let entrance_account_balance = T::MultiCurrency::free_balance(
							token_id,
							&T::EntranceAccount::get().into_account_truncating(),
						);
						if entrance_account_balance != BalanceOf::<T>::zero() {
							Self::on_initialize_update_ledger(
								token_id,
								account,
								index,
								unlock_amount,
								entrance_account_balance,
								time_unit,
								redeem_type,
							)
							.ok();
						}
					}
				}
			} else {
				MinTimeUnit::<T>::mutate(currency, |time_unit| -> Result<(), Error<T>> {
					match time_unit {
						TimeUnit::Era(era) => {
							if ongoing_elem + unlock_duration_elem > *era {
								*era = era.checked_add(1).ok_or(Error::<T>::CalculationOverflow)?;
							}
							Ok(())
						},
						TimeUnit::Round(round) => {
							if ongoing_elem + unlock_duration_elem > *round {
								*round =
									round.checked_add(1).ok_or(Error::<T>::CalculationOverflow)?;
							}
							Ok(())
						},
						TimeUnit::Kblock(kblock) => {
							if ongoing_elem + unlock_duration_elem > *kblock {
								*kblock =
									kblock.checked_add(1).ok_or(Error::<T>::CalculationOverflow)?;
							}
							Ok(())
						},
						TimeUnit::Hour(hour) => {
							if ongoing_elem + unlock_duration_elem > *hour {
								*hour =
									hour.checked_add(1).ok_or(Error::<T>::CalculationOverflow)?;
							}
							Ok(())
						},
						_ => Ok(()),
					}
				})?;
			};

			Ok(())
		}

		#[transactional]
		pub fn mint_inner(
			exchanger: AccountIdOf<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
			remark: BoundedVec<u8, ConstU32<32>>,
			channel_id: Option<u32>,
		) -> Result<BalanceOf<T>, DispatchError> {
			ensure!(token_amount >= MinimumMint::<T>::get(token_id), Error::<T>::BelowMinimumMint);

			let vtoken_id = T::CurrencyIdConversion::convert_to_vtoken(token_id)
				.map_err(|_| Error::<T>::NotSupportTokenType)?;
			let (token_amount_excluding_fee, vtoken_amount, fee) =
				Self::mint_without_tranfer(&exchanger, vtoken_id, token_id, token_amount)?;
			// Transfer the user's token to EntranceAccount.
			T::MultiCurrency::transfer(
				token_id,
				&exchanger,
				&T::EntranceAccount::get().into_account_truncating(),
				token_amount_excluding_fee,
			)?;

			// record the minting information for ChannelCommission module
			T::ChannelCommission::record_mint_amount(channel_id, vtoken_id, vtoken_amount)?;

			Self::deposit_event(Event::Minted {
				address: exchanger,
				token_id,
				token_amount,
				vtoken_amount,
				fee,
				remark,
				channel_id,
			});
			Ok(vtoken_amount.into())
		}

		#[transactional]
		pub fn redeem_inner(
			exchanger: AccountIdOf<T>,
			vtoken_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
			redeem_type: RedeemType<AccountIdOf<T>>,
		) -> DispatchResultWithPostInfo {
			let token_id = T::CurrencyIdConversion::convert_to_token(vtoken_id)
				.map_err(|_| Error::<T>::NotSupportTokenType)?;
			ensure!(
				vtoken_amount >= MinimumRedeem::<T>::get(vtoken_id),
				Error::<T>::BelowMinimumRedeem
			);

			ensure!(
				!T::BifrostSlp::all_delegation_requests_occupied(token_id),
				Error::<T>::CanNotRedeem,
			);

			let (_mint_rate, redeem_rate) = Fees::<T>::get();
			let redeem_fee = redeem_rate * vtoken_amount;
			let vtoken_amount =
				vtoken_amount.checked_sub(&redeem_fee).ok_or(Error::<T>::CalculationOverflow)?;
			// Charging fees
			T::MultiCurrency::transfer(
				vtoken_id,
				&exchanger,
				&T::RedeemFeeAccount::get(),
				redeem_fee,
			)?;

			let token_pool_amount = TokenPool::<T>::get(token_id);
			let vtoken_total_issuance = T::MultiCurrency::total_issuance(vtoken_id);
			let token_amount: BalanceOf<T> = U256::from(vtoken_amount.saturated_into::<u128>())
				.saturating_mul(token_pool_amount.saturated_into::<u128>().into())
				.checked_div(vtoken_total_issuance.saturated_into::<u128>().into())
				.map(|x| u128::try_from(x))
				.ok_or(Error::<T>::CalculationOverflow)?
				.map_err(|_| Error::<T>::CalculationOverflow)?
				.unique_saturated_into();

			let next_id = TokenUnlockNextId::<T>::get(token_id);
			match OngoingTimeUnit::<T>::get(token_id) {
				Some(time_unit) => {
					// Calculate the time to be locked
					let result_time_unit = Self::add_time_unit(
						UnlockDuration::<T>::get(token_id)
							.ok_or(Error::<T>::UnlockDurationNotFound)?,
						time_unit,
					)?;

					T::MultiCurrency::withdraw(vtoken_id, &exchanger, vtoken_amount)?;
					TokenPool::<T>::mutate(&token_id, |pool| -> Result<(), Error<T>> {
						*pool = pool
							.checked_sub(&token_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;
						Ok(())
					})?;
					UnlockingTotal::<T>::mutate(&token_id, |pool| -> Result<(), Error<T>> {
						*pool = pool
							.checked_add(&token_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;
						Ok(())
					})?;
					TokenUnlockLedger::<T>::insert(
						&token_id,
						&next_id,
						(&exchanger, token_amount, &result_time_unit, redeem_type),
					);

					if UserUnlockLedger::<T>::get(&exchanger, &token_id).is_some() {
						UserUnlockLedger::<T>::mutate(
							&exchanger,
							&token_id,
							|value| -> Result<(), Error<T>> {
								if let Some((total_locked, ledger_list)) = value {
									ledger_list
										.try_push(next_id)
										.map_err(|_| Error::<T>::TooManyRedeems)?;

									*total_locked = total_locked
										.checked_add(&token_amount)
										.ok_or(Error::<T>::CalculationOverflow)?;
								};
								Ok(())
							},
						)?;
					} else {
						let mut ledger_list_origin =
							BoundedVec::<UnlockId, T::MaximumUnlockIdOfUser>::default();
						ledger_list_origin
							.try_push(next_id)
							.map_err(|_| Error::<T>::TooManyRedeems)?;
						UserUnlockLedger::<T>::insert(
							&exchanger,
							&token_id,
							(token_amount, ledger_list_origin),
						);
					}

					if let Some((_, _, _token_id)) =
						TimeUnitUnlockLedger::<T>::get(&result_time_unit, &token_id)
					{
						TimeUnitUnlockLedger::<T>::mutate(
							&result_time_unit,
							&token_id,
							|value| -> Result<(), Error<T>> {
								if let Some((total_locked, ledger_list, _token_id)) = value {
									ledger_list
										.try_push(next_id)
										.map_err(|_| Error::<T>::TooManyRedeems)?;
									*total_locked = total_locked
										.checked_add(&token_amount)
										.ok_or(Error::<T>::CalculationOverflow)?;
								};
								Ok(())
							},
						)?;
					} else {
						let mut ledger_list_origin =
							BoundedVec::<UnlockId, T::MaximumUnlockIdOfTimeUnit>::default();
						ledger_list_origin
							.try_push(next_id)
							.map_err(|_| Error::<T>::TooManyRedeems)?;

						TimeUnitUnlockLedger::<T>::insert(
							&result_time_unit,
							&token_id,
							(token_amount, ledger_list_origin, token_id),
						);
					}
				},
				None => return Err(Error::<T>::OngoingTimeUnitNotSet.into()),
			}

			TokenUnlockNextId::<T>::mutate(&token_id, |unlock_id| -> Result<(), Error<T>> {
				*unlock_id = unlock_id.checked_add(1).ok_or(Error::<T>::CalculationOverflow)?;
				Ok(())
			})?;

			let extra_weight = T::OnRedeemSuccess::on_redeemed(
				exchanger.clone(),
				token_id,
				token_amount,
				vtoken_amount,
				redeem_fee,
			);

			T::ChannelCommission::record_redeem_amount(vtoken_id, vtoken_amount)?;

			Self::deposit_event(Event::Redeemed {
				address: exchanger,
				token_id,
				vtoken_amount,
				token_amount,
				fee: redeem_fee,
				unlock_id: next_id,
			});
			Ok(Some(T::WeightInfo::redeem() + extra_weight).into())
		}

		pub fn token_to_vtoken_inner(
			token_id: CurrencyIdOf<T>,
			vtoken_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		) -> Result<BalanceOf<T>, DispatchError> {
			let token_pool_amount = TokenPool::<T>::get(token_id);
			let vtoken_total_issuance = T::MultiCurrency::total_issuance(vtoken_id);

			let value = U256::from(token_amount.saturated_into::<u128>())
				.saturating_mul(vtoken_total_issuance.saturated_into::<u128>().into())
				.checked_div(token_pool_amount.saturated_into::<u128>().into())
				.ok_or(Error::<T>::CalculationOverflow)?;

			Ok(u128::try_from(value)
				.map(|x| x.unique_saturated_into())
				.map_err(|_| Error::<T>::CalculationOverflow)?)
		}

		pub fn vtoken_to_token_inner(
			token_id: CurrencyIdOf<T>,
			vtoken_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
		) -> Result<BalanceOf<T>, DispatchError> {
			let token_pool_amount = TokenPool::<T>::get(token_id);
			let vtoken_total_issuance = T::MultiCurrency::total_issuance(vtoken_id);

			let value = U256::from(vtoken_amount.saturated_into::<u128>())
				.saturating_mul(token_pool_amount.saturated_into::<u128>().into())
				.checked_div(vtoken_total_issuance.saturated_into::<u128>().into())
				.ok_or(Error::<T>::CalculationOverflow)?;

			Ok(u128::try_from(value)
				.map(|x| x.unique_saturated_into())
				.map_err(|_| Error::<T>::CalculationOverflow)?)
		}

		pub fn vtoken_id_inner(token_id: CurrencyIdOf<T>) -> Option<CurrencyIdOf<T>> {
			T::CurrencyIdConversion::convert_to_vtoken(token_id).ok()
		}

		pub fn token_id_inner(vtoken_id: CurrencyIdOf<T>) -> Option<CurrencyIdOf<T>> {
			T::CurrencyIdConversion::convert_to_token(vtoken_id).ok()
		}

		pub fn incentive_pool_account() -> AccountIdOf<T> {
			T::IncentivePoolAccount::get().into_account_truncating()
		}

		// to lock user vtoken for incentive minting
		fn lock_vtoken_for_incentive_minting(
			minter: AccountIdOf<T>,
			vtoken_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
		) -> Result<(), Error<T>> {
			// first, lock the vtoken
			// second, record the lock in ledger

			// check whether the minter has enough vtoken
			T::MultiCurrency::ensure_can_withdraw(vtoken_id, &minter, vtoken_amount)
				.map_err(|_| Error::<T>::NotEnoughBalance)?;

			// new amount that should be locked
			let mut new_lock_total = vtoken_amount;

			// check the previous locked amount under the same vtoken_id from ledger
			// and revise ledger to set the new_amount to be previous_amount + vtoken_amount
			VtokenLockLedger::<T>::mutate_exists(
				&minter,
				&vtoken_id,
				|value| -> Result<(), Error<T>> {
					// get the vtoken lock duration from VtokenIncentiveCoef
					let lock_duration = MintWithLockBlocks::<T>::get(vtoken_id)
						.ok_or(Error::<T>::IncentiveLockBlocksNotSet)?;
					let current_block = frame_system::Pallet::<T>::block_number();
					let due_block = current_block
						.checked_add(&lock_duration)
						.ok_or(Error::<T>::CalculationOverflow)?;

					if let Some(ref mut ledger) = value {
						new_lock_total = ledger
							.0
							.checked_add(&vtoken_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;

						ledger.0 = new_lock_total;

						// push new item to the boundedvec of the ledger
						ledger
							.1
							.try_push((vtoken_amount, due_block))
							.map_err(|_| Error::<T>::TooManyLocks)?;
					} else {
						let item = BoundedVec::try_from(vec![(vtoken_amount, due_block)])
							.map_err(|_| Error::<T>::ConvertError)?;

						*value = Some((vtoken_amount, item));
					}
					Ok(())
				},
			)?;

			// extend the locked amount to be new_lock_total
			T::MultiCurrency::set_lock(INCENTIVE_LOCK_ID, vtoken_id, &minter, new_lock_total)
				.map_err(|_| Error::<T>::NotEnoughBalance)?;

			Ok(())
		}

		fn calculate_incentive_vtoken_amount(
			minter: &AccountIdOf<T>,
			vtoken_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
		) -> Result<BalanceOf<T>, Error<T>> {
			// get the vtoken pool balance
			let vtoken_pool_balance =
				T::MultiCurrency::free_balance(vtoken_id, &Self::incentive_pool_account());
			ensure!(vtoken_pool_balance > BalanceOf::<T>::zero(), Error::<T>::NotEnoughBalance);

			// get current block number
			let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
			// get the veBNC total amount
			let vebnc_total_issuance = T::BbBNC::total_supply(current_block_number)
				.map_err(|_| Error::<T>::VeBNCCheckingError)?;
			ensure!(vebnc_total_issuance > BalanceOf::<T>::zero(), Error::<T>::BalanceZero);

			// get the veBNC balance of the minter
			let minter_vebnc_balance =
				T::BbBNC::balance_of(minter, None).map_err(|_| Error::<T>::VeBNCCheckingError)?;
			ensure!(minter_vebnc_balance > BalanceOf::<T>::zero(), Error::<T>::NotEnoughBalance);

			// get the percentage of the veBNC balance of the minter to the total veBNC amount and
			// get the square root of the percentage
			let percentage = Permill::from_rational(minter_vebnc_balance, vebnc_total_issuance);
			let sqrt_percentage =
				FixedU128::from_inner(percentage * 1_000_000_000_000_000_000u128).sqrt();
			let percentage = Permill::from_rational(
				sqrt_percentage.into_inner(),
				1_000_000_000_000_000_000u128.into(),
			);
			// get the total issuance of the vtoken
			let vtoken_total_issuance = T::MultiCurrency::total_issuance(vtoken_id);

			// get the incentive coef for the vtoken
			let incentive_coef = VtokenIncentiveCoef::<T>::get(vtoken_id)
				.ok_or(Error::<T>::IncentiveCoefNotFound)?;

			// calculate the incentive amount, but mind the overflow
			// incentive_amount = vtoken_pool_balance * incentive_coef * vtoken_amount *
			// sqrt_percentage / vtoken_total_issuance
			let incentive_amount =
				U256::from(percentage.mul_ceil(vtoken_pool_balance).saturated_into::<u128>())
					.checked_mul(U256::from(incentive_coef))
					.and_then(|x| x.checked_mul(U256::from(vtoken_amount.saturated_into::<u128>())))
					// .and_then(|x| x.checked_mul(percentage))
					.and_then(|x| {
						x.checked_div(U256::from(vtoken_total_issuance.saturated_into::<u128>()))
					})
					// first turn into u128，then use unique_saturated_into BalanceOf<T>
					.map(|x| x.saturated_into::<u128>())
					.map(|x| x.unique_saturated_into())
					.ok_or(Error::<T>::CalculationOverflow)?;

			Ok(incentive_amount)
		}

		pub fn get_exchange_rate(
			token_id: Option<CurrencyId>,
		) -> Result<Vec<(CurrencyIdOf<T>, U256)>, DispatchError> {
			let mut result: Vec<(CurrencyIdOf<T>, U256)> = Vec::new();

			match token_id {
				Some(token_id) => {
					let vtoken_amount = Self::get_vtoken_amount(token_id, 1000u128)?;
					result.push((token_id, vtoken_amount));
				},
				None =>
					for token_id in T::AssetIdMaps::get_all_currency() {
						if token_id.is_vtoken() {
							let vtoken_id = token_id;
							let token_id = T::CurrencyIdConversion::convert_to_token(vtoken_id)
								.map_err(|_| Error::<T>::NotSupportTokenType)?;

							let vtoken_amount = Self::get_vtoken_amount(token_id, 1000u128)?;
							result.push((token_id, vtoken_amount));
						}
					},
			}
			Ok(result)
		}

		fn get_vtoken_amount(token: CurrencyIdOf<T>, amount: u128) -> Result<U256, DispatchError> {
			let vtoken_id = T::CurrencyIdConversion::convert_to_vtoken(token)
				.map_err(|_| Error::<T>::NotSupportTokenType)?;

			let token_pool_amount = TokenPool::<T>::get(token);
			let vtoken_total_issuance = T::MultiCurrency::total_issuance(vtoken_id);

			let mut vtoken_amount = U256::from(amount);
			if token_pool_amount != BalanceOf::<T>::zero() {
				let vtoken_total_issuance_u256 =
					U256::from(vtoken_total_issuance.saturated_into::<u128>());
				let token_pool_amount_u256 = U256::from(token_pool_amount.saturated_into::<u128>());

				vtoken_amount = vtoken_amount
					.saturating_mul(vtoken_total_issuance_u256)
					.checked_div(token_pool_amount_u256)
					.ok_or(Error::<T>::CalculationOverflow)?;
			}
			Ok(vtoken_amount)
		}
	}
}

impl<T: Config> VtokenMintingOperator<CurrencyId, BalanceOf<T>, AccountIdOf<T>, TimeUnit>
	for Pallet<T>
{
	fn get_token_pool(currency_id: CurrencyId) -> BalanceOf<T> {
		TokenPool::<T>::get(currency_id)
	}

	fn increase_token_pool(currency_id: CurrencyId, token_amount: BalanceOf<T>) -> DispatchResult {
		TokenPool::<T>::mutate(currency_id, |pool| -> Result<(), Error<T>> {
			*pool = pool.checked_add(&token_amount).ok_or(Error::<T>::CalculationOverflow)?;

			Ok(())
		})?;

		Ok(())
	}

	fn decrease_token_pool(currency_id: CurrencyId, token_amount: BalanceOf<T>) -> DispatchResult {
		TokenPool::<T>::mutate(currency_id, |pool| -> Result<(), Error<T>> {
			*pool = pool.checked_sub(&token_amount).ok_or(Error::<T>::CalculationOverflow)?;
			Ok(())
		})?;

		Ok(())
	}

	fn update_ongoing_time_unit(currency_id: CurrencyId, time_unit: TimeUnit) -> DispatchResult {
		OngoingTimeUnit::<T>::mutate(currency_id, |time_unit_old| -> Result<(), Error<T>> {
			*time_unit_old = Some(time_unit);
			Ok(())
		})?;

		Ok(())
	}

	fn get_ongoing_time_unit(currency_id: CurrencyId) -> Option<TimeUnit> {
		OngoingTimeUnit::<T>::get(currency_id)
	}

	fn get_unlock_records(
		currency_id: CurrencyId,
		time_unit: TimeUnit,
	) -> Option<(BalanceOf<T>, Vec<u32>)> {
		if let Some((balance, list, _)) = TimeUnitUnlockLedger::<T>::get(&time_unit, currency_id) {
			Some((balance, list.into_inner()))
		} else {
			None
		}
	}

	#[transactional]
	fn deduct_unlock_amount(
		currency_id: CurrencyId,
		index: u32,
		deduct_amount: BalanceOf<T>,
	) -> DispatchResult {
		if let Some((who, unlock_amount, time_unit, _)) =
			TokenUnlockLedger::<T>::get(currency_id, index)
		{
			ensure!(unlock_amount >= deduct_amount, Error::<T>::NotEnoughBalanceToUnlock);

			UnlockingTotal::<T>::mutate(&currency_id, |pool| -> Result<(), Error<T>> {
				*pool = pool.checked_sub(&deduct_amount).ok_or(Error::<T>::CalculationOverflow)?;
				Ok(())
			})?;

			TimeUnitUnlockLedger::<T>::mutate_exists(
				&time_unit,
				&currency_id,
				|value| -> Result<(), Error<T>> {
					if let Some((total_locked_origin, ledger_list_origin, _)) = value {
						if total_locked_origin == &deduct_amount {
							*value = None;
							return Ok(());
						}
						*total_locked_origin = total_locked_origin
							.checked_sub(&deduct_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;
						if unlock_amount == deduct_amount {
							ledger_list_origin.retain(|&x| x != index);
						}
					} else {
						return Err(Error::<T>::TimeUnitUnlockLedgerNotFound);
					}
					Ok(())
				},
			)?;

			UserUnlockLedger::<T>::mutate_exists(
				&who,
				&currency_id,
				|value| -> Result<(), Error<T>> {
					if let Some((total_locked_origin, ledger_list_origin)) = value {
						if total_locked_origin == &deduct_amount {
							*value = None;
							return Ok(());
						}
						*total_locked_origin = total_locked_origin
							.checked_sub(&deduct_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;
						if unlock_amount == deduct_amount {
							ledger_list_origin.retain(|&x| x != index);
						}
					} else {
						return Err(Error::<T>::UserUnlockLedgerNotFound);
					}
					Ok(())
				},
			)?;

			if unlock_amount == deduct_amount {
				TokenUnlockLedger::<T>::remove(&currency_id, &index);
			} else {
				TokenUnlockLedger::<T>::mutate_exists(
					&currency_id,
					&index,
					|value| -> Result<(), Error<T>> {
						if let Some((_, total_locked_origin, _, _)) = value {
							if total_locked_origin == &deduct_amount {
								*value = None;
								return Ok(());
							}
							*total_locked_origin = total_locked_origin
								.checked_sub(&deduct_amount)
								.ok_or(Error::<T>::CalculationOverflow)?;
						} else {
							return Err(Error::<T>::TokenUnlockLedgerNotFound);
						}
						Ok(())
					},
				)?;
			}
		}
		Ok(())
	}

	fn get_entrance_and_exit_accounts() -> (AccountIdOf<T>, AccountIdOf<T>) {
		(
			T::EntranceAccount::get().into_account_truncating(),
			T::ExitAccount::get().into_account_truncating(),
		)
	}

	fn get_token_unlock_ledger(
		currency_id: CurrencyId,
		index: u32,
	) -> Option<(AccountIdOf<T>, BalanceOf<T>, TimeUnit, RedeemType<AccountIdOf<T>>)> {
		TokenUnlockLedger::<T>::get(currency_id, index)
	}

	fn get_moonbeam_parachain_id() -> u32 {
		T::MoonbeamChainId::get()
	}
}

impl<T: Config> VtokenMintingInterface<AccountIdOf<T>, CurrencyIdOf<T>, BalanceOf<T>>
	for Pallet<T>
{
	fn mint(
		exchanger: AccountIdOf<T>,
		token_id: CurrencyIdOf<T>,
		token_amount: BalanceOf<T>,
		remark: BoundedVec<u8, ConstU32<32>>,
		channel_id: Option<u32>,
	) -> Result<BalanceOf<T>, DispatchError> {
		Self::mint_inner(exchanger, token_id, token_amount, remark, channel_id)
	}

	fn redeem(
		exchanger: AccountIdOf<T>,
		vtoken_id: CurrencyIdOf<T>,
		vtoken_amount: BalanceOf<T>,
	) -> DispatchResultWithPostInfo {
		Self::redeem_inner(exchanger, vtoken_id, vtoken_amount, RedeemType::Native)
	}

	fn slpx_redeem(
		exchanger: AccountIdOf<T>,
		vtoken_id: CurrencyIdOf<T>,
		vtoken_amount: BalanceOf<T>,
		redeem_type: RedeemType<AccountIdOf<T>>,
	) -> DispatchResultWithPostInfo {
		Self::redeem_inner(exchanger, vtoken_id, vtoken_amount, redeem_type)
	}

	fn token_to_vtoken(
		token_id: CurrencyIdOf<T>,
		vtoken_id: CurrencyIdOf<T>,
		token_amount: BalanceOf<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		Self::token_to_vtoken_inner(token_id, vtoken_id, token_amount)
	}

	fn vtoken_to_token(
		token_id: CurrencyIdOf<T>,
		vtoken_id: CurrencyIdOf<T>,
		vtoken_amount: BalanceOf<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		Self::vtoken_to_token_inner(token_id, vtoken_id, vtoken_amount)
	}

	fn vtoken_id(token_id: CurrencyIdOf<T>) -> Option<CurrencyIdOf<T>> {
		Self::vtoken_id_inner(token_id)
	}

	fn token_id(vtoken_id: CurrencyIdOf<T>) -> Option<CurrencyIdOf<T>> {
		Self::token_id_inner(vtoken_id)
	}

	fn get_minimums_redeem(vtoken_id: CurrencyIdOf<T>) -> BalanceOf<T> {
		MinimumRedeem::<T>::get(vtoken_id)
	}

	fn get_token_pool(currency_id: CurrencyId) -> BalanceOf<T> {
		TokenPool::<T>::get(currency_id)
	}

	fn get_moonbeam_parachain_id() -> u32 {
		T::MoonbeamChainId::get()
	}
}

impl<T: Config> VTokenSupplyProvider<CurrencyIdOf<T>, BalanceOf<T>> for Pallet<T> {
	fn get_vtoken_supply(vtoken: CurrencyIdOf<T>) -> Option<BalanceOf<T>> {
		if CurrencyId::is_vtoken(&vtoken) {
			Some(T::MultiCurrency::total_issuance(vtoken))
		} else {
			None
		}
	}

	fn get_token_supply(token: CurrencyIdOf<T>) -> Option<BalanceOf<T>> {
		if CurrencyId::is_token(&token) | CurrencyId::is_native(&token) {
			Some(TokenPool::<T>::get(token))
		} else {
			None
		}
	}
}
