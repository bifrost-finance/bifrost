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
pub enum TargetChain<AccountId> {
	Astar(AccountId),
	Moonbeam(H160),
	Hydradx(AccountId),
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
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
		type MaxWhitelistNumber: Get<u32>;

		#[pallet::constant]
		type ParachainId: Get<ParaId>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		AddWhitelistAccountId {
			wasm_caller: AccountIdOf<T>,
		},
		RemoveWhitelistAccountId {
			wasm_caller: AccountIdOf<T>,
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
		XcmSwap {
			evm_caller: H160,
			currency_id_in: CurrencyIdOf<T>,
			currency_id_out: CurrencyIdOf<T>,
			target_chain: TargetChain<AccountIdOf<T>>,
		},
		XcmSwapFailed {
			evm_caller: H160,
			currency_id_in: CurrencyIdOf<T>,
			currency_id_out: CurrencyIdOf<T>,
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
			fee: BalanceOf<T>,
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
	}

	/// Contract whitelist
	#[pallet::storage]
	#[pallet::getter(fn whitelist_account_ids)]
	pub type WhitelistAccountId<T: Config> =
		StorageValue<_, BoundedVec<AccountIdOf<T>, T::MaxWhitelistNumber>, ValueQuery>;

	/// Charge corresponding fees for different CurrencyId
	#[pallet::storage]
	#[pallet::getter(fn execution_fee)]
	pub type ExecutionFee<T: Config> =
		StorageMap<_, Blake2_128Concat, CurrencyId, BalanceOf<T>, ValueQuery>;

	/// XCM fee for transferring to Moonbeam(BNC)
	#[pallet::storage]
	#[pallet::getter(fn transfer_to_moonbeam_fee)]
	pub type TransferToMoonbeamFee<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

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
		) -> DispatchResultWithPostInfo {
			let wasm_caller = ensure_signed(origin)?;
			Self::ensure_singer_on_whitelist(&wasm_caller)?;

			let mut evm_caller_account_id = Self::h160_to_account_id(evm_caller);
			if let TargetChain::Hydradx(_) = target_chain {
				evm_caller_account_id = wasm_caller.clone();
			}

			let token_amount =
				Self::exclude_other_fee(&target_chain, currency_id, &evm_caller_account_id)?;

			match T::VtokenMintingInterface::mint(
				evm_caller_account_id.clone(),
				currency_id,
				token_amount,
			) {
				Ok(_) => {
					// success
					let vtoken_id = T::VtokenMintingInterface::vtoken_id(currency_id)
						.ok_or(Error::<T>::TokenNotFoundInVtokenMinting)?;
					let vtoken_amount =
						T::MultiCurrency::free_balance(vtoken_id, &evm_caller_account_id);

					Self::transfer_to(
						evm_caller_account_id,
						&wasm_caller,
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
						evm_caller_account_id,
						&wasm_caller,
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

		// Swap and transfer to target chain
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::swap())]
		pub fn swap(
			origin: OriginFor<T>,
			evm_caller: H160,
			currency_id_in: CurrencyIdOf<T>,
			currency_id_out: CurrencyIdOf<T>,
			currency_id_out_min: AssetBalance,
			target_chain: TargetChain<AccountIdOf<T>>,
		) -> DispatchResultWithPostInfo {
			let wasm_caller = ensure_signed(origin)?;
			Self::ensure_singer_on_whitelist(&wasm_caller)?;

			let mut evm_caller_account_id = Self::h160_to_account_id(evm_caller);
			if let TargetChain::Hydradx(_) = target_chain {
				evm_caller_account_id = wasm_caller.clone();
			}

			let currency_id_in_amount =
				Self::exclude_other_fee(&target_chain, currency_id_in, &evm_caller_account_id)?;

			let in_asset_id: AssetId =
				AssetId::try_convert_from(currency_id_in, T::ParachainId::get().into())
					.map_err(|_| Error::<T>::TokenNotFoundInZenlink)?;
			let out_asset_id: AssetId =
				AssetId::try_convert_from(currency_id_out, T::ParachainId::get().into())
					.map_err(|_| Error::<T>::TokenNotFoundInZenlink)?;

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
						&wasm_caller,
						currency_id_out,
						currency_id_out_amount,
						&target_chain,
					)?;

					Self::deposit_event(Event::XcmSwap {
						evm_caller,
						currency_id_in,
						currency_id_out,
						target_chain,
					});
				},
				Err(_) => Self::transfer_to(
					evm_caller_account_id.clone(),
					&wasm_caller,
					currency_id_in,
					currency_id_in_amount,
					&target_chain,
				)?,
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
			let wasm_caller = ensure_signed(origin)?;
			Self::ensure_singer_on_whitelist(&wasm_caller)?;

			let mut evm_caller_account_id = Self::h160_to_account_id(evm_caller);
			if let TargetChain::Hydradx(_) = target_chain {
				evm_caller_account_id = wasm_caller.clone();
			}

			let vtoken_amount =
				Self::exclude_other_fee(&target_chain, vtoken_id, &evm_caller_account_id)?;

			let redeem_type = match target_chain {
				TargetChain::Astar(_) => RedeemType::Astar,
				TargetChain::Moonbeam(receiver) => RedeemType::Moonbeam(receiver),
				TargetChain::Hydradx(_) => RedeemType::Hydradx,
			};

			if vtoken_id == VFIL {
				T::MultiCurrency::transfer(
					BNC,
					&wasm_caller,
					&evm_caller_account_id,
					Self::transfer_to_moonbeam_fee(),
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
						&wasm_caller,
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

		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::add_whitelist())]
		pub fn add_whitelist(
			origin: OriginFor<T>,
			wasm_caller: T::AccountId,
		) -> DispatchResultWithPostInfo {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			let mut whitelist_account_ids = WhitelistAccountId::<T>::get();

			ensure!(
				!whitelist_account_ids.contains(&wasm_caller),
				Error::<T>::AccountIdAlreadyInWhitelist
			);
			whitelist_account_ids
				.try_push(wasm_caller.clone())
				.map_err(|_| Error::<T>::ExceededWhitelistMaxNumber)?;
			WhitelistAccountId::<T>::set(whitelist_account_ids);
			Self::deposit_event(Event::AddWhitelistAccountId { wasm_caller });
			Ok(().into())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::remove_whitelist())]
		pub fn remove_whitelist(
			origin: OriginFor<T>,
			wasm_caller: T::AccountId,
		) -> DispatchResultWithPostInfo {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			let mut whitelist_account_ids = WhitelistAccountId::<T>::get();

			ensure!(
				whitelist_account_ids.contains(&wasm_caller),
				Error::<T>::AccountIdNotInWhitelist
			);
			whitelist_account_ids.retain(|x| *x != wasm_caller);
			WhitelistAccountId::<T>::set(whitelist_account_ids);
			Self::deposit_event(Event::RemoveWhitelistAccountId { wasm_caller });
			Ok(().into())
		}

