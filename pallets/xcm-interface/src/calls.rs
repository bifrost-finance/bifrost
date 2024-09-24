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

use parity_scale_codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
use sp_std::boxed::Box;
use xcm::{v4::WeightLimit, VersionedAssets, VersionedLocation};

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum PolkadotXcmCall {
	#[codec(index = 2)]
	ReserveTransferAssets(
		Box<VersionedLocation>,
		Box<VersionedLocation>,
		Box<VersionedAssets>,
		u32,
	),
	#[codec(index = 8)]
	LimitedReserveTransferAssets(
		Box<VersionedLocation>,
		Box<VersionedLocation>,
		Box<VersionedAssets>,
		u32,
		WeightLimit,
	),
	#[codec(index = 9)]
	LimitedTeleportAssets(
		Box<VersionedLocation>,
		Box<VersionedLocation>,
		Box<VersionedAssets>,
		u32,
		WeightLimit,
	),
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum RelaychainCall {
	#[codec(index = 99)]
	XcmPallet(PolkadotXcmCall),
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum AssetHubCall {
	#[codec(index = 31)]
	PolkadotXcm(PolkadotXcmCall),
}
