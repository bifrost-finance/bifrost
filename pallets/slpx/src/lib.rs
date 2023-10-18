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
use bifrost_asset_registry::AssetMetadata;
use codec::{Decode, Encode, MaxEncodedLen};
use cumulus_primitives_core::ParaId;
use frame_support::{
	dispatch::{DispatchResult, DispatchResultWithPostInfo},
	ensure,
	sp_runtime::SaturatedConversion,
	traits::Get,
	RuntimeDebug,
};
use frame_system::{ensure_signed, pallet_prelude::OriginFor};
use node_primitives::{
	currency::{BNC, FIL, VBNC, VDOT, VFIL, VGLMR, VKSM, VMOVR},
	CurrencyId, CurrencyIdMapping, SlpxOperator, TokenInfo, TryConvertFrom, VtokenMintingInterface,
};
use orml_traits::{MultiCurrency, XcmTransfer};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_core::{Hasher, H160};
use sp_runtime::{
	traits::{BlakeTwo256, CheckedSub},
	DispatchError, Saturating,
};
use sp_std::vec;
use xcm::{latest::prelude::*, v3::MultiLocation};
use zenlink_protocol::AssetBalance;

pub mod weights;
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;
pub type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

#[derive(
	Encode,
	Decode,
	MaxEncodedLen,
	Eq,
	PartialEq,
	Copy,
	Clone,
	RuntimeDebug,
	PartialOrd,
	Ord,
	TypeInfo,
)]
pub enum SupportChain {
	Astar,
	Moonbeam,
	Hydradx,
}

#[derive(
	Encode,
	Decode,
	MaxEncodedLen,
	Eq,
	PartialEq,
	Copy,
	Clone,
	RuntimeDebug,
	PartialOrd,
	Ord,
	TypeInfo,
)]
pub enum TargetChain<AccountId> {
	Astar(H160),
	Moonbeam(H160),
	Hydradx(AccountId),
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use bifrost_stable_pool::{traits::StablePoolHandler, PoolTokenIndex, StableAssetPoolId};
	use frame_support::pallet_prelude::{ValueQuery, *};
	use node_primitives::RedeemType;
	use zenlink_protocol::{AssetId, ExportZenlink};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type ControlOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type DexOperator: ExportZenlink<Self::AccountId, AssetId>;

		/// The interface to call VtokenMinting module functions.
		type VtokenMintingInterface: VtokenMintingInterface<
			AccountIdOf<Self>,
			CurrencyIdOf<Self>,
			BalanceOf<Self>,
		>;

		/// The interface to call StablePool module functions.
		type StablePoolHandler: StablePoolHandler<
			Balance = BalanceOf<Self>,
			AccountId = AccountIdOf<Self>,
			CurrencyId = CurrencyIdOf<Self>,
		>;

		/// xtokens xcm transfer interface
		type XcmTransfer: XcmTransfer<AccountIdOf<Self>, BalanceOf<Self>, CurrencyIdOf<Self>>;

		/// Convert MultiLocation to `T::CurrencyId`.
		type CurrencyIdConvert: CurrencyIdMapping<
			CurrencyId,
			MultiLocation,
			AssetMetadata<BalanceOf<Self>>,
		>;

		/// TreasuryAccount
		#[pallet::constant]
		type TreasuryAccount: Get<AccountIdOf<Self>>;

