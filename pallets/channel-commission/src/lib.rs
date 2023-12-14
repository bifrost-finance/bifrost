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
use sp_core::U256;
use sp_runtime::{
	traits::{
		AccountIdConversion, CheckedAdd, CheckedSub, UniqueSaturatedFrom, UniqueSaturatedInto, Zero,
	},
	PerThing, Percent, Permill, Rounding, SaturatedConversion,
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
			rate: Option<u8>,
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
		PeriodTotalComissionsUpdated {
			commission_token: CurrencyId,
			old_amount: BalanceOf<T>,
			new_amount: BalanceOf<T>,
		},
		ChannelClaimableComissionUpdated {
			channel_id: ChannelId,
			commission_token: CurrencyId,
			amount: Option<BalanceOf<T>>,
		},
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

	/// Mapping a vtoken to a commission token, 【vtoken => commission_token】
	#[pallet::storage]
	#[pallet::getter(fn commission_tokens)]
	pub type CommissionTokens<T> = StorageMap<_, Blake2_128Concat, CurrencyId, CurrencyId>;

	/// Mapping a channel + vtoken to corresponding commission rate, 【(channel_id, vtoken) =>
	/// commission rate】
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
		Permill,
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
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let channel_count: u32 = ChannelNextId::<T>::get().into();

			// If the current block number is the first block of a new clearing period, we need to
			// prepare data for clearing.
			if n % T::ClearingDuration::get() == Zero::zero() {
				Self::set_clearing_environment();
			} else if (n % T::ClearingDuration::get()) < (channel_count + 1).into() {
				let channel_index = n % T::ClearingDuration::get().into() - 1u32.into();
				let channel_id: ChannelId =
					BlockNumberFor::<T>::unique_saturated_into(channel_index);
				Self::clear_channel_commissions(channel_id);
				Self::update_channel_vtoken_shares(channel_id);
			} else if (n % T::ClearingDuration::get()) == (channel_count + 1).into() {
				Self::clear_bifrost_commissions();
			}

			T::WeightInfo::on_initialize()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::register_channel())]
		pub fn register_channel(
			origin: OriginFor<T>,
			channel_name: Vec<u8>,
			receive_account: AccountIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			ensure!(
				channel_name.len() <= T::NameLengthLimit::get() as usize,
				Error::<T>::ChannelNameTooLong
			);

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
			let _ = ChannelCommissionTokenRates::<T>::clear_prefix(
				channel_id,
				REMOVE_TOKEN_LIMIT,
				None,
			);

			// remove the channel from ChannelVtokenShares storage
			let _ = ChannelVtokenShares::<T>::clear_prefix(channel_id, REMOVE_TOKEN_LIMIT, None);

			// remove the channel from PeriodChannelVtokenMint storage
			let _ =
				PeriodChannelVtokenMint::<T>::clear_prefix(channel_id, REMOVE_TOKEN_LIMIT, None);

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

			// if the receive account is not changed, do nothing
			let channel_op = Channels::<T>::get(channel_id);

			let old_receive_account = if let Some(channel_info) = channel_op {
				channel_info.0
			} else {
				Err(Error::<T>::ChannelNotExist)?
			};

			if old_receive_account == receive_account {
				return Ok(());
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
			rate: Option<u8>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			// check if the channel exists
			ensure!(Channels::<T>::contains_key(channel_id), Error::<T>::ChannelNotExist);
			// check if the vtoken exists
			ensure!(
				CommissionTokens::<T>::contains_key(vtoken),
				Error::<T>::VtokenNotConfiguredForCommission
			);
			// check if the rate is valid
			if let Some(rate) = rate {
				ensure!(rate <= 100, Error::<T>::InvalidCommissionRate);
			}

			// if rate is None, remove the channel commission rate, if rate is Some, update the
			// channel commission rate
			if let Some(rate) = rate {
				let rate = Percent::from_percent(rate);
				ChannelCommissionTokenRates::<T>::insert(channel_id, vtoken, rate);
			} else {
				ChannelCommissionTokenRates::<T>::remove(channel_id, vtoken);
			}

			Self::deposit_event(Event::ChannelCommissionSet { channel_id, vtoken, rate });

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::set_commission_tokens())]
		pub fn set_commission_tokens(
			origin: OriginFor<T>,
			vtoken: CurrencyId,
			commission_token: CurrencyId,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			// if old commission token is the same as the new one, do nothing
			if let Some(old_commission_token) = CommissionTokens::<T>::get(vtoken) {
				if old_commission_token == commission_token {
					return Ok(());
				}
			}

			// set the commission token
			CommissionTokens::<T>::insert(vtoken, commission_token);

			Self::deposit_event(Event::CommissionTokenSet {
				vtoken,
				commission_token: Some(commission_token),
			});

			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::claim_commissions())]
		pub fn claim_commissions(origin: OriginFor<T>, channel_id: ChannelId) -> DispatchResult {
			ensure_signed(origin)?;

			Self::settle_channel_commission(channel_id)?;
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub(crate) fn set_clearing_environment() {
		//  Move the vtoken issuance amount from ongoing period to the previous period and clear the
		// ongoing period issuance amount
		VtokenIssuanceSnapshots::<T>::iter().for_each(|(vtoken, issuance)| {
			let mut issuance = issuance;
			issuance.0 = issuance.1;

			// get the vtoken new issuance amount from Tokens module issuance storage
			let new_issuance = T::MultiCurrency::total_issuance(vtoken);
			issuance.1 = new_issuance;
			VtokenIssuanceSnapshots::<T>::insert(vtoken, issuance);

			Self::deposit_event(Event::VtokenIssuanceSnapshotUpdated {
				vtoken,
				old_issuance: issuance.0,
				new_issuance,
			});
		});

		// Move the total minted amount of the period from ongoing period to the previous period and
		// clear the ongoing period minted amount
		PeriodVtokenTotalMint::<T>::iter().for_each(|(vtoken, total_mint)| {
			let info = total_mint.clone();

			let mut total_mint = total_mint;
			total_mint.0 = total_mint.1;
			total_mint.1 = Zero::zero();
			PeriodVtokenTotalMint::<T>::insert(vtoken, total_mint);

			Self::deposit_event(Event::PeriodVtokenTotalMintUpdated {
				vtoken,
				old_total_mint: info.0,
				new_total_mint: info.1,
			});
		});

		// Move the total redeemed amount of the period from ongoing period to the previous period
		// and clear the ongoing period redeemed amount
		PeriodVtokenTotalRedeem::<T>::iter().for_each(|(vtoken, total_redeem)| {
			let info = total_redeem.clone();

			let mut total_redeem = total_redeem;
			total_redeem.0 = total_redeem.1;
			total_redeem.1 = Zero::zero();
			PeriodVtokenTotalRedeem::<T>::insert(vtoken, total_redeem);

			Self::deposit_event(Event::PeriodVtokenTotalRedeemUpdated {
				vtoken,
				old_total_redeem: info.0,
				new_total_redeem: info.1,
			});
		});

		// Move the channel minted amount of the period from ongoing period to the previous period
		// and clear the ongoing period minted amount
		PeriodChannelVtokenMint::<T>::iter().for_each(
			|(channel_id, vtoken, channel_vtoken_mint)| {
				let info = channel_vtoken_mint.clone();

				let mut channel_vtoken_mint = channel_vtoken_mint;
				channel_vtoken_mint.0 = channel_vtoken_mint.1;
				channel_vtoken_mint.1 = Zero::zero();
				PeriodChannelVtokenMint::<T>::insert(channel_id, vtoken, channel_vtoken_mint);

				Self::deposit_event(Event::PeriodChannelVtokenMintUpdated {
					channel_id,
					vtoken,
					old_mint_amount: info.0,
					new_mint_amount: info.1,
				});
			},
		);

		// Move the total commission amount of the period from ongoing period to the previous period
		// and clear the ongoing period commission amount
		PeriodTotalCommissions::<T>::iter().for_each(|(commission_token, total_commission)| {
			let info = total_commission.clone();

			let mut total_commission = total_commission;
			total_commission.0 = total_commission.1;
			total_commission.1 = Zero::zero();
			PeriodTotalCommissions::<T>::insert(commission_token, total_commission);

			Self::deposit_event(Event::PeriodTotalComissionsUpdated {
				commission_token,
				old_amount: info.0,
				new_amount: info.1,
			});
		});
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
				if commission_token_op.is_none() {
					return;
				}
				let commission_token = commission_token_op.unwrap();

				// get the vtoken issuance amount
				let vtoken_issuance = VtokenIssuanceSnapshots::<T>::get(vtoken)
					.unwrap_or((Zero::zero(), Zero::zero()))
					.0;
				if vtoken_issuance == Zero::zero() {
					return;
				}

				// get the total commission amount
				let total_commission = PeriodTotalCommissions::<T>::get(commission_token)
					.unwrap_or((Zero::zero(), Zero::zero()))
					.0;

				// calculate the channel commission amount
				let channel_commission = channel_vtoken_share.mul_floor(total_commission);

				// update channel_commission to ChannelClaimableCommissions storage
				ChannelClaimableCommissions::<T>::mutate(
					channel_id,
					commission_token,
					|amount_op| {
						if let Some(amount) = amount_op {
							let sum_up_op = amount.checked_add(&channel_commission);

							if let Some(sum_up) = sum_up_op {
								*amount = sum_up;
							}
						} else {
							*amount_op = Some(channel_commission);
						}

						Self::deposit_event(Event::ChannelClaimableComissionUpdated {
							channel_id,
							commission_token,
							amount: *amount_op,
						});
					},
				);

				// add the amount to the PeriodClearedCommissions storage
				PeriodClearedCommissions::<T>::mutate(commission_token, |amount_op| {
					if let Some(amount) = amount_op {
						let sum_up_op = amount.checked_add(&channel_commission);

						if let Some(sum_up) = sum_up_op {
							*amount = sum_up;
						}
					} else {
						*amount_op = Some(channel_commission);
					}
				});
			},
		);
	}

	pub(crate) fn update_channel_vtoken_shares(channel_id: ChannelId) {
		// 对于同一个渠道，更新它的所有 vtoken 的 share 占比
		ChannelVtokenShares::<T>::iter_prefix(channel_id).for_each(
			|(vtoken, channel_old_share)| {
				// get the vtoken issuance amount
				let (old_vtoken_issuance, new_vtoken_issuance) =
					VtokenIssuanceSnapshots::<T>::get(vtoken)
						.unwrap_or((Zero::zero(), Zero::zero()));

				// get the total minted amount of the period
				let total_mint = PeriodVtokenTotalMint::<T>::get(vtoken)
					.unwrap_or((Zero::zero(), Zero::zero()))
					.1;

				// get the total redeemed amount of the period
				let total_redeem = PeriodVtokenTotalRedeem::<T>::get(vtoken)
					.unwrap_or((Zero::zero(), Zero::zero()))
					.1;

				// only update the share when total_mint > total_redeem
				if total_redeem < total_mint {
					// 当前周期有效铸造量 = 当前周期总铸造量 - 当前周期总赎回量
					let net_mint = total_mint.checked_sub(&total_redeem).unwrap_or(Zero::zero());
					// 渠道当期毛铸造量
					let channel_mint = PeriodChannelVtokenMint::<T>::get(channel_id, vtoken)
						.unwrap_or((Zero::zero(), Zero::zero()))
						.1;
					// 计算渠道 A 当前周期的新增有效铸造量： 渠道 A 当前周期的 share *
					// 当前周期协议的净铸造量
					let channel_period_net_mint = if total_mint == Zero::zero() {
						Zero::zero()
					} else {
						Self::calculate_mul_div_result(channel_mint, net_mint, total_mint)
					};

					let numerator =
						channel_old_share.mul_floor(old_vtoken_issuance) + channel_period_net_mint;
					let denominator = new_vtoken_issuance;
					let channel_new_share: Permill = if denominator == Zero::zero() {
						Zero::zero()
					} else {
						Permill::from_rational_with_rounding(numerator, denominator, Rounding::Down)
							.unwrap_or(Zero::zero())
					};

					// update the share to ChannelVtokenShares storage
					ChannelVtokenShares::<T>::insert(channel_id, vtoken, channel_new_share);

					Self::deposit_event(Event::ChannelVtokenSharesUpdated {
						channel_id,
						vtoken,
						share: channel_new_share,
					});
				}
			},
		);
	}

	pub(crate) fn clear_bifrost_commissions() {
		// 对于所有的CommissionTokens，计算Bifrost的佣金金额
		CommissionTokens::<T>::iter_values().for_each(|commission_token| {
			// get the total commission amount
			let total_commission = PeriodTotalCommissions::<T>::get(commission_token)
				.unwrap_or((Zero::zero(), Zero::zero()))
				.0;

			// get the cleared amount from the PeriodClearedCommissions storage
			let cleared_commission =
				PeriodClearedCommissions::<T>::get(commission_token).unwrap_or(Zero::zero());

			// calculate the bifrost commission amount
			let bifrost_commission =
				total_commission.checked_sub(&cleared_commission).unwrap_or(Zero::zero());

			if bifrost_commission == Zero::zero() {
				return;
			}

			// transfer the bifrost commission amount from CommissionPalletId account to the bifrost
			// commission receiver account
			let _ = T::MultiCurrency::transfer(
				commission_token,
				&Self::account_id(),
				&T::BifrostCommissionReceiver::get(),
				bifrost_commission,
			);
		});

		// clear PeriodClearedComissions
		let _ = PeriodClearedCommissions::<T>::clear(REMOVE_TOKEN_LIMIT, None);
	}

	pub(crate) fn calculate_mul_div_result(
		multiplier_1: BalanceOf<T>,
		multiplier_2: BalanceOf<T>,
		divider: BalanceOf<T>,
	) -> BalanceOf<T> {
		if divider == Zero::zero() {
			return Zero::zero();
		}

		let result: u128 = U256::from(multiplier_1.saturated_into::<u128>())
			.saturating_mul(multiplier_2.saturated_into::<u128>().into())
			.checked_div(divider.saturated_into::<u128>().into())
			.map(|x| u128::try_from(x).unwrap_or(Zero::zero()))
			.unwrap_or(Zero::zero());

		BalanceOf::<T>::unique_saturated_from(result)
	}

	pub(crate) fn account_id() -> AccountIdOf<T> {
		T::CommissionPalletId::get().into_account_truncating()
	}

	pub(crate) fn settle_channel_commission(channel_id: ChannelId) -> Result<(), Error<T>> {
		// get channel receive account
		let channel_op = Channels::<T>::get(channel_id);

		let receiver_account = if let Some(channel_info) = channel_op {
			channel_info.0
		} else {
			Err(Error::<T>::ChannelNotExist)?
		};

		// transfer all the claimable commission amount to the channel receive account
		for (commission_token, amount) in ChannelClaimableCommissions::<T>::iter_prefix(channel_id)
		{
			T::MultiCurrency::transfer(
				commission_token,
				&Self::account_id(),
				&receiver_account,
				amount,
			)
			.map_err(|_| Error::<T>::TransferError)?;

			// remove the commission amount from ChannelClaimableCommissions storage
			ChannelClaimableCommissions::<T>::remove(channel_id, commission_token);

			Self::deposit_event(Event::CommissionClaimed { channel_id, commission_token, amount });
		}

		Ok(())
	}
}

