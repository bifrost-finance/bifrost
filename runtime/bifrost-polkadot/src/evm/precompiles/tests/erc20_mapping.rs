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

use crate::evm::precompiles::erc20_mapping::{BifrostErc20Mapping, Erc20Mapping};
use bifrost_primitives::{
	CurrencyId,
	TokenSymbol::{BNC, KSM},
};
use hex_literal::hex;
use parity_scale_codec::{Decode, Encode};
use primitive_types::H160;
use sp_core::bytes::to_hex;

macro_rules! encode {
	($asset_id:expr) => {{
		BifrostErc20Mapping::encode_evm_address($asset_id).unwrap()
	}};
}

macro_rules! decode {
	($evm_address:expr) => {{
		BifrostErc20Mapping::decode_evm_address(H160::from($evm_address)).unwrap()
	}};
}

macro_rules! decode_optional {
	($evm_address:expr) => {{
		BifrostErc20Mapping::decode_evm_address(H160::from($evm_address))
	}};
}

#[test]
fn decode_asset_id_from_evm_address_should_work() {
	assert_eq!(decode!(hex!("ffffffff00000000000000000000000000000001")), CurrencyId::Native(BNC));
	assert_eq!(decode!(hex!("ffffffff00000000000000000000000000000800")), CurrencyId::Token2(0));
	assert_eq!(decode!(hex!("ffffffff00000000000000000000000000000900")), CurrencyId::VToken2(0));
	assert_eq!(decode!(hex!("ffffffff00000000000000000000000000000204")), CurrencyId::Token(KSM));
	assert_eq!(decode!(hex!("ffffffff00000000000000000000000000000104")), CurrencyId::VToken(KSM));
}

#[test]
fn decode_asset_id_from_evm_address_should_not_work_with_invalid_asset_addresses() {
	assert_eq!(decode_optional!(hex!("0000000000000000000000000000000200000000")), None);
	assert_eq!(decode_optional!(hex!("0000000000000000000000000000000000000001")), None);
	assert_eq!(decode_optional!(hex!("90000000000000000000000000000001ffffffff")), None);
	assert_eq!(decode_optional!(hex!("0000000000000000000000000000001100000003")), None);
	assert_eq!(decode_optional!(hex!("0000000000000000900000000000000100000003")), None);
	assert_eq!(decode_optional!(hex!("7777777777777777777777777777777777777777")), None);
}

#[test]
fn encode_asset_id_to_evm_address_should_work() {
	assert_eq!(
		encode!(CurrencyId::Native(BNC)),
		H160::from(hex!("ffffffff00000000000000000000000000000001"))
	);
	assert_eq!(
		encode!(CurrencyId::Token2(0)),
		H160::from(hex!("ffffffff00000000000000000000000000000800"))
	);
	assert_eq!(
		encode!(CurrencyId::VToken2(0)),
		H160::from(hex!("ffffffff00000000000000000000000000000900"))
	);
	assert_eq!(
		encode!(CurrencyId::Token(KSM)),
		H160::from(hex!("ffffffff00000000000000000000000000000204"))
	);
	assert_eq!(
		encode!(CurrencyId::VToken(KSM)),
		H160::from(hex!("ffffffff00000000000000000000000000000104"))
	);
}
