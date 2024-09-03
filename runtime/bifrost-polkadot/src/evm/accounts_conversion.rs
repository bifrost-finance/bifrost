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
#![allow(unused_imports)]
use crate::{
	evm::{ConsensusEngineId, FindAuthor},
	AccountId, Aura, EVMAccounts, Runtime,
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
			let authority_id =
				pallet_aura::Authorities::<Runtime>::get()[author_index as usize].clone();
			return Some(H160::from_slice(&authority_id.to_raw_vec()[4..24]));
		}
		None
	}
}