impl<T: Config> VTokenMintRedeemProvider<CurrencyId, BalanceOf<T>> for Pallet<T> {
	fn record_mint_amount(
		channel_id: Option<ChannelId>,
		vtoken: CurrencyId,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		if amount == Zero::zero() {
			return Ok(());
		}

		// first add to PeriodVtokenTotalMint
		let mut total_mint =
			PeriodVtokenTotalMint::<T>::get(vtoken).unwrap_or((Zero::zero(), Zero::zero()));
		let sum_up_amount = total_mint.1.checked_add(&amount).ok_or(Error::<T>::Overflow)?;

		total_mint.1 = sum_up_amount;
		PeriodVtokenTotalMint::<T>::insert(vtoken, total_mint);

		// only non-bifrost minting needs to record the channel minting amount
		if let Some(channel_id) = channel_id {
			// then add to PeriodChannelVtokenMint
			let mut channel_vtoken_mint = PeriodChannelVtokenMint::<T>::get(channel_id, vtoken)
				.unwrap_or((Zero::zero(), Zero::zero()));
			let sum_up_amount =
				channel_vtoken_mint.1.checked_add(&amount).ok_or(Error::<T>::Overflow)?;

			channel_vtoken_mint.1 = sum_up_amount;
			PeriodChannelVtokenMint::<T>::insert(channel_id, vtoken, channel_vtoken_mint);
		}

		Ok(())
	}
	// record the redeem amount of vtoken
	fn record_redeem_amount(vtoken: CurrencyId, amount: BalanceOf<T>) -> Result<(), DispatchError> {
		if amount == Zero::zero() {
			return Ok(());
		}

		// first add to PeriodVtokenTotalRedeem
		let mut total_redeem =
			PeriodVtokenTotalRedeem::<T>::get(vtoken).unwrap_or((Zero::zero(), Zero::zero()));
		let sum_up_amount = total_redeem.1.checked_add(&amount).ok_or(Error::<T>::Overflow)?;

		total_redeem.1 = sum_up_amount;
		PeriodVtokenTotalRedeem::<T>::insert(vtoken, total_redeem);

		Ok(())
	}
}

impl<T: Config> SlpHostingFeeProvider<CurrencyId, BalanceOf<T>, AccountIdOf<T>> for Pallet<T> {
	// record the hosting fee
	fn record_hosting_fee(
		staking_token: CurrencyId,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		if amount == Zero::zero() {
			return Ok(());
		}

		// get the commission token of the staking token
		let vtoken = staking_token.to_vtoken()?;
		let commission_token = CommissionTokens::<T>::get(vtoken)?;

		// add to PeriodTotalCommissions (加到周期系统总佣金里)
		let mut total_commission = PeriodTotalCommissions::<T>::get(commission_token)
			.unwrap_or((Zero::zero(), Zero::zero()));

		let sum_up_amount = total_commission.1.checked_add(&amount).ok_or(Error::<T>::Overflow)?;

		total_commission.1 = sum_up_amount;
		PeriodTotalCommissions::<T>::insert(commission_token, total_commission);

		Ok(())
	}
}
