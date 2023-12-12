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

use alloc::vec;
use bifrost_primitives::{CurrencyId, SlpHostingFeeProvider, VTokenMintRedeemProvider};
use frame_support::{pallet_prelude::*, PalletId};
use frame_system::pallet_prelude::*;
use orml_traits::MultiCurrency;
use sp_runtime::{
	traits::{AccountIdConversion, CheckedAdd, Zero},
	Percent,
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

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Currecny operation handler
		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		/// The only origin that can edit token issuer list
		type ControlOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Commission master Pallet Id to get the commission master account
		type CommissionPalletId: Get<PalletId>;

		/// The receiving account of Bifrost commission
		type BifrostCommissionReceiver: Get<AccountIdOf<Self>>;

		// /// The interface to call VtokenMinting module functions.
		// type VtokenMinting: VtokenMintingOperator<
		// 	CurrencyId,
		// 	BalanceOf<Self>,
		// 	AccountIdOf<Self>,
		// 	TimeUnit,
		// >;

		// ///
		// type SlpOperator: SlpOperator<
		//     CurrencyId,
		//     BalanceOf<Self>,
		//     AccountIdOf<Self>,
		//     TimeUnit,
		// >;

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;

		// Commission clearing duration, in blocks
		#[pallet::constant]
		type ClearingDuration: Get<u32>;

		// The maximum bytes length of channel name
		#[pallet::constant]
		type NameLengthLimit: Get<u32>;
	}

	#[pallet::error]
	pub enum Error<T> {
		Overflow,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		ChannelRegistered { channel_id: u32, receive_account: AccountIdOf<T>, name: Vec<u8> },
	}

	/// Auto increment channel id
	#[pallet::storage]
	#[pallet::getter(fn channel_next_id)]
	pub type ChannelNextId<T: Config> = StorageValue<_, ChannelId, ValueQuery>;

	/// Mapping a channel id to a receive account and a name, 【channel_id =>(receive_account,
	/// name)】
	#[pallet::storage]
	#[pallet::getter(fn channels)]
	pub type Channels<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		ChannelId,
		(AccountIdOf<T>, BoundedVec<u8, T::NameLengthLimit>),
	>;

	/// Mapping a staking token to a commission token, 【staking_token => commission_token】
	#[pallet::storage]
	#[pallet::getter(fn commission_tokens)]
	pub type CommissionTokens<T> = StorageMap<_, Blake2_128Concat, CurrencyId, CurrencyId>;

	/// Mapping a channel + a staking token to corresponding commission rate, 【(channel_id,
	/// staking_token) => commission rate】
	#[pallet::storage]
	#[pallet::getter(fn channel_commission_token_rates)]
	pub type ChannelCommissionTokenRates<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ChannelId,
		Blake2_128Concat,
		CurrencyId,
		Percent,
		OptionQuery,
	>;

	/// Mapping a channel + vtoken to corresponding channel share, 【(channel_id, vtoken) => share】
	#[pallet::storage]
	#[pallet::getter(fn channel_vtoken_shares)]
	pub type ChannelVtokenShares<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ChannelId,
		Blake2_128Concat,
		CurrencyId,
		BalanceOf<T>,
		OptionQuery,
	>;

	/// 【vtoken => (old_issuance, new_issuance)】,old_issuance is the vtoken issuance at last
	/// clearing point,  new_issuance is the ongoing accumulative issuance the last clearing point
	#[pallet::storage]
	#[pallet::getter(fn vtoken_issuance_snapshots)]
	pub type VtokenIssuanceSnapshots<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, (BalanceOf<T>, BalanceOf<T>)>;

	/// Vtoken total minted amount in the ongoing period for the chain, 【vtoken => (old_total_mint,
	/// new_total_mint)】
	#[pallet::storage]
	#[pallet::getter(fn period_vtoken_total_mint)]
	pub type PeriodVtokenTotalMint<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, (BalanceOf<T>, BalanceOf<T>)>;

	/// Vtoken total redeemed amount in the ongoing period for the chain, 【vtoken =>
	/// (old_total_redeem, new_total_redeem)】
	#[pallet::storage]
	#[pallet::getter(fn period_vtoken_total_redeem)]
	pub type PeriodVtokenTotalRedeem<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, (BalanceOf<T>, BalanceOf<T>)>;

	/// Vtoken minted amount in the ongoing period for the channel, 【(channel_id, vtoken) =>
	/// (old_mint_amount, new_mint_amount)】
	#[pallet::storage]
	#[pallet::getter(fn period_channel_vtoken_mint)]
	pub type PeriodChannelVtokenMint<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ChannelId,
		Blake2_128Concat,
		CurrencyId,
		(BalanceOf<T>, BalanceOf<T>),
		OptionQuery,
	>;

	/// Commission pool for last period and ongoing period, 【commission token => (old_amount,
	/// new_amount)】
	#[pallet::storage]
	#[pallet::getter(fn period_total_commissions)]
	pub type PeriodTotalCommissions<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, (BalanceOf<T>, BalanceOf<T>)>;

	/// Commission amount that has been cleared for the current clearing process, 【commission token
	/// => amount】
	#[pallet::storage]
	#[pallet::getter(fn period_cleared_commissions)]
	pub type PeriodClearedCommissions<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, BalanceOf<T>>;

	/// Commission amount to be claimed by channels, 【channel id + commission token => amount】
	#[pallet::storage]
	#[pallet::getter(fn channel_claimable_commissions)]
	pub type ChannelClaimableCommissions<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ChannelId,
		Blake2_128Concat,
		CurrencyId,
		BalanceOf<T>,
		OptionQuery,
	>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	// #[pallet::call]
	// impl<T: Config> Pallet<T> {
	// 	#[pallet::call_index(0)]
	// 	#[pallet::weight(T::WeightInfo::cross_in())]
	// 	pub fn cross_in(
	// 		origin: OriginFor<T>,
	// 		location: Box<MultiLocation>,
	// 		currency_id: CurrencyId,
	// 		#[pallet::compact] amount: BalanceOf<T>,
	// 		remark: Option<Vec<u8>>,
	// 	) -> DispatchResult {
	// 		let issuer = ensure_signed(origin)?;

	// 		ensure!(
	// 			CrossCurrencyRegistry::<T>::contains_key(currency_id),
	// 			Error::<T>::CurrencyNotSupportCrossInAndOut
	// 		);

	// 		let crossing_minimum_amount = Self::get_crossing_minimum_amount(currency_id)
	// 			.ok_or(Error::<T>::NoCrossingMinimumSet)?;
	// 		ensure!(amount >= crossing_minimum_amount.0, Error::<T>::AmountLowerThanMinimum);

	// 		let issue_whitelist =
	// 			Self::get_issue_whitelist(currency_id).ok_or(Error::<T>::NotAllowed)?;
	// 		ensure!(issue_whitelist.contains(&issuer), Error::<T>::NotAllowed);

	// 		let entrance_account_mutlilcaition = Box::new(MultiLocation {
	// 			parents: 0,
	// 			interior: X1(AccountId32 {
	// 				network: Any,
	// 				id: T::EntrancePalletId::get().into_account_truncating(),
	// 			}),
	// 		});

	// 		// If the cross_in destination is entrance account, it is not required to be registered.
	// 		let dest = if entrance_account_mutlilcaition == location {
	// 			T::EntrancePalletId::get().into_account_truncating()
	// 		} else {
	// 			Self::outer_multilocation_to_account(currency_id, location.clone())
	// 				.ok_or(Error::<T>::NoAccountIdMapping)?
	// 		};

	// 		T::MultiCurrency::deposit(currency_id, &dest, amount)?;

	// 		Self::deposit_event(Event::CrossedIn {
	// 			dest,
	// 			currency_id,
	// 			location: *location,
	// 			amount,
	// 			remark,
	// 		});
	// 		Ok(())
	// 	}
	// }
}

