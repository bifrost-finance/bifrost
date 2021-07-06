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

use cumulus_primitives_core::ParaId as CumulusParaId;
use frame_support::weights::Weight;
use sp_std::vec::Vec;
use xcm::{
	v0::{prelude::XcmResult, MultiLocation},
	DoubleEncoded,
};

pub trait HandleUmpMessage {
	fn handle_ump_message(from: CumulusParaId, msg: &[u8], max_weight: Weight);
}

pub trait HandleDmpMessage {
	fn handle_dmp_message(at_relay_block: u32, msg: Vec<u8>, max_weight: Weight);
}

pub trait HandleXcmpMessage {
	fn handle_xcmp_message(
		from: CumulusParaId,
		at_relay_block: u32,
		msg: &[u8],
		max_weight: Weight,
	);
}

/// Bifrost Xcm Executor
pub trait BifrostXcmExecutor {
	fn ump_transact(origin: MultiLocation, call: DoubleEncoded<()>) -> XcmResult;

	fn ump_transfer_asset(
		origin: MultiLocation,
		dest: MultiLocation,
		amount: u128,
		relay: bool,
	) -> XcmResult;
}
