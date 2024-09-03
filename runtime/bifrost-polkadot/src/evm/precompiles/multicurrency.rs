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

use crate::{
	evm::{
		precompiles::{
			erc20_mapping::{BifrostErc20Mapping, Erc20Mapping},
			handle::{EvmDataWriter, FunctionModifier, PrecompileHandleExt},
			substrate::RuntimeHelper,
			succeed, Address, Output,
		},
		ExtendedAddressMapping,
	},
	Currencies,
};
use bifrost_asset_registry::AssetIdMaps;
use bifrost_primitives::{Balance, CurrencyId, CurrencyIdMapping};
use frame_support::traits::OriginTrait;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use orml_traits::{MultiCurrency as MultiCurrencyT, MultiCurrency};
use pallet_evm::{
	AddressMapping, ExitRevert, Precompile, PrecompileFailure, PrecompileHandle, PrecompileResult,
};
use primitive_types::H160;
use sp_runtime::{traits::Dispatchable, RuntimeDebug};
use sp_std::{marker::PhantomData, prelude::*};

#[module_evm_utility_macro::generate_function_selector]
#[derive(RuntimeDebug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
pub enum Action {
	Name = "name()",
	Symbol = "symbol()",
	Decimals = "decimals()",
	TotalSupply = "totalSupply()",
	BalanceOf = "balanceOf(address)",
	Allowance = "allowance(address,address)",
	Transfer = "transfer(address,uint256)",
	Approve = "approve(address,uint256)",
	TransferFrom = "transferFrom(address,address,uint256)",
}
pub struct MultiCurrencyPrecompile<Runtime>(PhantomData<Runtime>);

impl<Runtime> Precompile for MultiCurrencyPrecompile<Runtime>
where
	Runtime: frame_system::Config
		+ pallet_evm::Config
		+ bifrost_asset_registry::Config
		+ bifrost_currencies::Config,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin: OriginTrait,
	Currencies: MultiCurrency<Runtime::AccountId, CurrencyId = CurrencyId, Balance = Balance>,
	bifrost_currencies::Pallet<Runtime>:
		MultiCurrency<Runtime::AccountId, CurrencyId = CurrencyId, Balance = Balance>,
	<Runtime as frame_system::Config>::AccountId: core::convert::From<sp_runtime::AccountId32>,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin: OriginTrait,
{
	fn execute(handle: &mut impl PrecompileHandle) -> pallet_evm::PrecompileResult {
		let address = handle.code_address();
		if let Some(asset_id) = BifrostErc20Mapping::decode_evm_address(address) {
			log::debug!(target: "evm", "multicurrency: currency id: {:?}", asset_id);

			let selector = match handle.read_selector() {
				Ok(selector) => selector,
				Err(e) => return Err(e),
			};

			handle.check_function_modifier(match selector {
				Action::Transfer => FunctionModifier::NonPayable,
				_ => FunctionModifier::View,
			})?;

			return match selector {
				Action::Name => Self::name(asset_id, handle),
				Action::Symbol => Self::symbol(asset_id, handle),
				Action::Decimals => Self::decimals(asset_id, handle),
				Action::TotalSupply => Self::total_supply(asset_id, handle),
				Action::BalanceOf => Self::balance_of(asset_id, handle),
				Action::Transfer => Self::transfer(asset_id, handle),
				Action::Allowance => Self::not_supported(asset_id, handle),
				Action::Approve => Self::not_supported(asset_id, handle),
				Action::TransferFrom => Self::not_supported(asset_id, handle),
			};
		}
		Err(PrecompileFailure::Revert {
			exit_status: ExitRevert::Reverted,
			output: "invalid currency id".into(),
		})
	}
}