impl<T: Config> VTokenMintRedeemProvider<CurrencyId, BalanceOf<T>> for Pallet<T> {
	fn record_mint_amount(
		channel_id: ChannelId,
		vtoken: CurrencyId,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		if amount == Zero::zero() {
			return Ok(());
		}

		// first add to PeriodVtokenTotalMint (加到周期系统总毛铸造量里)
		let mut total_mint =
			PeriodVtokenTotalMint::<T>::get(vtoken).unwrap_or((Zero::zero(), Zero::zero()));
		let sum_up_amount = total_mint.1.checked_add(&amount).ok_or(Error::<T>::Overflow)?;

		total_mint.1 = sum_up_amount;
		PeriodVtokenTotalMint::<T>::insert(vtoken, total_mint);

		// then add to PeriodChannelVtokenMint (加到周期渠道的毛铸造量里)
		let mut channel_vtoken_mint = PeriodChannelVtokenMint::<T>::get(channel_id, vtoken)
			.unwrap_or((Zero::zero(), Zero::zero()));
		let sum_up_amount =
			channel_vtoken_mint.1.checked_add(&amount).ok_or(Error::<T>::Overflow)?;

		channel_vtoken_mint.1 = sum_up_amount;
		PeriodChannelVtokenMint::<T>::insert(channel_id, vtoken, channel_vtoken_mint);

		Ok(())
	}
	// record the redeem amount of vtoken
	fn record_redeem_amount(vtoken: CurrencyId, amount: BalanceOf<T>) -> Result<(), DispatchError> {
		if amount == Zero::zero() {
			return Ok(());
		}

		// first add to PeriodVtokenTotalRedeem (加到周期系统总毛赎回量里)
		let mut total_redeem =
			PeriodVtokenTotalRedeem::<T>::get(vtoken).unwrap_or((Zero::zero(), Zero::zero()));
		let sum_up_amount = total_redeem.1.checked_add(&amount).ok_or(Error::<T>::Overflow)?;

		total_redeem.1 = sum_up_amount;
		PeriodVtokenTotalRedeem::<T>::insert(vtoken, total_redeem);

		Ok(())
	}
}

impl<T: Config> SlpHostingFeeProvider<CurrencyId, BalanceOf<T>, AccountIdOf<T>> for Pallet<T> {
	// transfer the hosting fee to the receiver account
	fn collect_hosting_fee(
		from: AccountIdOf<T>,
		commission_token: CurrencyId,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		if amount == Zero::zero() {
			return Ok(());
		}

		// get the receiver account from CommissionPalletId
		let receiver_account = T::CommissionPalletId::get().into_account_truncating();

		// transfer the hosting fee to the receiver account
		T::MultiCurrency::transfer(commission_token, &from, &receiver_account, amount)?;

		Ok(())
	}
	// record the hosting fee
	fn record_hosting_fee(
		commission_token: CurrencyId,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		if amount == Zero::zero() {
			return Ok(());
		}

		// add to PeriodTotalCommissions (加到周期系统总佣金里)
		let mut total_commission = PeriodTotalCommissions::<T>::get(commission_token)
			.unwrap_or((Zero::zero(), Zero::zero()));

		let sum_up_amount = total_commission.1.checked_add(&amount).ok_or(Error::<T>::Overflow)?;

		total_commission.1 = sum_up_amount;
		PeriodTotalCommissions::<T>::insert(commission_token, total_commission);

		Ok(())
	}
}
