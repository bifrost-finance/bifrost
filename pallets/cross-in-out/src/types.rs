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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use crate::{Convert, Junction, MultiLocation, X1};
use codec::{Decode, Encode};
use frame_support::{pallet_prelude::PhantomData, RuntimeDebug};
use scale_info::TypeInfo;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct SignatureStruct {
	pub sig_type: SigType,
	pub bytes: Vec<u8>,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum SigType {
	FilecoinSecp256k1,
	FilecoinBLS,
}

struct ForeignAccountIdConverter<T>(PhantomData<T>);
impl<T> Convert<Box<MultiLocation>, Option<Vec<u8>>> for ForeignAccountIdConverter<T> {
	fn convert(location: Box<MultiLocation>) -> Option<Vec<u8>> {
		match *location {
			MultiLocation { parents: 100, interior: X1(Junction::GeneralKey(key)) } =>
				Some(key.to_vec()),
			_ => None,
		}
	}
}