		#[pallet::constant]
		type ParachainId: Get<ParaId>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		AddWhitelistAccountId {
			support_chain: SupportChain,
			evm_contract_account_id: AccountIdOf<T>,
		},
		RemoveWhitelistAccountId {
			support_chain: SupportChain,
			evm_contract_account_id: AccountIdOf<T>,
		},
		XcmMint {
			evm_caller: H160,
			currency_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
			target_chain: TargetChain<AccountIdOf<T>>,
		},
		XcmMintFailed {
			evm_caller: H160,
			currency_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
			target_chain: TargetChain<AccountIdOf<T>>,
		},
		XcmZenlinkSwap {
			evm_caller: H160,
			currency_id_in: CurrencyIdOf<T>,
			currency_id_out: CurrencyIdOf<T>,
			currency_id_out_amount: BalanceOf<T>,
			target_chain: TargetChain<AccountIdOf<T>>,
		},
		XcmZenlinkSwapFailed {
			evm_caller: H160,
			currency_id_in: CurrencyIdOf<T>,
			currency_id_out: CurrencyIdOf<T>,
			currency_id_in_amount: BalanceOf<T>,
			target_chain: TargetChain<AccountIdOf<T>>,
		},
		XcmStablePoolSwap {
			evm_caller: H160,
			pool_token_index_in: PoolTokenIndex,
			pool_token_index_out: PoolTokenIndex,
			currency_id_out_amount: BalanceOf<T>,
			target_chain: TargetChain<AccountIdOf<T>>,
		},
		XcmStablePoolSwapFailed {
			evm_caller: H160,
			pool_token_index_in: PoolTokenIndex,
			pool_token_index_out: PoolTokenIndex,
			currency_id_in_amount: BalanceOf<T>,
			target_chain: TargetChain<AccountIdOf<T>>,
		},
		XcmRedeem {
			evm_caller: H160,
			vtoken_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
			target_chain: TargetChain<AccountIdOf<T>>,
		},
		XcmRedeemFailed {
			evm_caller: H160,
			vtoken_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
			target_chain: TargetChain<AccountIdOf<T>>,
		},
		SetTransferToFee {
			support_chain: SupportChain,
			transfer_to_fee: BalanceOf<T>,
		},
		SetExecutionFee {
			currency_id: CurrencyId,
			execution_fee: BalanceOf<T>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Token not found in vtoken minting
		TokenNotFoundInVtokenMinting,
		/// Token not found in zenlink
		TokenNotFoundInZenlink,
		/// Contract Account already exists in the whitelist
		AccountIdAlreadyInWhitelist,
		/// Contract Account is not in the whitelist
		AccountIdNotInWhitelist,
		/// The maximum number of whitelist addresses is 10
		ExceededWhitelistMaxNumber,
		/// Execution fee not set
		NotSetExecutionFee,
		/// Insufficient balance to execute the fee
		FreeBalanceTooLow,
		/// ArgumentsError
		ArgumentsError,
	}

	/// Contract whitelist
	#[pallet::storage]
	#[pallet::getter(fn whitelist_account_ids)]
	pub type WhitelistAccountId<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		SupportChain,
		BoundedVec<AccountIdOf<T>, ConstU32<10>>,
		ValueQuery,
	>;

	/// Charge corresponding fees for different CurrencyId
	#[pallet::storage]
	#[pallet::getter(fn execution_fee)]
	pub type ExecutionFee<T: Config> =
		StorageMap<_, Blake2_128Concat, CurrencyId, BalanceOf<T>, OptionQuery>;

