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
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(dead_code)]
#![allow(unused_variables)]
pub mod types;
pub mod weights;

use frame_support::PalletId;
pub use frame_support::{inherent::Vec, pallet_prelude::Weight};
use node_primitives::{CurrencyId, FarmingInfo, PoolId, VtokenMintingInterface};
use orml_traits::MultiCurrency;
pub use pallet::*;
use sp_runtime::traits::AccountIdConversion;
pub use types::*;
pub use weights::WeightInfo;
pub use RoundIndex;

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

#[allow(type_alias_bounds)]
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[allow(type_alias_bounds)]
pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

#[allow(type_alias_bounds)]
pub type BalanceOf<T: Config> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::{RoundInfo, TokenInfo};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_arithmetic::Permill;

	pub type RoundIndex = u32;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type EnsureConfirmAsGovernance: EnsureOrigin<<Self as frame_system::Config>::Origin>;

		type WeightInfo: WeightInfo;

		/// The interface to call Farming module functions.
		type FarmingInfo: FarmingInfo<BalanceOf<Self>, CurrencyIdOf<Self>>;

		/// The interface to call VtokenMinting module functions.
		type VtokenMintingInterface: VtokenMintingInterface<
			AccountIdOf<Self>,
			CurrencyIdOf<Self>,
			BalanceOf<Self>,
		>;

		#[pallet::constant]
		type TreasuryAccount: Get<Self::AccountId>;

		/// ModuleID for creating sub account
		#[pallet::constant]
		type PalletId: Get<PalletId>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::storage]
	#[pallet::getter(fn round)]
	/// Currend Round Information
	pub(crate) type Round<T: Config> = StorageValue<_, RoundInfo<T::BlockNumber>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_status)]
	pub(crate) type TokenStatus<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, TokenInfo<BalanceOf<T>>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_list)]
	pub(crate) type TokenList<T: Config> = StorageValue<_, Vec<CurrencyIdOf<T>>, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		TokenConfigChanged {
			token: CurrencyIdOf<T>,
			exec_delay: u32,
			system_stakable_farming_rate: Permill,
			add_or_sub: bool,
			system_stakable_base: BalanceOf<T>,
			farming_poolids: Vec<PoolId>,
		},
		TokenInfoRefreshed {
			token: CurrencyIdOf<T>,
		},
		Payout {
			token: CurrencyIdOf<T>,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Invalid token config params
		InvalidTokenConfig,
		/// Token info not found
		TokenInfoNotFound,
		/// payout error
		PaymentFailed,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: T::BlockNumber) -> Weight {
			// get token list
			let token_list = Self::token_list();
			let mut x = 0u32;
			let mut y = 0u32;

			let mut round = <Round<T>>::get();
			// new round start
			if round.should_update(n) {
				// mutate round
				round.update(n);

				// update new token configs
				for i in &token_list {
					if let Some(mut token_info) = Self::token_status(*i) {
						x += 1;
						if token_info.check_config_change() {
							token_info.update_config();
						}
					}
				}
			}

			let pallet_account: AccountIdOf<T> = T::PalletId::get().into_account();
			for i in &token_list {
				if let Some(token_info) = Self::token_status(*i) {
					if round.check_delay(n, token_info.current_config.exec_delay) {
						y += 1;
						Self::process_token_info(pallet_account.clone(), token_info, *i);
					}
				}
			}

			T::WeightInfo::on_initialize(x, y)
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// token config，take effect when next round begins
		#[pallet::weight(<T as Config>::WeightInfo::token_config())]
		pub fn token_config(
			origin: OriginFor<T>,
			token: CurrencyIdOf<T>,
			exec_delay: u32,
			system_stakable_farming_rate: Permill,
			add_or_sub: bool,
			system_stakable_base: BalanceOf<T>,
			farming_poolids: Vec<PoolId>,
		) -> DispatchResultWithPostInfo {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?; // Motion
			ensure!(
				exec_delay != 0 &&
					system_stakable_farming_rate > Permill::zero() &&
					!farming_poolids.is_empty(),
				Error::<T>::InvalidTokenConfig
			);
			let mut token_info = if let Some(state) = <TokenStatus<T>>::get(&token) {
				state
			} else {
				<TokenInfo<BalanceOf<T>>>::default()
			};

			token_info.new_config.exec_delay = exec_delay;
			token_info.new_config.system_stakable_farming_rate = system_stakable_farming_rate;
			token_info.new_config.system_stakable_base = system_stakable_base;
			token_info.new_config.add_or_sub = add_or_sub;
			token_info.new_config.farming_poolids = farming_poolids.clone();

			<TokenStatus<T>>::insert(&token, token_info);

			Self::deposit_event(Event::TokenConfigChanged {
				token,
				exec_delay,
				system_stakable_farming_rate,
				add_or_sub,
				system_stakable_base,
				farming_poolids,
			});

			Ok(().into())
		}

		/// refresh token info，query farming pallet, and update TokenInfo, change to new
		/// config，ignore exec_delay, execute immediately
		#[pallet::weight(<T as Config>::WeightInfo::refresh_token_info())]
		pub fn refresh_token_info(
			origin: OriginFor<T>,
			token: CurrencyIdOf<T>,
		) -> DispatchResultWithPostInfo {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?; // Motion
													  //todo: switch to new config
			let token_info = <TokenStatus<T>>::get(&token).ok_or(Error::<T>::TokenInfoNotFound)?;

			let pallet_account: AccountIdOf<T> = T::PalletId::get().into_account();
			Pallet::<T>::process_token_info(pallet_account, token_info, token);

			Self::deposit_event(Event::TokenInfoRefreshed { token });

			Ok(().into())
		}

		/// payout to treasury
		#[pallet::weight(<T as Config>::WeightInfo::payout())]
		pub fn payout(origin: OriginFor<T>, token: CurrencyIdOf<T>) -> DispatchResultWithPostInfo {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?; // Motion

			let token_info = <TokenStatus<T>>::get(&token).ok_or(Error::<T>::TokenInfoNotFound)?;

			// todo staking token + not redeem success amount

			let amount = token_info.system_shadow_amount;
			T::MultiCurrency::deposit(token, &T::TreasuryAccount::get(), amount).ok();

			Self::deposit_event(Event::Payout { token });

			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn process_token_info(
		account: AccountIdOf<T>,
		mut token_info: TokenInfo<BalanceOf<T>>,
		token_id: CurrencyIdOf<T>,
	) {
		// query farming info
		for m in &token_info.current_config.farming_poolids {
			token_info.system_stakable_amount += T::FarmingInfo::get_token_shares(*m, token_id);
		}
		// check amount, and call vtoken minting pallet
		// todo:
		if token_info
			.current_config
			.system_stakable_farming_rate
			.mul_floor(token_info.system_stakable_amount) +
			token_info.current_config.system_stakable_base >
			(token_info.system_shadow_amount - token_info.pending_redeem_amount)
		{
			let mint_amount = token_info.system_stakable_amount +
				token_info.current_config.system_stakable_base -
				(token_info.system_shadow_amount - token_info.pending_redeem_amount);
			// todo: deposit
			match T::VtokenMintingInterface::mint(account, token_id, mint_amount) {
				Ok(_) => {
					token_info.system_shadow_amount += mint_amount;
				},
				Err(error) => {
					log::warn!("mint error: {:?}", error);
				},
			}
		} else if token_info.system_stakable_amount + token_info.current_config.system_stakable_base <
			(token_info.system_shadow_amount - token_info.pending_redeem_amount)
		{
			let redeem_amount = (token_info.system_shadow_amount -
				token_info.pending_redeem_amount) -
				(token_info.system_stakable_amount +
					token_info.current_config.system_stakable_base);
			token_info.pending_redeem_amount += redeem_amount;
			// todo: redeem_amount to vtoken amount
			T::VtokenMintingInterface::redeem(account, token_id, redeem_amount).ok();
		}

		<TokenStatus<T>>::insert(&token_id, token_info);
	}

	pub fn on_redeem_success(
		token_id: CurrencyIdOf<T>,
		to: AccountIdOf<T>,
		token_amount: BalanceOf<T>,
	) -> Weight {
		let mut token_info = if let Some(state) = <TokenStatus<T>>::get(&token_id) {
			state
		} else {
			<TokenInfo<BalanceOf<T>>>::default()
		};

		token_info.system_shadow_amount -= token_amount;
		token_info.pending_redeem_amount -= token_amount;
		// todo withdraw 销毁。
		<TokenStatus<T>>::insert(&token_id, token_info);
		T::WeightInfo::on_redeem_success()
	}

	pub fn on_refund(
		token_id: CurrencyIdOf<T>,
		to: AccountIdOf<T>,
		token_amount: BalanceOf<T>,
	) -> Weight {
		Self::on_redeem_success(token_id, to, token_amount)
	}
}
