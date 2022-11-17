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

#![cfg(test)]

use crate::{mock::*, *};
use bifrost_asset_registry::AssetMetadata;
use bifrost_runtime_common::milli;
use frame_support::{assert_noop, assert_ok, sp_runtime::Permill, BoundedVec};
use node_primitives::TokenInfo;

#[test]
fn _checkpoint() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		let old_locked = LockedBalance { amount: 0, end: 0 };
		let new_locked = LockedBalance { amount: 10000000000000, end: 1668752990696 };

		assert_ok!(VeMinting::set_config(
			Origin::signed(ALICE),
			Some(4 * 365 * 86400),
			Some(10_u128.pow(18)),
			Some(7 * 86400)
		));
		// assert_eq!(VeMinting::ve_configs(), VeConfig::default());
		// VeMinting::_checkpoint(&BOB, old_locked, new_locked);
		System::set_block_number(System::block_number() + 20);
		assert_ok!(VeMinting::_checkpoint(&BOB, old_locked, new_locked));
		// let mut u_point = Point::<BalanceOf<Runtime>, BlockNumberFor<Runtime>>::default();
		// assert_eq!(VeMinting::user_point_history(&BOB, U256::from(1)), u_point);
		let current_timestamp: Timestamp =
			sp_timestamp::InherentDataProvider::from_system_time().timestamp().as_millis();
		assert_eq!(VeMinting::balanceOf(&BOB, current_timestamp), 0);
	});
}

fn asset_registry() {
	let items = vec![(KSM, 10 * milli::<Runtime>(KSM)), (BNC, 10 * milli::<Runtime>(BNC))];
	for (currency_id, metadata) in items.iter().map(|(currency_id, minimal_balance)| {
		(
			currency_id,
			AssetMetadata {
				name: currency_id.name().map(|s| s.as_bytes().to_vec()).unwrap_or_default(),
				symbol: currency_id.symbol().map(|s| s.as_bytes().to_vec()).unwrap_or_default(),
				decimals: currency_id.decimals().unwrap_or_default(),
				minimal_balance: *minimal_balance,
			},
		)
	}) {
		AssetRegistry::do_register_metadata(*currency_id, &metadata).expect("Token register");
	}
}
