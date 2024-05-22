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
#![allow(unused_imports)]
use crate::{
	evm::{ConsensusEngineId, FindAuthor},
	AccountId, Aura, EVMAccounts,
};
use core::marker::PhantomData;
use frame_support::traits::IsType;
use hex_literal::hex;
use pallet_evm::AddressMapping;
use pallet_traits::evm::InspectEvmAccounts;
use parity_scale_codec::{Decode, Encode};
use sp_core::{crypto::ByteArray, H160};
use sp_runtime::traits::AccountIdConversion;

pub struct ExtendedAddressMapping;

impl AddressMapping<AccountId> for ExtendedAddressMapping {
	fn into_account_id(address: H160) -> AccountId {
		EVMAccounts::account_id(address)
	}
}

// Ethereum-compatible blocks author (20 bytes)
// Converted by truncating from Substrate author (32 bytes)
pub struct FindAuthorTruncated<F>(PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for FindAuthorTruncated<F> {
	fn find_author<'a, I>(digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		if let Some(author_index) = F::find_author(digests) {
			let authority_id = Aura::authorities()[author_index as usize].clone();
			return Some(H160::from_slice(&authority_id.to_raw_vec()[4..24]));
		}
		None
	}
}