		#[pallet::call_index(5)]
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

		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::set_transfer_to_fee())]
		pub fn set_transfer_to_moonbeam_fee(
			origin: OriginFor<T>,
			fee: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;
			TransferToMoonbeamFee::<T>::set(fee);
			Self::deposit_event(Event::SetTransferToFee { fee });
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Check if the signer is in the whitelist
	fn ensure_singer_on_whitelist(wasm_caller: &T::AccountId) -> DispatchResult {
		let whitelist_account_ids = WhitelistAccountId::<T>::get();
		ensure!(whitelist_account_ids.contains(wasm_caller), Error::<T>::AccountIdNotInWhitelist);
		Ok(())
	}

	/// Charge an execution fee
	fn exclude_other_fee(
		target_chain: &TargetChain<AccountIdOf<T>>,
		currency_id: CurrencyIdOf<T>,
		evm_caller_account_id: &T::AccountId,
	) -> Result<BalanceOf<T>, DispatchError> {
		let free_balance = T::MultiCurrency::free_balance(currency_id, evm_caller_account_id);
		let minimum_balance = T::MultiCurrency::minimum_balance(currency_id);
		match target_chain {
			TargetChain::Hydradx(_) => {
				let exclude_fee = free_balance
					.checked_sub(&minimum_balance)
					.ok_or(Error::<T>::FreeBalanceTooLow)?;
				Ok(exclude_fee)
			},
			_ => {
				let execution_fee = Self::execution_fee(currency_id);
				T::MultiCurrency::transfer(
					currency_id,
					evm_caller_account_id,
					&T::TreasuryAccount::get(),
					execution_fee,
				)?;
				let exclude_fee = free_balance
					.checked_sub(&execution_fee.saturating_add(minimum_balance))
					.ok_or(Error::<T>::FreeBalanceTooLow)?;
				Ok(exclude_fee)
			},
		}
	}

	fn transfer_to(
		caller: T::AccountId,
		wasm_caller: &T::AccountId,
		currency_id: CurrencyIdOf<T>,
		amount: BalanceOf<T>,
		target_chain: &TargetChain<T::AccountId>,
	) -> DispatchResult {
		match target_chain {
			TargetChain::Astar(receiver) => {
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
				let fee_amount = Self::transfer_to_moonbeam_fee();
				match currency_id {
					VKSM | VMOVR | VBNC | FIL | VFIL | VDOT | VGLMR => {
						T::MultiCurrency::transfer(BNC, wasm_caller, &caller, fee_amount)?;
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

	fn h160_to_account_id(address: H160) -> T::AccountId {
		let mut data = [0u8; 24];
		data[0..4].copy_from_slice(b"evm:");
		data[4..24].copy_from_slice(&address[..]);
		let hash = BlakeTwo256::hash(&data);

		let account_id_32 = sp_runtime::AccountId32::from(Into::<[u8; 32]>::into(hash));
		T::AccountId::decode(&mut account_id_32.as_ref()).expect("Fail to decode address")
	}
}

// Functions to be called by other pallets.
impl<T: Config> SlpxOperator<BalanceOf<T>> for Pallet<T> {
	fn get_moonbeam_transfer_to_fee() -> BalanceOf<T> {
		Self::transfer_to_moonbeam_fee()
	}
}
