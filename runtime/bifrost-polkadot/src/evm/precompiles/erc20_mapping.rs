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

use crate::evm::precompiles::EvmAddress;
use bifrost_primitives::CurrencyId;
use parity_scale_codec::{Decode, Encode};
use primitive_types::H160;

pub const CURRENCY_PRECOMPILE_ADDRESS_PREFIX: &[u8] = &[255u8; 4];

/// A mapping between AssetId and Erc20 EVM address.
pub trait Erc20Mapping {
	fn encode_evm_address(currency_id: CurrencyId) -> Option<EvmAddress>;

	fn decode_evm_address(evm_address: EvmAddress) -> Option<CurrencyId>;
}

pub struct BifrostErc20Mapping;

/// Erc20Mapping logic for HydraDX
/// The asset id (with type u32) is encoded in the last 4 bytes of EVM address
impl Erc20Mapping for BifrostErc20Mapping {
	fn encode_evm_address(currency_id: CurrencyId) -> Option<EvmAddress> {
		let asset_id_bytes = currency_id.encode();

		let mut evm_address_bytes = [0u8; 20];

		evm_address_bytes[0..4].copy_from_slice(CURRENCY_PRECOMPILE_ADDRESS_PREFIX);
		match currency_id {
			CurrencyId::VSBond(..) | CurrencyId::VSBond2(..) =>
				evm_address_bytes[6..].copy_from_slice(asset_id_bytes.as_slice()),
			CurrencyId::LPToken(..) =>
				evm_address_bytes[15..].copy_from_slice(asset_id_bytes.as_slice()),
			_ => evm_address_bytes[18..].copy_from_slice(asset_id_bytes.as_slice()),
		};

		Some(EvmAddress::from(evm_address_bytes))
	}

	fn decode_evm_address(evm_address: EvmAddress) -> Option<CurrencyId> {
		if !is_asset_address(evm_address) {
			return None;
		}

		let mut currency_id = &evm_address.to_fixed_bytes()[6..];
		if !currency_id.to_vec().starts_with(&[0, 0]) {
			return CurrencyId::decode(&mut currency_id).ok();
		};

		let mut currency_id = &evm_address.to_fixed_bytes()[15..];
		if !currency_id.to_vec().starts_with(&[0, 0]) {
			return CurrencyId::decode(&mut currency_id).ok();
		};

		let mut currency_id = &evm_address.to_fixed_bytes()[18..];
		CurrencyId::decode(&mut currency_id).ok()
	}
}

pub fn is_asset_address(address: H160) -> bool {
	&address.to_fixed_bytes()[0..4] == CURRENCY_PRECOMPILE_ADDRESS_PREFIX
}