impl<Runtime> MultiCurrencyPrecompile<Runtime>
where
	Runtime: frame_system::Config
		+ pallet_evm::Config
		+ bifrost_asset_registry::Config
		+ bifrost_currencies::Config,
	Currencies: MultiCurrency<Runtime::AccountId, CurrencyId = CurrencyId, Balance = Balance>,
	bifrost_currencies::Pallet<Runtime>:
		MultiCurrency<Runtime::AccountId, CurrencyId = CurrencyId, Balance = Balance>,
	<Runtime as frame_system::Config>::AccountId: core::convert::From<sp_runtime::AccountId32>,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin: OriginTrait,
{
	fn name(currency_id: CurrencyId, handle: &mut impl PrecompileHandle) -> PrecompileResult {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input
		let input = handle.read_input()?;
		input.expect_arguments(0)?;

		match AssetIdMaps::<Runtime>::get_currency_metadata(currency_id) {
			Some(metadata) => {
				let encoded = Output::encode_bytes(metadata.name.as_slice());
				Ok(succeed(encoded))
			},
			None => Err(PrecompileFailure::Error {
				exit_status: pallet_evm::ExitError::Other("Non-existing asset.".into()),
			}),
		}
	}

	fn symbol(currency_id: CurrencyId, handle: &mut impl PrecompileHandle) -> PrecompileResult {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input
		let input = handle.read_input()?;
		input.expect_arguments(0)?;

		match AssetIdMaps::<Runtime>::get_currency_metadata(currency_id) {
			Some(metadata) => {
				let encoded = Output::encode_bytes(metadata.symbol.as_slice());
				Ok(succeed(encoded))
			},
			None => Err(PrecompileFailure::Error {
				exit_status: pallet_evm::ExitError::Other("Non-existing asset.".into()),
			}),
		}
	}

	fn decimals(currency_id: CurrencyId, handle: &mut impl PrecompileHandle) -> PrecompileResult {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input
		let input = handle.read_input()?;
		input.expect_arguments(0)?;

		match AssetIdMaps::<Runtime>::get_currency_metadata(currency_id) {
			Some(metadata) => {
				let encoded = Output::encode_uint::<u8>(metadata.decimals);
				Ok(succeed(encoded))
			},
			None => Err(PrecompileFailure::Error {
				exit_status: pallet_evm::ExitError::Other("Non-existing asset.".into()),
			}),
		}
	}

	fn total_supply(
		currency_id: CurrencyId,
		handle: &mut impl PrecompileHandle,
	) -> PrecompileResult {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input
		let input = handle.read_input()?;
		input.expect_arguments(0)?;

		let total_issuance = Currencies::total_issuance(currency_id);

		log::debug!(target: "evm", "multicurrency: totalSupply: {:?}", total_issuance);

		let encoded = Output::encode_uint::<u128>(total_issuance);

		Ok(succeed(encoded))
	}

	fn balance_of(currency_id: CurrencyId, handle: &mut impl PrecompileHandle) -> PrecompileResult {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input
		let mut input = handle.read_input()?;
		input.expect_arguments(1)?;

		let owner: H160 = input.read::<Address>()?.into();
		let who: Runtime::AccountId = ExtendedAddressMapping::into_account_id(owner).into(); //TODO: use pallet?

		let free_balance = Currencies::free_balance(currency_id, &who);

		log::debug!(target: "evm", "multicurrency: balanceOf: {:?}", free_balance);

		let encoded = Output::encode_uint::<u128>(free_balance);

		Ok(succeed(encoded))
	}

	fn transfer(currency_id: CurrencyId, handle: &mut impl PrecompileHandle) -> PrecompileResult {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input
		let mut input = handle.read_input()?;
		input.expect_arguments(2)?;

		let to: H160 = input.read::<Address>()?.into();
		let amount = input.read::<Balance>()?;

		let origin = ExtendedAddressMapping::into_account_id(handle.context().caller);
		let to = ExtendedAddressMapping::into_account_id(to);

		log::debug!(target: "evm", "multicurrency: transfer from: {:?}, to: {:?}, amount: {:?}", origin, to, amount);

		<bifrost_currencies::Pallet<Runtime> as MultiCurrency<Runtime::AccountId>>::transfer(
			currency_id,
			&(<sp_runtime::AccountId32 as Into<Runtime::AccountId>>::into(origin)),
			&(<sp_runtime::AccountId32 as Into<Runtime::AccountId>>::into(to)),
			amount,
		)
		.map_err(|e| PrecompileFailure::Revert {
			exit_status: ExitRevert::Reverted,
			output: Into::<&str>::into(e).as_bytes().to_vec(),
		})?;

		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}

	fn not_supported(_: CurrencyId, _: &mut impl PrecompileHandle) -> PrecompileResult {
		Err(PrecompileFailure::Error {
			exit_status: pallet_evm::ExitError::Other("not supported".into()),
		})
	}
}
