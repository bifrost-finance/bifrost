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
pub use weights::WeightInfo;

pub use frame_support::weights::Weight;
use frame_support::{inherent::Vec, traits::Get, PalletId};
use node_primitives::{CurrencyId, FarmingInfo, PoolId, VtokenMintingInterface};
use orml_traits::MultiCurrency;
pub use pallet::*;
use sp_runtime::traits::{AccountIdConversion, Saturating, Zero};
pub use types::*;
pub use RoundIndex;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

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
	use frame_support::{
		pallet_prelude::*,
		sp_runtime::{Perbill, Permill},
	};
	use frame_system::pallet_prelude::*;

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

		/// Max token length
		#[pallet::constant]
		type MaxTokenLen: Get<u32>;

		/// Max farming poolid length
		#[pallet::constant]
		type MaxFarmingPoolIdLen: Get<u32>;

		/// ModuleID for creating sub account
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		#[pallet::constant]
		type BlocksPerRound: Get<u32>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::storage]
	#[pallet::getter(fn round)]
	/// Currend Round Information
	pub(crate) type Round<T: Config> = StorageValue<_, RoundInfo<T::BlockNumber>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_status)]
	pub(crate) type TokenStatus<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, TokenInfo<BalanceOf<T>>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_list)]
	pub(crate) type TokenList<T: Config> =
		StorageValue<_, BoundedVec<CurrencyIdOf<T>, T::MaxTokenLen>, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		NewRound {
			current: RoundIndex,
			first: T::BlockNumber,
			length: u32,
		},
		TokenConfigChanged {
			token: CurrencyIdOf<T>,
			exec_delay: u32,
			system_stakable_farming_rate: Permill,
			add_or_sub: bool,
			system_stakable_base: BalanceOf<T>,
			farming_poolids: Vec<PoolId>,
			lptoken_rates: Vec<Perbill>,
		},
		DepositFailed {
			token: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
			farming_staking_amount: BalanceOf<T>,
			system_stakable_amount: BalanceOf<T>,
			system_shadow_amount: BalanceOf<T>,
			pending_redeem_amount: BalanceOf<T>,
		},
		MintSuccess {
			token: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
			farming_staking_amount: BalanceOf<T>,
			system_stakable_amount: BalanceOf<T>,
			system_shadow_amount: BalanceOf<T>,
			pending_redeem_amount: BalanceOf<T>,
		},
		MintFailed {
			token: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
			farming_staking_amount: BalanceOf<T>,
			system_stakable_amount: BalanceOf<T>,
			system_shadow_amount: BalanceOf<T>,
			pending_redeem_amount: BalanceOf<T>,
		},
		WithdrawSuccess {
			token: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
			farming_staking_amount: BalanceOf<T>,
			system_stakable_amount: BalanceOf<T>,
			system_shadow_amount: BalanceOf<T>,
			pending_redeem_amount: BalanceOf<T>,
		},
		WithdrawFailed {
			token: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
			farming_staking_amount: BalanceOf<T>,
			system_stakable_amount: BalanceOf<T>,
			system_shadow_amount: BalanceOf<T>,
			pending_redeem_amount: BalanceOf<T>,
		},
		Redeemed {
			token: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
			farming_staking_amount: BalanceOf<T>,
			system_stakable_amount: BalanceOf<T>,
			system_shadow_amount: BalanceOf<T>,
			pending_redeem_amount: BalanceOf<T>,
		},
		RedeemFailed {
			token: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
			farming_staking_amount: BalanceOf<T>,
			system_stakable_amount: BalanceOf<T>,
			system_shadow_amount: BalanceOf<T>,
			pending_redeem_amount: BalanceOf<T>,
		},
		VtokenNotFound {
			token: CurrencyIdOf<T>,
		},
		TokenInfoRefreshed {
			token: CurrencyIdOf<T>,
		},
		Payout {
			token: CurrencyIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			from: AccountIdOf<T>,
			to: AccountIdOf<T>,
			amount: BalanceOf<T>,
			free: BalanceOf<T>,
			vfree: BalanceOf<T>,
			shadow: BalanceOf<T>,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Invalid token config params
		InvalidTokenConfig,
		/// exceed max token len
		ExceedMaxTokenLen,
		/// exceed max poolid len
		ExceedMaxFarmingPoolidLen,
		/// Token info not found
		TokenInfoNotFound,
		/// payout error
		PayoutFailed,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: T::BlockNumber) -> Weight {
			// get token list
			let token_list = Self::token_list();

			let mut round = if let Some(round) = <Round<T>>::get() {
				round
			} else {
				RoundInfo::new(0u32, 0u32.into(), T::BlocksPerRound::get())
			};
			// new round start
			if round.should_update(n) {
				// mutate round
				round.update(n);
				<Round<T>>::put(round);
				Self::deposit_event(Event::NewRound {
					current: round.current,
					first: round.first,
					length: round.length,
				});
			}

			let pallet_account: AccountIdOf<T> = T::PalletId::get().into_account_truncating();
			for i in token_list.into_iter() {
				if let Some(mut token_info) = Self::token_status(i) {
					if round.check_delay(n, token_info.current_config.exec_delay) {
						if token_info.check_config_change() {
							token_info.update_config();
						}
						Self::process_token_info(pallet_account.clone(), token_info, i);
					}
				}
			}

			T::WeightInfo::on_initialize()
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
			exec_delay: Option<u32>, // TODO: blocknum
			system_stakable_farming_rate: Option<Permill>,
			add_or_sub: Option<bool>,
			system_stakable_base: Option<BalanceOf<T>>,
			farming_poolids: Option<Vec<PoolId>>,
			lptoken_rates: Option<Vec<Perbill>>, // TODO, can be > 1
		) -> DispatchResultWithPostInfo {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?; // Motion

			let mut new_token = false;
			let mut token_info = if let Some(state) = <TokenStatus<T>>::get(&token) {
				state
			} else {
				new_token = true;
				<TokenInfo<BalanceOf<T>>>::default()
			};

			token_info.new_config = token_info.current_config.clone();

			if let Some(exec_delay) = exec_delay {
				ensure!(exec_delay != 0, Error::<T>::InvalidTokenConfig);
				token_info.new_config.exec_delay = exec_delay;
			}

			if let Some(system_stakable_farming_rate) = system_stakable_farming_rate {
				ensure!(
					system_stakable_farming_rate >= Permill::zero(),
					Error::<T>::InvalidTokenConfig
				);
				token_info.new_config.system_stakable_farming_rate = system_stakable_farming_rate;
			}

			if let Some(system_stakable_base) = system_stakable_base {
				token_info.new_config.system_stakable_base = system_stakable_base;
			}

			if let Some(add_or_sub) = add_or_sub {
				token_info.new_config.add_or_sub = add_or_sub;
			}

			if let Some(farming_poolids) = farming_poolids.clone() {
				ensure!(!farming_poolids.is_empty(), Error::<T>::InvalidTokenConfig);
				ensure!(
					farming_poolids.len() as u32 <= T::MaxFarmingPoolIdLen::get(),
					Error::<T>::ExceedMaxFarmingPoolidLen
				);
				token_info.new_config.farming_poolids = farming_poolids.clone();
			}

			if let Some(lptoken_rates) = lptoken_rates.clone() {
				ensure!(!lptoken_rates.is_empty(), Error::<T>::InvalidTokenConfig);
				ensure!(
					lptoken_rates.len() as u32 <= T::MaxFarmingPoolIdLen::get(),
					Error::<T>::ExceedMaxFarmingPoolidLen
				);
				token_info.new_config.lptoken_rates = lptoken_rates.clone();
			}

			<TokenStatus<T>>::insert(&token, token_info.clone());
			if new_token {
				let mut token_list = Self::token_list();
				token_list.try_push(token).map_err(|_| Error::<T>::ExceedMaxTokenLen)?;
				<TokenList<T>>::put(token_list);
			}

			Self::deposit_event(Event::TokenConfigChanged {
				token,
				exec_delay: token_info.new_config.exec_delay,
				system_stakable_farming_rate: token_info.new_config.system_stakable_farming_rate,
				add_or_sub: token_info.new_config.add_or_sub,
				system_stakable_base: token_info.new_config.system_stakable_base,
				farming_poolids: token_info.new_config.farming_poolids.clone(),
				lptoken_rates: token_info.new_config.lptoken_rates.clone(),
			});

			Ok(().into())
		}

		/// token config，take effect when next round begins
		#[pallet::weight(<T as Config>::WeightInfo::delete_token())]
		pub fn delete_token(
			origin: OriginFor<T>,
			token: CurrencyIdOf<T>,
		) -> DispatchResultWithPostInfo {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?; // Motion

			<TokenStatus<T>>::remove(&token);

			let mut token_list = Self::token_list();
			token_list.retain(|&x| x != token);
			<TokenList<T>>::put(token_list);

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
			let mut token_info =
				<TokenStatus<T>>::get(&token).ok_or(Error::<T>::TokenInfoNotFound)?;
			if token_info.check_config_change() {
				token_info.update_config();
			}

			let pallet_account: AccountIdOf<T> = T::PalletId::get().into_account_truncating();
			Pallet::<T>::process_token_info(pallet_account, token_info, token);

			Self::deposit_event(Event::TokenInfoRefreshed { token });

			Ok(().into())
		}

		/// payout to treasury
		#[pallet::weight(<T as Config>::WeightInfo::payout())]
		pub fn payout(origin: OriginFor<T>, token: CurrencyIdOf<T>) -> DispatchResultWithPostInfo {
			T::EnsureConfirmAsGovernance::ensure_origin(origin)?; // Motion

			let token_info = <TokenStatus<T>>::get(&token).ok_or(Error::<T>::TokenInfoNotFound)?;

			let vtoken_id =
				T::VtokenMintingInterface::vtoken_id(token).ok_or(Error::<T>::TokenInfoNotFound)?;

			let pallet_account: AccountIdOf<T> = T::PalletId::get().into_account_truncating();

			let vfree_amount = T::MultiCurrency::free_balance(vtoken_id, &pallet_account);

			let free_amount =
				T::VtokenMintingInterface::vtoken_to_token(token, vtoken_id, vfree_amount);

			let token_amount = free_amount.saturating_sub(token_info.system_shadow_amount);

			let vtoken_amount =
				T::VtokenMintingInterface::token_to_vtoken(token, vtoken_id, token_amount);

			T::MultiCurrency::transfer(
				vtoken_id,
				&pallet_account,
				&T::TreasuryAccount::get(),
				vtoken_amount,
			)
			.map_err(|_| Error::<T>::PayoutFailed)?;

			Self::deposit_event(Event::Payout {
				token,
				vtoken: vtoken_id,
				from: pallet_account,
				to: T::TreasuryAccount::get(),
				amount: vtoken_amount,
				vfree: vfree_amount,
				free: free_amount,
				shadow: token_info.system_shadow_amount,
			});

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
		let mut farming_staking_amount = BalanceOf::<T>::zero();
		for i in 0..token_info.current_config.farming_poolids.len() {
			farming_staking_amount = farming_staking_amount +
				token_info.current_config.lptoken_rates[i].mul_floor(
					T::FarmingInfo::get_token_shares(
						token_info.current_config.farming_poolids[i],
						token_id,
					),
				);
		}
		token_info.farming_staking_amount = farming_staking_amount;

		// check amount, and call vtoken minting pallet
		let stakable_amount = if token_info.current_config.add_or_sub {
			token_info
				.current_config
				.system_stakable_farming_rate
				.mul_floor(token_info.farming_staking_amount)
				.saturating_add(token_info.current_config.system_stakable_base)
		} else {
			token_info
				.current_config
				.system_stakable_farming_rate
				.mul_floor(token_info.farming_staking_amount)
				.saturating_sub(token_info.current_config.system_stakable_base)
		};
		token_info.system_stakable_amount = stakable_amount;

		// write state before other calls
		<TokenStatus<T>>::insert(&token_id, token_info.clone());

		if stakable_amount >
			token_info.system_shadow_amount.saturating_sub(token_info.pending_redeem_amount)
		{
			let mint_amount = stakable_amount.saturating_sub(
				token_info.system_shadow_amount.saturating_sub(token_info.pending_redeem_amount),
			);
			match T::MultiCurrency::deposit(token_id, &account, mint_amount) {
				Ok(_) =>
					match T::VtokenMintingInterface::mint(account.clone(), token_id, mint_amount) {
						Ok(_) => {
							token_info.system_shadow_amount =
								token_info.system_shadow_amount.saturating_add(mint_amount);
							Self::deposit_event(Event::MintSuccess {
								token: token_id,
								amount: mint_amount,
								farming_staking_amount: token_info.farming_staking_amount,
								system_stakable_amount: token_info.system_stakable_amount,
								system_shadow_amount: token_info.system_shadow_amount,
								pending_redeem_amount: token_info.pending_redeem_amount,
							});
						},
						Err(error) => {
							Self::deposit_event(Event::MintFailed {
								token: token_id,
								amount: mint_amount,
								farming_staking_amount: token_info.farming_staking_amount,
								system_stakable_amount: token_info.system_stakable_amount,
								system_shadow_amount: token_info.system_shadow_amount,
								pending_redeem_amount: token_info.pending_redeem_amount,
							});
							log::warn!("mint error: {:?}", error);
							match T::MultiCurrency::withdraw(token_id, &account, mint_amount) {
								Ok(_) => {
									Self::deposit_event(Event::WithdrawSuccess {
										token: token_id,
										amount: mint_amount,
										farming_staking_amount: token_info.farming_staking_amount,
										system_stakable_amount: token_info.system_stakable_amount,
										system_shadow_amount: token_info.system_shadow_amount,
										pending_redeem_amount: token_info.pending_redeem_amount,
									});
								},
								Err(error) => {
									log::warn!("{:?} withdraw error: {:?}", &token_id, error);
									Self::deposit_event(Event::WithdrawFailed {
										token: token_id,
										amount: mint_amount,
										farming_staking_amount: token_info.farming_staking_amount,
										system_stakable_amount: token_info.system_stakable_amount,
										system_shadow_amount: token_info.system_shadow_amount,
										pending_redeem_amount: token_info.pending_redeem_amount,
									});
								},
							}
						},
					},
				Err(error) => {
					Self::deposit_event(Event::DepositFailed {
						token: token_id,
						amount: mint_amount,
						farming_staking_amount: token_info.farming_staking_amount,
						system_stakable_amount: token_info.system_stakable_amount,
						system_shadow_amount: token_info.system_shadow_amount,
						pending_redeem_amount: token_info.pending_redeem_amount,
					});
					log::warn!("{:?} deposit error: {:?}", token_id, error);
				},
			}
		}

		if stakable_amount <
			token_info.system_shadow_amount.saturating_sub(token_info.pending_redeem_amount)
		{
			let redeem_amount = token_info
				.system_shadow_amount
				.saturating_sub(token_info.pending_redeem_amount)
				.saturating_sub(stakable_amount);
			match T::VtokenMintingInterface::vtoken_id(token_id) {
				Some(vtoken_id) => {
					let vredeem_amount = T::VtokenMintingInterface::token_to_vtoken(
						token_id,
						vtoken_id,
						redeem_amount,
					);
					if vredeem_amount != BalanceOf::<T>::zero() {
						match T::VtokenMintingInterface::redeem(account, vtoken_id, vredeem_amount)
						{
							Ok(_) => {
								let new_token_info =
									if let Some(state) = <TokenStatus<T>>::get(&token_id) {
										state
									} else {
										<TokenInfo<BalanceOf<T>>>::default()
									};
								token_info.pending_redeem_amount =
									new_token_info.pending_redeem_amount;
							},
							Err(error) => {
								Self::deposit_event(Event::RedeemFailed {
									token: token_id,
									amount: vredeem_amount,
									farming_staking_amount: token_info.farming_staking_amount,
									system_stakable_amount: token_info.system_stakable_amount,
									system_shadow_amount: token_info.system_shadow_amount,
									pending_redeem_amount: token_info.pending_redeem_amount,
								});
								log::warn!("redeem error: {:?}", error);
							},
						}
					}
				},
				None => {
					Self::deposit_event(Event::VtokenNotFound { token: token_id });
					log::warn!("vtoken_id not found: {:?}", token_id);
				},
			}
		}

		<TokenStatus<T>>::insert(&token_id, token_info.clone());
	}

	pub fn on_redeem_success(
		token_id: CurrencyIdOf<T>,
		to: AccountIdOf<T>,
		token_amount: BalanceOf<T>,
	) -> Weight {
		let pallet_account: AccountIdOf<T> = T::PalletId::get().into_account_truncating();
		if pallet_account != to {
			return 0 as Weight;
		}
		let mut token_info = if let Some(state) = <TokenStatus<T>>::get(&token_id) {
			state
		} else {
			<TokenInfo<BalanceOf<T>>>::default()
		};

		token_info.pending_redeem_amount =
			token_info.pending_redeem_amount.saturating_sub(token_amount);
		match T::MultiCurrency::withdraw(token_id, &to, token_amount) {
			Ok(_) => {
				Self::deposit_event(Event::WithdrawSuccess {
					token: token_id,
					amount: token_amount,
					farming_staking_amount: token_info.farming_staking_amount,
					system_stakable_amount: token_info.system_stakable_amount,
					system_shadow_amount: token_info.system_shadow_amount,
					pending_redeem_amount: token_info.pending_redeem_amount,
				});
				token_info.system_shadow_amount =
					token_info.system_shadow_amount.saturating_sub(token_amount);
			},
			Err(error) => {
				log::warn!("{:?} withdraw error: {:?}", &token_id, error);
				Self::deposit_event(Event::WithdrawFailed {
					token: token_id,
					amount: token_amount,
					farming_staking_amount: token_info.farming_staking_amount,
					system_stakable_amount: token_info.system_stakable_amount,
					system_shadow_amount: token_info.system_shadow_amount,
					pending_redeem_amount: token_info.pending_redeem_amount,
				});
			},
		}
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

	pub fn on_redeemed(
		address: AccountIdOf<T>,
		token_id: CurrencyIdOf<T>,
		token_amount: BalanceOf<T>,
		vtoken_amount: BalanceOf<T>,
		fee: BalanceOf<T>,
	) -> Weight {
		let pallet_account: AccountIdOf<T> = T::PalletId::get().into_account_truncating();
		if pallet_account != address {
			return 0 as Weight;
		}
		let mut token_info = if let Some(state) = <TokenStatus<T>>::get(&token_id) {
			state
		} else {
			<TokenInfo<BalanceOf<T>>>::default()
		};

		token_info.pending_redeem_amount =
			token_info.pending_redeem_amount.saturating_add(token_amount);

		<TokenStatus<T>>::insert(&token_id, token_info.clone());

		Self::deposit_event(Event::Redeemed {
			token: token_id,
			amount: token_amount,
			farming_staking_amount: token_info.farming_staking_amount,
			system_stakable_amount: token_info.system_stakable_amount,
			system_shadow_amount: token_info.system_shadow_amount,
			pending_redeem_amount: token_info.pending_redeem_amount,
		});
		T::WeightInfo::on_redeemed()
	}
}
