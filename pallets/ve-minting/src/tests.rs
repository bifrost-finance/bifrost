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

use crate::{mock::*, traits::VeMintingInterface, *};
use bifrost_asset_registry::AssetMetadata;
use bifrost_runtime_common::milli;
use frame_support::assert_ok;
use node_primitives::TokenInfo;

#[test]
fn _checkpoint() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);
		let old_locked = LockedBalance { amount: 0, end: 0 };
		let new_locked = LockedBalance {
			amount: 10000000000000,
			end: System::block_number() + 365 * 86400 * 1000 / 12,
		};

		assert_ok!(VeMinting::set_config(
			RuntimeOrigin::signed(ALICE),
			Some(0),
			Some(7 * 86400 * 1000 / 12),
			Some(4 * 365 * 86400 / 12),
			Some(10_u128.pow(18)),
			Some(7 * 86400 / 12),
			Some(0)
		));
		System::set_block_number(System::block_number() + 20);
		assert_ok!(VeMinting::_checkpoint(&BOB, old_locked, new_locked));
		assert_eq!(VeMinting::balance_of(&BOB, Some(System::block_number())), Ok(0));
	});
}

#[test]
fn update_reward() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);
		assert_ok!(VeMinting::set_config(
			RuntimeOrigin::signed(ALICE),
			Some(0),
			Some(7 * 86400 * 1000 / 12),
			Some(4 * 365 * 86400 * 1000 / 12),
			Some(10_u128.pow(12)),
			Some(7 * 86400 / 12),
			Some(0)
		));

		System::set_block_number(System::block_number() + 20);
		System::set_block_number(System::block_number() + 20);
		assert_ok!(VeMinting::_create_lock(
			&BOB,
			10000000000000,
			System::block_number() + 365 * 86400 * 1000 / 12,
		));
		assert_ok!(VeMinting::deposit_for(&BOB, 10000000000000));
		assert_ok!(VeMinting::update_reward(Some(&BOB)));

		assert_eq!(VeMinting::balance_of(&BOB, None), Ok(20000000000000));
		assert_eq!(VeMinting::balance_of(&BOB, Some(System::block_number())), Ok(20000000000000));
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

#[test]
fn notify_reward_amount() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);
		assert_ok!(VeMinting::set_config(
			RuntimeOrigin::signed(ALICE),
			Some(0),
			Some(7 * 86400 * 1000 / 12),
			Some(4 * 365 * 86400 * 1000 / 12),
			Some(10_u128.pow(12)),
			Some(7 * 86400 / 12),
			Some(3)
		));

		System::set_block_number(System::block_number() + 20);
		System::set_block_number(System::block_number() + 20);
		assert_ok!(VeMinting::_create_lock(
			&BOB,
			10000000000000,
			System::block_number() + 365 * 86400 * 1000 / 12
		));

		let rewards = vec![(KSM, 1000)];
		assert_ok!(VeMinting::notify_rewards(
			RuntimeOrigin::signed(ALICE),
			Some(7 * 86400),
			rewards
		));
		assert_ok!(VeMinting::deposit_for(&BOB, 10000000000000));
		assert_ok!(VeMinting::update_reward(Some(&BOB)));
	});
}
