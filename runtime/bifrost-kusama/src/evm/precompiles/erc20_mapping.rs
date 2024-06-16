//                    :                     $$\   $$\                 $$\
// $$$$$$$\  $$\   $$\                  !YJJ^                   $$ |  $$ |                $$ |
// $$  __$$\ $$ |  $$ |                7B5. ~B5^                 $$ |  $$ |$$\   $$\  $$$$$$$ |
// $$$$$$\  $$$$$$\  $$ |  $$ |\$$\ $$  |             .?B@G    ~@@P~               $$$$$$$$ |$$ |
// $$ |$$  __$$ |$$  __$$\ \____$$\ $$ |  $$ | \$$$$  /           :?#@@@Y    .&@@@P!.            $$
// __$$ |$$ |  $$ |$$ /  $$ |$$ |  \__|$$$$$$$ |$$ |  $$ | $$  $$<         ^?J^7P&@@!  .5@@#Y~!J!.
// $$ |  $$ |$$ |  $$ |$$ |  $$ |$$ |     $$  __$$ |$$ |  $$ |$$  /\$$\       ^JJ!.   :!J5^ ?5?^
// ^?Y7.        $$ |  $$ |\$$$$$$$ |\$$$$$$$ |$$ |     \$$$$$$$ |$$$$$$$  |$$ /  $$ |     ~PP: 7#B5!
// .         :?P#G: 7G?.      \__|  \__| \____$$ | \_______|\__|      \_______|\_______/ \__|  \__|
//  .!P@G    7@@@#Y^    .!P@@@#.   ~@&J:              $$\   $$ |
//  !&@@J    :&@@@@P.   !&@@@@5     #@@P.             \$$$$$$  |
//   :J##:   Y@@&P!      :JB@@&~   ?@G!                \______/
//     .?P!.?GY7:   .. .    ^?PP^:JP~
//       .7Y7.  .!YGP^ ?BP?^   ^JJ^         This file is part of https://github.com/galacticcouncil/HydraDX-node
//         .!Y7Y#@@#:   ?@@@G?JJ^           Built with <3 for decentralisation.
//            !G@@@Y    .&@@&J:
//              ^5@#.   7@#?.               Copyright (C) 2021-2023  Intergalactic, Limited (GIB).
//                :5P^.?G7.                 SPDX-License-Identifier: Apache-2.0
//                  :?Y!                    Licensed under the Apache License, Version 2.0 (the
// "License");                                          you may not use this file except in
// compliance with the License.                                          http://www.apache.org/licenses/LICENSE-2.0

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

pub struct HydraErc20Mapping;

/// Erc20Mapping logic for HydraDX
/// The asset id (with type u32) is encoded in the last 4 bytes of EVM address
impl Erc20Mapping for HydraErc20Mapping {
	fn encode_evm_address(currency_id: CurrencyId) -> Option<EvmAddress> {
		let asset_id_bytes = currency_id.encode();

		let mut evm_address_bytes = [0u8; 20];

		evm_address_bytes[0..4].copy_from_slice(CURRENCY_PRECOMPILE_ADDRESS_PREFIX);
		evm_address_bytes[18..].copy_from_slice(asset_id_bytes.as_slice());

		Some(EvmAddress::from(evm_address_bytes))
	}

	fn decode_evm_address(evm_address: EvmAddress) -> Option<CurrencyId> {
		if !is_asset_address(evm_address) {
			return None;
		}

		let mut currency_id = &evm_address.to_fixed_bytes()[18..];
		CurrencyId::decode(&mut currency_id).ok()
	}
}

pub fn is_asset_address(address: H160) -> bool {
	&address.to_fixed_bytes()[0..4] == CURRENCY_PRECOMPILE_ADDRESS_PREFIX
}
