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

extern crate alloc;

use alloc::{vec, vec::Vec};
use bifrost_primitives::{
	CurrencyId, CurrencyIdExt, SlpHostingFeeProvider, VTokenMintRedeemProvider,
};
use frame_support::{pallet_prelude::*, PalletId};
use frame_system::pallet_prelude::*;
use orml_traits::MultiCurrency;
use sp_io::MultiRemovalResults;
use sp_runtime::{
	helpers_128bit::multiply_by_rational_with_rounding,
	traits::{AccountIdConversion, CheckedAdd, UniqueSaturatedFrom, UniqueSaturatedInto, Zero},
	PerThing, Percent, Permill, Rounding, SaturatedConversion, Saturating,
};
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod mock;
mod tests;
pub mod weights;

pub use pallet::*;

type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::Balance;
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type ChannelId = u32;

const REMOVE_TOKEN_LIMIT: u32 = 100;
const DEFAULT_COMMISSION_RATE: Percent = Percent::from_percent(20);

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Currency operation handler
		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		/// The only origin that can edit token issuer list
		type ControlOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Commission master Pallet Id to get the commission master account
		type CommissionPalletId: Get<PalletId>;

		/// The receiving account of Bifrost commission
		type BifrostCommissionReceiver: Get<AccountIdOf<Self>>;

		/// Weight information for extrinsic in this module.
		type WeightInfo: WeightInfo;

		// Commission clearing duration, in blocks
		#[pallet::constant]
		type ClearingDuration: Get<BlockNumberFor<Self>>;

		// The maximum bytes length of channel name
		#[pallet::constant]
		type NameLengthLimit: Get<u32>;
	}

	#[pallet::error]
	pub enum Error<T> {
		Overflow,
		ChannelNameTooLong,
		ConversionError,
		ChannelNotExist,
		TransferError,
		VtokenNotConfiguredForCommission,
		InvalidCommissionRate,
		CommissionTokenAlreadySet,
		InvalidVtoken,
		/// Error indicating that no changes were made during a modification operation.
		NoChangesMade,
		/// Represents an error that occurs when a division operation encounters a divisor of zero.
		/// This is a critical error, as division by zero is undefined and cannot be performed.
		DivisionByZero,
		/// Error indicating that the removal operation was not completed successfully.
		RemovalNotComplete,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		ChannelRegistered {
			channel_id: ChannelId,
			receive_account: AccountIdOf<T>,
			name: Vec<u8>,
		},
		ChannelRemoved {
			channel_id: ChannelId,
		},
		ChannelReceiveAccountUpdated {
			channel_id: ChannelId,
			receiver: AccountIdOf<T>,
		},
		CommissionTokenSet {
			vtoken: CurrencyId,
			commission_token: Option<CurrencyId>,
		},
		ChannelCommissionSet {
			channel_id: ChannelId,
			vtoken: CurrencyId,
			rate: Percent,
		},
		CommissionClaimed {
			channel_id: ChannelId,
			commission_token: CurrencyId,
			amount: BalanceOf<T>,
		},
		ChannelVtokenSharesUpdated {
			channel_id: ChannelId,
			vtoken: CurrencyId,
			share: Permill,
		},
		VtokenIssuanceSnapshotUpdated {
			vtoken: CurrencyId,
			old_issuance: BalanceOf<T>,
			new_issuance: BalanceOf<T>,
		},
		PeriodVtokenTotalMintUpdated {
			vtoken: CurrencyId,
			old_total_mint: BalanceOf<T>,
			new_total_mint: BalanceOf<T>,
		},
		PeriodVtokenTotalRedeemUpdated {
			vtoken: CurrencyId,
			old_total_redeem: BalanceOf<T>,
			new_total_redeem: BalanceOf<T>,
		},
		PeriodChannelVtokenMintUpdated {
			channel_id: ChannelId,
			vtoken: CurrencyId,
			old_mint_amount: BalanceOf<T>,
			new_mint_amount: BalanceOf<T>,
		},
		PeriodTotalCommissionsUpdated {
			commission_token: CurrencyId,
			old_amount: BalanceOf<T>,
			new_amount: BalanceOf<T>,
		},
		ChannelClaimableCommissionUpdated {
			channel_id: ChannelId,
			commission_token: CurrencyId,
			amount: BalanceOf<T>,
		},
		/// Emitted when a Permill calculation fails.
		/// This event carries the numerator and denominator that caused the failure.
		CalculationFailed {
			numerator: BalanceOf<T>,
			denominator: BalanceOf<T>,
		},
		/// Bifrost commission transfer failed.
		/// Parameters are the commission token and the amount that failed to transfer.
		BifrostCommissionTransferFailed {
			from: AccountIdOf<T>,
			to: AccountIdOf<T>,
			commission_token: CurrencyId,
			amount: BalanceOf<T>,
		},
		/// Error event indicating that the removal process of clearing was not completed.
		RemovalNotCompleteError {
			target_num: u32,
			limit: u32,
			executed_num: u32,
		},
	}

	/// Auto increment channel id
	#[pallet::storage]
	pub type ChannelNextId<T: Config> = StorageValue<_, ChannelId, ValueQuery>;

	/// Mapping a channel id to a receive account and a name, 【channel_id =>(receive_account,
	/// name)】
	#[pallet::storage]
	pub type Channels<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		ChannelId,
		(AccountIdOf<T>, BoundedVec<u8, T::NameLengthLimit>),
	>;

	/// Mapping a vtoken to a commission token, 【vtoken => commission_token】
	#[pallet::storage]
	pub type CommissionTokens<T> = StorageMap<_, Blake2_128Concat, CurrencyId, CurrencyId>;

	/// Mapping a channel + vtoken to corresponding commission rate, 【(channel_id, vtoken) =>
	/// commission rate】
	#[pallet::storage]
	pub type ChannelCommissionTokenRates<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ChannelId,
		Blake2_128Concat,
		CurrencyId,
		Percent,
		ValueQuery,
	>;

	/// Mapping a channel + vtoken to corresponding channel share, 【(channel_id, vtoken) => share】
	#[pallet::storage]
	pub type ChannelVtokenShares<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ChannelId,
		Blake2_128Concat,
		CurrencyId,
		Permill,
		ValueQuery,
	>;

	/// 【vtoken => (old_issuance, new_issuance)】,old_issuance is the vtoken issuance at last
	/// clearing point,  new_issuance is the ongoing accumulative issuance the last clearing point
	#[pallet::storage]
	pub type VtokenIssuanceSnapshots<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, (BalanceOf<T>, BalanceOf<T>), ValueQuery>;

	/// Vtoken total minted amount in the ongoing period for the chain, 【vtoken => (old_total_mint,
	/// new_total_mint)】
	#[pallet::storage]
	pub type PeriodVtokenTotalMint<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, (BalanceOf<T>, BalanceOf<T>), ValueQuery>;

	/// Vtoken total redeemed amount in the ongoing period for the chain, 【vtoken =>
	/// (old_total_redeem, new_total_redeem)】
	#[pallet::storage]
	pub type PeriodVtokenTotalRedeem<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, (BalanceOf<T>, BalanceOf<T>), ValueQuery>;

	/// Vtoken minted amount in the ongoing period for the channel, 【(channel_id, vtoken) =>
	/// (old_mint_amount, new_mint_amount)】
	#[pallet::storage]
	pub type PeriodChannelVtokenMint<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ChannelId,
		Blake2_128Concat,
		CurrencyId,
		(BalanceOf<T>, BalanceOf<T>),
		ValueQuery,
	>;

	/// Commission pool for last period and ongoing period, 【commission token => (old_amount,
	/// new_amount)】
	#[pallet::storage]
	pub type PeriodTotalCommissions<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, (BalanceOf<T>, BalanceOf<T>), ValueQuery>;

	/// Commission amount that has been cleared for the current clearing process, 【commission token
	/// => amount】
	#[pallet::storage]
	pub type PeriodClearedCommissions<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, BalanceOf<T>, ValueQuery>;

	/// Commission amount to be claimed by channels, 【channel id + commission token => amount】
	#[pallet::storage]
	pub type ChannelClaimableCommissions<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ChannelId,
		Blake2_128Concat,
		CurrencyId,
		BalanceOf<T>,
		ValueQuery,
	>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let channel_count: u32 = ChannelNextId::<T>::get().into();

			// get the commission token count
			let commission_token_count = CommissionTokens::<T>::iter().count() as u32;

			// If the current block number is the first block of a new clearing period, we need to
			// prepare data for clearing.
			if (n % T::ClearingDuration::get()).is_zero() {
				Self::set_clearing_environment();
			} else if (n % T::ClearingDuration::get()) < (channel_count + 1).into() {
				let channel_index = n % T::ClearingDuration::get() - 1u32.into();
				let channel_id: ChannelId =
					BlockNumberFor::<T>::unique_saturated_into(channel_index);
				Self::clear_channel_commissions(channel_id);
				Self::update_channel_vtoken_shares(channel_id);
			} else if (n % T::ClearingDuration::get()) == (channel_count + 1).into() {
				Self::clear_bifrost_commissions();
			}

			// Weight under the assumption that we have 30 vtoken
			T::WeightInfo::on_initialize(commission_token_count)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		// Weight under the assumption that we have 30 vtoken
		#[pallet::weight(T::WeightInfo::register_channel(CommissionTokens::<T>::iter().count() as u32))]
		pub fn register_channel(
			origin: OriginFor<T>,
			channel_name: Vec<u8>,
			receive_account: AccountIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			// add to Channels storage
			let channel_id = ChannelNextId::<T>::get();
			let name = BoundedVec::try_from(channel_name.clone())
				.map_err(|_| Error::<T>::ConversionError)?;
			Channels::<T>::insert(channel_id, (receive_account.clone(), name));

			// increase NextChannelId
			ChannelNextId::<T>::put(channel_id + 1);

			// for each vtoken, add the (channel_id + vtoken) to ChannelCommissionTokenRates storage
			CommissionTokens::<T>::iter_keys().for_each(|vtoken| {
				ChannelCommissionTokenRates::<T>::insert(
					channel_id,
					vtoken,
					DEFAULT_COMMISSION_RATE,
				);

				// for each vtoken, add the 0 share to ChannelVtokenShares storage
				ChannelVtokenShares::<T>::insert(channel_id, vtoken, Permill::zero());
			});

			Self::deposit_event(Event::ChannelRegistered {
				channel_id,
				receive_account,
				name: channel_name,
			});

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::remove_channel())]
		pub fn remove_channel(origin: OriginFor<T>, channel_id: ChannelId) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			// check if the channel exists
			ensure!(Channels::<T>::contains_key(channel_id), Error::<T>::ChannelNotExist);

			Self::settle_channel_commission(channel_id)?;

			// remove the channel from Channels storage
			Channels::<T>::remove(channel_id);

			// remove the channel from ChannelCommissionTokenRates storage
			Self::check_removed_all(ChannelCommissionTokenRates::<T>::clear_prefix(
				channel_id,
				REMOVE_TOKEN_LIMIT,
				None,
			))?;

			// remove the channel from ChannelVtokenShares storage
			Self::check_removed_all(ChannelVtokenShares::<T>::clear_prefix(
				channel_id,
				REMOVE_TOKEN_LIMIT,
				None,
			))?;

			// remove the channel from PeriodChannelVtokenMint storage
			Self::check_removed_all(PeriodChannelVtokenMint::<T>::clear_prefix(
				channel_id,
				REMOVE_TOKEN_LIMIT,
				None,
			))?;

			Self::deposit_event(Event::ChannelRemoved { channel_id });

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::update_channel_receive_account())]
		pub fn update_channel_receive_account(
			origin: OriginFor<T>,
			channel_id: ChannelId,
			receive_account: AccountIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			// If the channel exists, get the receive account; otherwise, return an error.
			let old_receive_account = Channels::<T>::get(channel_id)
				.map(|channel_info| channel_info.0)
				.ok_or(Error::<T>::ChannelNotExist)?;

			if old_receive_account == receive_account {
				return Err(Error::<T>::NoChangesMade.into());
			}

			// update the channel receive account
			Channels::<T>::mutate(channel_id, |channel_info| {
				if let Some(channel_info) = channel_info {
					channel_info.0 = receive_account.clone();
				}
			});

			Self::deposit_event(Event::ChannelReceiveAccountUpdated {
				channel_id,
				receiver: receive_account,
			});

			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::set_channel_commission_token())]
		pub fn set_channel_commission_token(
			origin: OriginFor<T>,
			channel_id: ChannelId,
			vtoken: CurrencyId,
			rate: Percent,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			ensure!(vtoken.is_vtoken(), Error::<T>::InvalidVtoken);

			// check if the channel exists
			ensure!(Channels::<T>::contains_key(channel_id), Error::<T>::ChannelNotExist);
			// check if the vtoken exists
			ensure!(
				CommissionTokens::<T>::contains_key(vtoken),
				Error::<T>::VtokenNotConfiguredForCommission
			);

			// if rate is None, remove the channel commission rate, if rate is Some, update the
			// channel commission rate
			if rate.is_zero() {
				ChannelCommissionTokenRates::<T>::remove(channel_id, vtoken);
			} else {
				ChannelCommissionTokenRates::<T>::insert(channel_id, vtoken, rate);
			}

			Self::deposit_event(Event::ChannelCommissionSet { channel_id, vtoken, rate });

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::set_commission_tokens())]
		pub fn set_commission_tokens(
			origin: OriginFor<T>,
			vtoken: CurrencyId,
			commission_token_op: Option<CurrencyId>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			ensure!(vtoken.is_vtoken(), Error::<T>::InvalidVtoken);

			// if old commission token is the same as the new one, do nothing
			if CommissionTokens::<T>::get(vtoken) == commission_token_op {
				return Err(Error::<T>::NoChangesMade.into());
			}

			if let Some(commission_token) = commission_token_op {
				// set the commission token
				CommissionTokens::<T>::insert(vtoken, commission_token);

				// set VtokenIssuanceSnapshots for the vtoken
				let issuance = T::MultiCurrency::total_issuance(vtoken);
				let zero_balance: BalanceOf<T> = Zero::zero();
				VtokenIssuanceSnapshots::<T>::insert(vtoken, (zero_balance, issuance));

				Self::deposit_event(Event::CommissionTokenSet {
					vtoken,
					commission_token: Some(commission_token),
				});
			} else {
				// remove the commission token、
				CommissionTokens::<T>::remove(vtoken);

				// remove the vtoken from VtokenIssuanceSnapshots
				VtokenIssuanceSnapshots::<T>::remove(vtoken);

				// remove the vtoken from PeriodVtokenTotalMint storage
				PeriodVtokenTotalMint::<T>::remove(vtoken);

				// remove the vtoken from PeriodVtokenTotalRedeem storage
				PeriodVtokenTotalRedeem::<T>::remove(vtoken);

				// for all channel_ids
				Channels::<T>::iter_keys().for_each(|channel_id| {
					// remove the vtoken from ChannelCommissionTokenRates storage
					ChannelCommissionTokenRates::<T>::remove(channel_id, vtoken);
					// remove the vtoken from ChannelVtokenShares storage
					ChannelVtokenShares::<T>::remove(channel_id, vtoken);
					// remove the vtoken from PeriodChannelVtokenMint storage
					PeriodChannelVtokenMint::<T>::remove(channel_id, vtoken);
				});

				// remove the vtoken from PeriodTotalCommissions storage
				PeriodTotalCommissions::<T>::remove(vtoken);

				// remove the vtoken from PeriodClearedCommissions storage
				PeriodClearedCommissions::<T>::remove(vtoken);

				// only ChannelClaimableCommissions not removed. Channel can still claim the
				// previous commission

				Self::deposit_event(Event::CommissionTokenSet { vtoken, commission_token: None });
			}

			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::claim_commissions())]
		pub fn claim_commissions(origin: OriginFor<T>, channel_id: ChannelId) -> DispatchResult {
			ensure_signed(origin)?;

			Self::settle_channel_commission(channel_id)?;
			Ok(())
		}

		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::set_channel_vtoken_shares(Channels::<T>::iter().count() as u32))]
		pub fn set_channel_vtoken_shares(
			origin: OriginFor<T>,
			channel_id: ChannelId,
			vtoken: CurrencyId,
			shares: Permill,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			// check if the channel exists
			ensure!(Channels::<T>::contains_key(channel_id), Error::<T>::ChannelNotExist);

			// check if the vtoken exists
			ensure!(
				CommissionTokens::<T>::contains_key(vtoken),
				Error::<T>::VtokenNotConfiguredForCommission
			);

			// get all channel_ids
			let channel_ids: Vec<ChannelId> = Channels::<T>::iter_keys().collect();
			// for each channel_id，get its vtoken share for the particular vtoken from the storage
			// if channel_id equals the passed in channel_id, we use the passed in share instead of
			// the storage one
			let mut total_shares = Permill::zero();
			for id in channel_ids {
				let share = if id == channel_id {
					shares
				} else {
					ChannelVtokenShares::<T>::get(id, vtoken)
				};

				// add up all the vtoken shares of all channels for this particular vtoken,
				// but use the passed in share for the passed in channel_id
				// if the sum of all shares is greater than 1, throw an error
				let total_shares_op = total_shares.checked_add(&share);

				total_shares = total_shares_op.ok_or_else(|| Error::<T>::InvalidCommissionRate)?;
			}

			// update the channel vtoken share
			ChannelVtokenShares::<T>::insert(channel_id, vtoken, shares);

			Self::deposit_event(Event::ChannelVtokenSharesUpdated {
				channel_id,
				vtoken,
				share: shares,
			});

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub(crate) fn set_clearing_environment() {
		//  Move the vtoken issuance amount from ongoing period to the previous period and clear the
		// ongoing period issuance amount
		let snapshots: Vec<CurrencyId> = VtokenIssuanceSnapshots::<T>::iter_keys().collect();
		for vtoken in snapshots {
			VtokenIssuanceSnapshots::<T>::mutate(vtoken, |issuance| {
				issuance.0 = issuance.1;

				// get the vtoken new issuance amount from Tokens module issuance storage
				let new_issuance = T::MultiCurrency::total_issuance(vtoken);

				issuance.1 = new_issuance;

				Self::deposit_event(Event::VtokenIssuanceSnapshotUpdated {
					vtoken,
					old_issuance: issuance.0,
					new_issuance,
				});
			});
		}

		// Move the total minted amount of the period from ongoing period to the previous period and
		// clear the ongoing period minted amount
		let snapshots: Vec<CurrencyId> = PeriodVtokenTotalMint::<T>::iter_keys().collect();
		for vtoken in snapshots {
			PeriodVtokenTotalMint::<T>::mutate(vtoken, |total_mint| {
				let info = *total_mint;

				total_mint.0 = total_mint.1;
				total_mint.1 = Zero::zero();

				Self::deposit_event(Event::PeriodVtokenTotalMintUpdated {
					vtoken,
					old_total_mint: info.0,
					new_total_mint: info.1,
				});
			});
		}

		// Move the total redeemed amount of the period from ongoing period to the previous period
		// and clear the ongoing period redeemed amount
		let snapshots: Vec<CurrencyId> = PeriodVtokenTotalRedeem::<T>::iter_keys().collect();
		for vtoken in snapshots {
			PeriodVtokenTotalRedeem::<T>::mutate(vtoken, |total_redeem| {
				let info = *total_redeem;

				total_redeem.0 = total_redeem.1;
				total_redeem.1 = Zero::zero();

				Self::deposit_event(Event::PeriodVtokenTotalRedeemUpdated {
					vtoken,
					old_total_redeem: info.0,
					new_total_redeem: info.1,
				});
			});
		}

		// Move the channel minted amount of the period from ongoing period to the previous period
		// and clear the ongoing period minted amount
		let snapshots: Vec<(ChannelId, CurrencyId)> =
			PeriodChannelVtokenMint::<T>::iter_keys().collect();
		for (channel_id, vtoken) in snapshots {
			PeriodChannelVtokenMint::<T>::mutate(channel_id, vtoken, |channel_vtoken_mint| {
				let info = *channel_vtoken_mint;

				channel_vtoken_mint.0 = channel_vtoken_mint.1;
				channel_vtoken_mint.1 = Zero::zero();

				Self::deposit_event(Event::PeriodChannelVtokenMintUpdated {
					channel_id,
					vtoken,
					old_mint_amount: info.0,
					new_mint_amount: info.1,
				});
			});
		}

		// Move the total commission amount of the period from ongoing period to the previous period
		// and clear the ongoing period commission amount
		let snapshots: Vec<CurrencyId> = PeriodTotalCommissions::<T>::iter_keys().collect();
		for commission_token in snapshots {
			PeriodTotalCommissions::<T>::mutate(commission_token, |total_commission| {
				let info = *total_commission;

				total_commission.0 = total_commission.1;
				total_commission.1 = Zero::zero();

				Self::deposit_event(Event::PeriodTotalCommissionsUpdated {
					commission_token,
					old_amount: info.0,
					new_amount: info.1,
				});
			});
		}
	}

	pub(crate) fn clear_channel_commissions(channel_id: ChannelId) {
		// check if the channel exists
		if !Channels::<T>::contains_key(channel_id) {
			return;
		}

		// calculate the commission amount for each commission token
		ChannelVtokenShares::<T>::iter_prefix(channel_id).for_each(
			|(vtoken, channel_vtoken_share)| {
				// get the commission_token of the vtoken
				let commission_token_op = CommissionTokens::<T>::get(vtoken);

				if let Some(commission_token) = commission_token_op {
					// get the vtoken issuance amount
					let vtoken_issuance = VtokenIssuanceSnapshots::<T>::get(vtoken).0;
					if vtoken_issuance.is_zero() {
						return;
					}

					// get the total commission amount
					let total_commission = PeriodTotalCommissions::<T>::get(commission_token).0;

					// calculate the channel commission amount
					let raw_channel_commission = channel_vtoken_share.mul_floor(total_commission);

					// get the channel vtoken commission rate
					let mut channel_commission_rate =
						ChannelCommissionTokenRates::<T>::get(channel_id, vtoken);
					if channel_commission_rate.is_zero() {
						channel_commission_rate = DEFAULT_COMMISSION_RATE;
					}

					// calculate the channel commission amount
					let channel_commission =
						channel_commission_rate.mul_floor(raw_channel_commission);

					// update channel_commission to ChannelClaimableCommissions storage
					ChannelClaimableCommissions::<T>::mutate(
						channel_id,
						commission_token,
						|amount| {
							let sum_up = amount.saturating_add(channel_commission);
							*amount = sum_up;

							Self::deposit_event(Event::ChannelClaimableCommissionUpdated {
								channel_id,
								commission_token,
								amount: *amount,
							});
						},
					);

					// add the amount to the PeriodClearedCommissions storage
					PeriodClearedCommissions::<T>::mutate(commission_token, |amount| {
						let sum_up = amount.saturating_add(channel_commission);
						*amount = sum_up;
					});
				}
			},
		);
	}

	pub(crate) fn update_channel_vtoken_shares(channel_id: ChannelId) {
		// for a channel_id，update the share of all vtoken
		for (vtoken, channel_old_share) in ChannelVtokenShares::<T>::iter_prefix(channel_id) {
			// get the vtoken issuance amount
			let (old_vtoken_issuance, new_vtoken_issuance) =
				VtokenIssuanceSnapshots::<T>::get(vtoken);

			// get the total minted amount of the period
			let total_mint = PeriodVtokenTotalMint::<T>::get(vtoken).0;

			// get the total redeemed amount of the period
			let total_redeem = PeriodVtokenTotalRedeem::<T>::get(vtoken).0;

			// only update the share when total_mint > total_redeem
			if total_redeem < total_mint {
				// net_mint = total_mint - total_redeem
				let net_mint = total_mint.saturating_sub(total_redeem);
				// channel mint
				let channel_mint = PeriodChannelVtokenMint::<T>::get(channel_id, vtoken).0;
				// channel_net_mint： channel_share * net_mint
				let channel_period_net_mint = if total_mint.is_zero() {
					Zero::zero()
				} else {
					Self::calculate_mul_div_result(channel_mint, net_mint, total_mint)
						.unwrap_or(Zero::zero())
				};

				let numerator =
					channel_old_share.mul_floor(old_vtoken_issuance) + channel_period_net_mint;
				let denominator = new_vtoken_issuance;

				ChannelVtokenShares::<T>::mutate(channel_id, vtoken, |share| {
					let channel_new_share: Permill = match denominator.is_zero() {
						true => Permill::zero(),
						false => Permill::from_rational_with_rounding(
							numerator,
							denominator,
							Rounding::Down,
						).unwrap_or_else(|()| {
							log::error!("Failed to calculate Permill from numerator: {:?} and denominator: {:?}.",numerator, denominator);
							// Emit the failure event
							Self::deposit_event(Event::CalculationFailed {
								numerator,
								denominator,
							});
							Permill::zero() // Return zero as a fallback
						}),
					};

					*share = channel_new_share;

					Self::deposit_event(Event::ChannelVtokenSharesUpdated {
						channel_id,
						vtoken,
						share: channel_new_share,
					});
				});
			}
		}
	}

	pub(crate) fn clear_bifrost_commissions() {
		// for all CommissionTokens，calculate the commission of Bifrost
		CommissionTokens::<T>::iter_values().for_each(|commission_token| {
			// get the total commission amount
			let total_commission = PeriodTotalCommissions::<T>::get(commission_token).0;

			// get the cleared amount from the PeriodClearedCommissions storage
			let cleared_commission = PeriodClearedCommissions::<T>::get(commission_token);

			// calculate the bifrost commission amount
			let bifrost_commission = total_commission.saturating_sub(cleared_commission);

			if bifrost_commission.is_zero() {
				return;
			}

			// transfer the bifrost commission amount from CommissionPalletId account to the bifrost
			// commission receiver account
			if let Err(_) = T::MultiCurrency::transfer(
				commission_token,
				&Self::account_id(),
				&T::BifrostCommissionReceiver::get(),
				bifrost_commission,
			) {
				log::error!(
					"Failed to transfer bifrost commission for token: {:?}",
					commission_token
				);
				Self::deposit_event(Event::BifrostCommissionTransferFailed {
					from: Self::account_id(),
					to: T::BifrostCommissionReceiver::get(),
					commission_token,
					amount: bifrost_commission,
				});
			}
		});

		// clear PeriodClearedCommissions
		let res = PeriodClearedCommissions::<T>::clear(REMOVE_TOKEN_LIMIT, None);
		let executed_num = res.backend;
		if let Err(_) = Self::check_removed_all(res) {
			log::error!("The removal process was not complete; cursor is still present.");
			Self::deposit_event(Event::RemovalNotCompleteError {
				target_num: PeriodClearedCommissions::<T>::iter().count() as u32,
				limit: REMOVE_TOKEN_LIMIT,
				executed_num,
			});
		}
	}

	pub(crate) fn calculate_mul_div_result(
		multiplier_1: BalanceOf<T>,
		multiplier_2: BalanceOf<T>,
		divider: BalanceOf<T>,
	) -> Result<BalanceOf<T>, Error<T>> {
		if divider.is_zero() {
			return Ok(Zero::zero());
		}

		let result: u128 = multiply_by_rational_with_rounding(
			multiplier_1.saturated_into::<u128>(),
			multiplier_2.saturated_into::<u128>(),
			divider.saturated_into::<u128>(),
			Rounding::Down,
		)
		.ok_or(Error::DivisionByZero)?;

		Ok(BalanceOf::<T>::unique_saturated_from(result))
	}

	pub(crate) fn account_id() -> AccountIdOf<T> {
		T::CommissionPalletId::get().into_account_truncating()
	}

	pub(crate) fn settle_channel_commission(channel_id: ChannelId) -> Result<(), Error<T>> {
		// get channel receive account
		let channel_op = Channels::<T>::get(channel_id);

		let receiver_account = channel_op
			.map(|channel_info| channel_info.0)
			.ok_or(Error::<T>::ChannelNotExist)?;

		// transfer all the claimable commission amount to the channel receive account
		let mut tokens_to_remove = Vec::new();
		for (commission_token, amount) in ChannelClaimableCommissions::<T>::iter_prefix(channel_id)
		{
			T::MultiCurrency::transfer(
				commission_token,
				&Self::account_id(),
				&receiver_account,
				amount,
			)
			.map_err(|_| Error::<T>::TransferError)?;

			// Collect the token for removal after iteration
			tokens_to_remove.push((commission_token, amount));
		}
		// Remove the collected tokens from ChannelClaimableCommissions storage
		for (commission_token, amount) in tokens_to_remove {
			ChannelClaimableCommissions::<T>::remove(channel_id, commission_token);
			Self::deposit_event(Event::CommissionClaimed { channel_id, commission_token, amount });
		}

		Ok(())
	}

	fn check_removed_all(res: MultiRemovalResults) -> Result<(), Error<T>> {
		ensure!(res.maybe_cursor.is_none(), Error::<T>::RemovalNotComplete);
		Ok(())
	}
}

