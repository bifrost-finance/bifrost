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

use core::marker::PhantomData;

use crate::evm::precompiles::{
	erc20_mapping::is_asset_address, multicurrency::MultiCurrencyPrecompile,
};
use ethabi::Token;
use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo};
use hex_literal::hex;
use pallet_evm::{
	ExitRevert, ExitSucceed, IsPrecompileResult, Precompile, PrecompileFailure, PrecompileHandle,
	PrecompileOutput, PrecompileResult, PrecompileSet,
};
use pallet_evm_precompile_blake2::Blake2F;
use pallet_evm_precompile_bn128::{Bn128Add, Bn128Mul, Bn128Pairing};
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_simple::{ECRecover, Identity, Ripemd160, Sha256};
use parity_scale_codec::Decode;
use primitive_types::{H160, U256};
use sp_runtime::traits::Dispatchable;
use sp_std::{borrow::ToOwned, vec::Vec};

pub mod costs;
pub mod erc20_mapping;
pub mod handle;
pub mod multicurrency;
pub mod substrate;

pub type EvmResult<T = ()> = Result<T, PrecompileFailure>;

#[cfg(test)]
mod tests;

pub type EvmAddress = H160;

/// The `address` type of Solidity.
/// H160 could represent 2 types of data (bytes20 and address) that are not encoded the same way.
/// To avoid issues writing H160 is thus not supported.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Address(pub H160);

impl From<H160> for Address {
	fn from(a: H160) -> Address {
		Address(a)
	}
}

impl From<Address> for H160 {
	fn from(a: Address) -> H160 {
		a.0
	}
}

pub struct BifrostPrecompiles<R>(PhantomData<R>);

impl<R> BifrostPrecompiles<R> {
	#[allow(clippy::new_without_default)] // We'll never use Default and can't derive it.
	pub fn new() -> Self {
		Self(Default::default())
	}
}

// Same as Moonbean and Centrifuge, should benefit interoperability
// See also
// https://docs.moonbeam.network/builders/pallets-precompiles/precompiles/overview/#precompiled-contract-addresses
const DISPATCH_ADDR: H160 = addr(1025);

pub const ECRECOVER: H160 = H160(hex!("0000000000000000000000000000000000000001"));
pub const SHA256: H160 = H160(hex!("0000000000000000000000000000000000000002"));
pub const RIPEMD: H160 = H160(hex!("0000000000000000000000000000000000000003"));
pub const IDENTITY: H160 = H160(hex!("0000000000000000000000000000000000000004"));
pub const MODEXP: H160 = H160(hex!("0000000000000000000000000000000000000005"));
pub const BN_ADD: H160 = H160(hex!("0000000000000000000000000000000000000006"));
pub const BN_MUL: H160 = H160(hex!("0000000000000000000000000000000000000007"));
pub const BN_PAIRING: H160 = H160(hex!("0000000000000000000000000000000000000008"));
pub const BLAKE2F: H160 = H160(hex!("0000000000000000000000000000000000000009"));

pub const ETH_PRECOMPILE_END: H160 = BLAKE2F;

fn is_standard_precompile(address: H160) -> bool {
	!address.is_zero() && address <= ETH_PRECOMPILE_END
}

impl<R> PrecompileSet for BifrostPrecompiles<R>
where
	R: pallet_evm::Config + bifrost_currencies::Config,
	R::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + Decode,
	<R::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<R::AccountId>>,
	MultiCurrencyPrecompile<R>: Precompile,
{
	fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
		let context = handle.context();
		let address = handle.code_address();

		// Filter known precompile addresses except Ethereum officials
		if address > ETH_PRECOMPILE_END && context.address != address {
			return Some(Err(PrecompileFailure::Revert {
				exit_status: ExitRevert::Reverted,
				output: "cannot be called with DELEGATECALL or CALLCODE".into(),
			}));
		}

		if address == ECRECOVER {
			Some(ECRecover::execute(handle))
		} else if address == SHA256 {
			Some(Sha256::execute(handle))
		} else if address == RIPEMD {
			Some(Ripemd160::execute(handle))
		} else if address == IDENTITY {
			Some(Identity::execute(handle))
		} else if address == MODEXP {
			Some(Modexp::execute(handle))
		} else if address == BN_ADD {
			Some(Bn128Add::execute(handle))
		} else if address == BN_MUL {
			Some(Bn128Mul::execute(handle))
		} else if address == BN_PAIRING {
			Some(Bn128Pairing::execute(handle))
		} else if address == BLAKE2F {
			Some(Blake2F::execute(handle))
		} else if address == DISPATCH_ADDR {
			Some(pallet_evm_precompile_dispatch::Dispatch::<R>::execute(handle))
		} else if is_asset_address(address) {
			Some(MultiCurrencyPrecompile::<R>::execute(handle))
		} else {
			None
		}
	}

	fn is_precompile(&self, address: H160, _remaining_gas: u64) -> IsPrecompileResult {
		let is_precompile = address == DISPATCH_ADDR ||
			is_asset_address(address) ||
			is_standard_precompile(address);
		IsPrecompileResult::Answer { is_precompile, extra_cost: 0 }
	}
}

// This is a reimplementation of the upstream u64->H160 conversion
// function, made `const` to make our precompile address `const`s a
// bit cleaner. It can be removed when upstream has a const conversion
// function.
pub const fn addr(a: u64) -> H160 {
	let b = a.to_be_bytes();
	H160([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

pub fn revert(output: impl AsRef<[u8]>) -> PrecompileFailure {
	PrecompileFailure::Revert {
		exit_status: ExitRevert::Reverted,
		output: output.as_ref().to_owned(),
	}
}

pub fn succeed(output: impl AsRef<[u8]>) -> PrecompileOutput {
	PrecompileOutput { exit_status: ExitSucceed::Returned, output: output.as_ref().to_owned() }
}

pub struct Output;

impl Output {
	pub fn encode_uint<T>(b: T) -> Vec<u8>
	where
		U256: From<T>,
	{
		ethabi::encode(&[Token::Uint(U256::from(b))])
	}

	pub fn encode_bytes(b: &[u8]) -> Vec<u8> {
		ethabi::encode(&[Token::Bytes(b.to_vec())])
	}
}

/// The `bytes`/`string` type of Solidity.
/// It is different from `Vec<u8>` which will be serialized with padding for each `u8` element
/// of the array, while `Bytes` is tightly packed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bytes(pub Vec<u8>);

impl Bytes {
	/// Interpret as `bytes`.
	pub fn as_bytes(&self) -> &[u8] {
		&self.0
	}

	/// Interpret as `string`.
	/// Can fail if the string is not valid UTF8.
	pub fn as_str(&self) -> Result<&str, sp_std::str::Utf8Error> {
		sp_std::str::from_utf8(&self.0)
	}
}

impl From<&[u8]> for Bytes {
	fn from(a: &[u8]) -> Self {
		Self(a.to_owned())
	}
}

impl From<&str> for Bytes {
	fn from(a: &str) -> Self {
		a.as_bytes().into()
	}
}

impl From<Bytes> for Vec<u8> {
	fn from(val: Bytes) -> Self {
		val.0
	}
}