	/// XCM fee for transferring to Moonbeam(BNC)
	#[pallet::storage]
	#[pallet::getter(fn transfer_to_fee)]
	pub type TransferToFee<T: Config> =
		StorageMap<_, Blake2_128Concat, SupportChain, BalanceOf<T>, OptionQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// vtoken mint and transfer to target chain
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::mint())]
		pub fn mint(
			origin: OriginFor<T>,
			evm_caller: H160,
			currency_id: CurrencyIdOf<T>,
			target_chain: TargetChain<AccountIdOf<T>>,
			remark: BoundedVec<u8, ConstU32<32>>,
		) -> DispatchResultWithPostInfo {
			let (evm_contract_account_id, evm_caller_account_id) =
				Self::ensure_singer_on_whitelist(origin, evm_caller, &target_chain)?;

			let token_amount = Self::charge_execution_fee(currency_id, &evm_caller_account_id)?;

			match T::VtokenMintingInterface::mint(
				evm_caller_account_id.clone(),
				currency_id,
				token_amount,
				remark,
			) {
				Ok(_) => {
					// success
					let vtoken_id = T::VtokenMintingInterface::vtoken_id(currency_id)
						.ok_or(Error::<T>::TokenNotFoundInVtokenMinting)?;
					let vtoken_amount =
						T::MultiCurrency::free_balance(vtoken_id, &evm_caller_account_id);

					Self::transfer_to(
						evm_caller_account_id.clone(),
						&evm_contract_account_id,
						vtoken_id,
						vtoken_amount,
						&target_chain,
					)?;

					Self::deposit_event(Event::XcmMint {
						evm_caller,
						currency_id,
						token_amount,
						target_chain,
					});
				},
				Err(_) => {
					Self::transfer_to(
						evm_caller_account_id.clone(),
						&evm_contract_account_id,
						currency_id,
						token_amount,
						&target_chain,
					)?;
					Self::deposit_event(Event::XcmMintFailed {
						evm_caller,
						currency_id,
						token_amount,
						target_chain,
					});
				},
			};
			Ok(().into())
		}

		/// Swap and transfer to target chain
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::zenlink_swap())]
		pub fn zenlink_swap(
			origin: OriginFor<T>,
			evm_caller: H160,
			currency_id_in: CurrencyIdOf<T>,
			currency_id_out: CurrencyIdOf<T>,
			currency_id_out_min: AssetBalance,
			target_chain: TargetChain<AccountIdOf<T>>,
		) -> DispatchResultWithPostInfo {
			let (evm_contract_account_id, evm_caller_account_id) =
				Self::ensure_singer_on_whitelist(origin, evm_caller, &target_chain)?;

			let in_asset_id: AssetId =
				AssetId::try_convert_from(currency_id_in, T::ParachainId::get().into())
					.map_err(|_| Error::<T>::TokenNotFoundInZenlink)?;
			let out_asset_id: AssetId =
				AssetId::try_convert_from(currency_id_out, T::ParachainId::get().into())
					.map_err(|_| Error::<T>::TokenNotFoundInZenlink)?;

			let currency_id_in_amount =
				Self::charge_execution_fee(currency_id_in, &evm_caller_account_id)?;

			let path = vec![in_asset_id, out_asset_id];
			match T::DexOperator::inner_swap_exact_assets_for_assets(
				&evm_caller_account_id,
				currency_id_in_amount.saturated_into(),
				currency_id_out_min,
				&path,
				&evm_caller_account_id,
			) {
				Ok(_) => {
					let currency_id_out_amount =
						T::MultiCurrency::free_balance(currency_id_out, &evm_caller_account_id);

					Self::transfer_to(
						evm_caller_account_id.clone(),
						&evm_contract_account_id,
						currency_id_out,
						currency_id_out_amount,
						&target_chain,
					)?;

					Self::deposit_event(Event::XcmZenlinkSwap {
						evm_caller,
						currency_id_in,
						currency_id_out,
						currency_id_out_amount,
						target_chain,
					});
				},
				Err(_) => {
					Self::transfer_to(
						evm_caller_account_id.clone(),
						&evm_contract_account_id,
						currency_id_in,
						currency_id_in_amount,
						&target_chain,
					)?;

					Self::deposit_event(Event::XcmZenlinkSwapFailed {
						evm_caller,
						currency_id_in,
						currency_id_out,
						currency_id_in_amount,
						target_chain,
					});
				},
			}
			Ok(().into())
		}

		/// Redeem
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::redeem())]
		pub fn redeem(
			origin: OriginFor<T>,
			evm_caller: H160,
			vtoken_id: CurrencyIdOf<T>,
			target_chain: TargetChain<AccountIdOf<T>>,
		) -> DispatchResultWithPostInfo {
			let (evm_contract_account_id, evm_caller_account_id) =
				Self::ensure_singer_on_whitelist(origin, evm_caller, &target_chain)?;
			let vtoken_amount = Self::charge_execution_fee(vtoken_id, &evm_caller_account_id)?;

			let redeem_type = match target_chain.clone() {
				TargetChain::Astar(receiver) => {
					let receiver = Self::h160_to_account_id(receiver);
					RedeemType::Astar(receiver)
				},
				TargetChain::Moonbeam(receiver) => RedeemType::Moonbeam(receiver),
				TargetChain::Hydradx(receiver) => RedeemType::Hydradx(receiver),
			};

			if vtoken_id == VFIL {
				let fee_amount = Self::transfer_to_fee(SupportChain::Moonbeam)
					.unwrap_or_else(|| Self::get_default_fee(BNC));
				T::MultiCurrency::transfer(
					BNC,
					&evm_contract_account_id,
					&evm_caller_account_id,
					fee_amount,
				)?;
			}

			match T::VtokenMintingInterface::slpx_redeem(
				evm_caller_account_id.clone(),
				vtoken_id,
				vtoken_amount,
				redeem_type,
			) {
				Ok(_) => Self::deposit_event(Event::XcmRedeem {
					evm_caller,
					vtoken_id,
					vtoken_amount,
					target_chain,
				}),
				Err(_) => {
					Self::transfer_to(
						evm_caller_account_id.clone(),
						&evm_contract_account_id,
						vtoken_id,
						vtoken_amount,
						&target_chain,
					)?;
					Self::deposit_event(Event::XcmRedeemFailed {
						evm_caller,
						vtoken_id,
						vtoken_amount,
						target_chain,
					});
				},
			};
			Ok(().into())
		}

		/// Stable pool swap
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::stable_pool_swap())]
		pub fn stable_pool_swap(
			origin: OriginFor<T>,
			evm_caller: H160,
			pool_id: StableAssetPoolId,
			currency_id_in: CurrencyIdOf<T>,
			currency_id_out: CurrencyIdOf<T>,
			min_dy: BalanceOf<T>,
			target_chain: TargetChain<AccountIdOf<T>>,
		) -> DispatchResult {
			let (evm_contract_account_id, evm_caller_account_id) =
				Self::ensure_singer_on_whitelist(origin, evm_caller, &target_chain)?;
			let pool_token_index_in =
				T::StablePoolHandler::get_pool_token_index(pool_id, currency_id_in)
					.ok_or(Error::<T>::ArgumentsError)?;
			let pool_token_index_out =
				T::StablePoolHandler::get_pool_token_index(pool_id, currency_id_out)
					.ok_or(Error::<T>::ArgumentsError)?;
			let currency_id_in_amount =
				Self::charge_execution_fee(currency_id_in, &evm_caller_account_id)?;

			match T::StablePoolHandler::swap(
				&evm_caller_account_id,
				pool_id,
				pool_token_index_in,
				pool_token_index_out,
				currency_id_in_amount.saturated_into(),
				min_dy.saturated_into(),
			) {
				Ok(_) => {
					let currency_id_out_amount =
						T::MultiCurrency::free_balance(currency_id_out, &evm_caller_account_id);

					Self::transfer_to(
						evm_caller_account_id.clone(),
						&evm_contract_account_id,
						currency_id_out,
						currency_id_out_amount,
						&target_chain,
					)?;

					Self::deposit_event(Event::XcmStablePoolSwap {
						evm_caller,
						pool_token_index_in,
						pool_token_index_out,
						currency_id_out_amount,
						target_chain,
					});
				},
				Err(_) => {
					Self::transfer_to(
						evm_caller_account_id.clone(),
						&evm_contract_account_id,
						currency_id_in,
						currency_id_in_amount,
						&target_chain,
					)?;
					Self::deposit_event(Event::XcmStablePoolSwapFailed {
						evm_caller,
						pool_token_index_in,
						pool_token_index_out,
						currency_id_in_amount,
						target_chain,
					});
				},
			};
			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::add_whitelist())]
		pub fn add_whitelist(
			origin: OriginFor<T>,
			support_chain: SupportChain,
			evm_contract_account_id: T::AccountId,
		) -> DispatchResultWithPostInfo {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			let mut whitelist_account_ids = WhitelistAccountId::<T>::get(&support_chain);

			ensure!(
				!whitelist_account_ids.contains(&evm_contract_account_id),
				Error::<T>::AccountIdAlreadyInWhitelist
			);
			whitelist_account_ids
				.try_push(evm_contract_account_id.clone())
				.map_err(|_| Error::<T>::ExceededWhitelistMaxNumber)?;
			WhitelistAccountId::<T>::insert(support_chain, whitelist_account_ids);
			Self::deposit_event(Event::AddWhitelistAccountId {
				support_chain,
				evm_contract_account_id,
			});
			Ok(().into())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::remove_whitelist())]
		pub fn remove_whitelist(
			origin: OriginFor<T>,
			support_chain: SupportChain,
			evm_contract_account_id: T::AccountId,
		) -> DispatchResultWithPostInfo {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			let mut whitelist_account_ids = WhitelistAccountId::<T>::get(&support_chain);

			ensure!(
				whitelist_account_ids.contains(&evm_contract_account_id),
				Error::<T>::AccountIdNotInWhitelist
			);
			whitelist_account_ids.retain(|x| *x != evm_contract_account_id);
			WhitelistAccountId::<T>::insert(support_chain, whitelist_account_ids);
			Self::deposit_event(Event::RemoveWhitelistAccountId {
				support_chain,
				evm_contract_account_id,
			});
			Ok(().into())
		}

		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::set_execution_fee())]
		pub fn set_execution_fee(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			execution_fee: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;
			ExecutionFee::<T>::insert(currency_id, execution_fee);
			Self::deposit_event(Event::SetExecutionFee { currency_id, execution_fee });
			Ok(().into())
		}

		#[pallet::call_index(7)]
		#[pallet::weight(<T as Config>::WeightInfo::set_transfer_to_fee())]
		pub fn set_transfer_to_fee(
			origin: OriginFor<T>,
			support_chain: SupportChain,
			transfer_to_fee: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;
			TransferToFee::<T>::insert(support_chain, transfer_to_fee);
			Self::deposit_event(Event::SetTransferToFee { support_chain, transfer_to_fee });
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Check if the signer is in the whitelist
	fn ensure_singer_on_whitelist(
		origin: OriginFor<T>,
		evm_caller: H160,
		target_chain: &TargetChain<AccountIdOf<T>>,
	) -> Result<(AccountIdOf<T>, AccountIdOf<T>), DispatchError> {
		let evm_contract_account_id = ensure_signed(origin)?;
		let mut evm_caller_account_id = Self::h160_to_account_id(evm_caller);
		let support_chain = match target_chain {
			TargetChain::Astar(_) => SupportChain::Astar,
			TargetChain::Moonbeam(_) => SupportChain::Moonbeam,
			TargetChain::Hydradx(_) => {
				evm_caller_account_id = evm_contract_account_id.clone();
				SupportChain::Hydradx
			},
		};
		let whitelist_account_ids = WhitelistAccountId::<T>::get(&support_chain);
		ensure!(
			whitelist_account_ids.contains(&evm_contract_account_id),
			Error::<T>::AccountIdNotInWhitelist
		);
		Ok((evm_contract_account_id, evm_caller_account_id))
	}

	/// Charge an execution fee
	fn charge_execution_fee(
		currency_id: CurrencyIdOf<T>,
		evm_caller_account_id: &AccountIdOf<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		let free_balance = T::MultiCurrency::free_balance(currency_id, evm_caller_account_id);
		let execution_fee =
			Self::execution_fee(currency_id).unwrap_or_else(|| Self::get_default_fee(currency_id));
		let minimum_balance = T::MultiCurrency::minimum_balance(currency_id);
		T::MultiCurrency::transfer(
			currency_id,
			evm_caller_account_id,
			&T::TreasuryAccount::get(),
			execution_fee,
		)?;
		let balance_exclude_fee = free_balance
			.checked_sub(&execution_fee.saturating_add(minimum_balance))
			.ok_or(Error::<T>::FreeBalanceTooLow)?;
		Ok(balance_exclude_fee)
	}

	fn transfer_to(
		caller: AccountIdOf<T>,
		evm_contract_account_id: &AccountIdOf<T>,
		currency_id: CurrencyIdOf<T>,
		amount: BalanceOf<T>,
		target_chain: &TargetChain<AccountIdOf<T>>,
	) -> DispatchResult {
		match target_chain {
			TargetChain::Astar(receiver) => {
				let receiver = Self::h160_to_account_id(*receiver);
				let dest = MultiLocation {
					parents: 1,
					interior: X2(
						Parachain(T::VtokenMintingInterface::get_astar_parachain_id()),
						AccountId32 { network: None, id: receiver.encode().try_into().unwrap() },
					),
				};

				T::XcmTransfer::transfer(caller, currency_id, amount, dest, Unlimited)?;
			},
			TargetChain::Hydradx(receiver) => {
				let dest = MultiLocation {
					parents: 1,
					interior: X2(
						Parachain(T::VtokenMintingInterface::get_hydradx_parachain_id()),
						AccountId32 { network: None, id: receiver.encode().try_into().unwrap() },
					),
				};

				T::XcmTransfer::transfer(caller, currency_id, amount, dest, Unlimited)?;
			},
			TargetChain::Moonbeam(receiver) => {
				let dest = MultiLocation {
					parents: 1,
					interior: X2(
						Parachain(T::VtokenMintingInterface::get_moonbeam_parachain_id()),
						AccountKey20 { network: None, key: receiver.to_fixed_bytes() },
					),
				};
				let fee_amount = Self::transfer_to_fee(SupportChain::Moonbeam)
					.unwrap_or_else(|| Self::get_default_fee(BNC));
				match currency_id {
					VKSM | VMOVR | VBNC | FIL | VFIL | VDOT | VGLMR => {
						T::MultiCurrency::transfer(
							BNC,
							evm_contract_account_id,
							&caller,
							fee_amount,
						)?;
						let assets = vec![(currency_id, amount), (BNC, fee_amount)];

						T::XcmTransfer::transfer_multicurrencies(
							caller, assets, 1, dest, Unlimited,
						)?;
					},
					_ => {
						T::XcmTransfer::transfer(caller, currency_id, amount, dest, Unlimited)?;
					},
				};
			},
		};
		Ok(())
	}

	fn h160_to_account_id(address: H160) -> AccountIdOf<T> {
		let mut data = [0u8; 24];
		data[0..4].copy_from_slice(b"evm:");
		data[4..24].copy_from_slice(&address[..]);
		let hash = BlakeTwo256::hash(&data);

		let account_id_32 = sp_runtime::AccountId32::from(Into::<[u8; 32]>::into(hash));
		T::AccountId::decode(&mut account_id_32.as_ref()).expect("Fail to decode address")
	}

	pub fn get_default_fee(currency_id: CurrencyId) -> BalanceOf<T> {
		let decimals = currency_id
			.decimals()
			.unwrap_or(
				T::CurrencyIdConvert::get_currency_metadata(currency_id)
					.map_or(12, |metatata| metatata.decimals.into()),
			)
			.into();

		BalanceOf::<T>::saturated_from(10u128.saturating_pow(decimals).saturating_div(100u128))
	}
}

// Functions to be called by other pallets.
impl<T: Config> SlpxOperator<BalanceOf<T>> for Pallet<T> {
	fn get_moonbeam_transfer_to_fee() -> BalanceOf<T> {
		Self::transfer_to_fee(SupportChain::Moonbeam).unwrap_or_else(|| Self::get_default_fee(BNC))
	}
}