impl<T: Config> VTokenMintRedeemProvider<CurrencyId, BalanceOf<T>> for Pallet<T> {
	fn record_mint_amount(
		channel_id: Option<ChannelId>,
		vtoken: CurrencyId,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		if amount.is_zero() {
			return Ok(());
		}

		// Retrieve and update total mint for the given vtoken in a single step.
		PeriodVtokenTotalMint::<T>::mutate(vtoken, |total_mint| -> Result<(), Error<T>> {
			// Safely add the new amount to the existing total.
			total_mint.1 = total_mint.1.checked_add(&amount).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		// Only non-Bifrost minting needs to record the channel minting amount.
		if let Some(channel_id) = channel_id {
			PeriodChannelVtokenMint::<T>::mutate(
				channel_id,
				vtoken,
				|channel_vtoken_mint| -> Result<(), Error<T>> {
					let sum_up_amount =
						channel_vtoken_mint.1.checked_add(&amount).ok_or(Error::<T>::Overflow)?;

					channel_vtoken_mint.1 = sum_up_amount;
					Ok(())
				},
			)?;
		}

		Ok(())
	}
	// record the redeem amount of vtoken
	fn record_redeem_amount(vtoken: CurrencyId, amount: BalanceOf<T>) -> Result<(), DispatchError> {
		if amount.is_zero() {
			return Ok(());
		}

		// First, add to PeriodVtokenTotalRedeem.
		PeriodVtokenTotalRedeem::<T>::mutate(vtoken, |total_redeem| -> Result<(), Error<T>> {
			total_redeem.1 = total_redeem.1.checked_add(&amount).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}
}

impl<T: Config> SlpHostingFeeProvider<CurrencyId, BalanceOf<T>, AccountIdOf<T>> for Pallet<T> {
	// record the hosting fee
	fn record_hosting_fee(
		staking_token: CurrencyId,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		if amount.is_zero() {
			return Ok(());
		}

		// get the commission token of the staking token
		let vtoken = staking_token.to_vtoken().map_err(|_| Error::<T>::ConversionError)?;

		// If the vtoken is configured for commission, record the hosting fee
		if let Some(commission_token) = CommissionTokens::<T>::get(vtoken) {
			PeriodTotalCommissions::<T>::mutate(
				commission_token,
				|total_commission| -> Result<(), Error<T>> {
					let sum_up_amount =
						total_commission.1.checked_add(&amount).ok_or(Error::<T>::Overflow)?;

					total_commission.1 = sum_up_amount;
					Ok(())
				},
			)?;
		}

		Ok(())
	}
}
