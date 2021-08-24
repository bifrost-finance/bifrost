// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

use codec::{Decode, Encode};
use sp_std::prelude::*;

/// The type used to represent the xcmp transfer direction
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub enum TransferOriginType {
	FromSelf = 0,
	FromRelayChain = 1,
	FromSiblingParaChain = 2,
}

pub struct XcmBaseWeight(u64);

impl From<u64> for XcmBaseWeight {
	fn from(u: u64) -> Self {
		XcmBaseWeight(u)
	}
}

impl From<XcmBaseWeight> for u64 {
	fn from(x: XcmBaseWeight) -> Self {
		x.0.into()
	}
}

/// represent the xcmp transact type
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub enum ParachainTransactProxyType {
	Primary = 0,
	Derived = 1,
}

#[repr(u16)]
pub enum ParachainDerivedProxyAccountType {
	Salp = 0,
	Staking = 1,
}
