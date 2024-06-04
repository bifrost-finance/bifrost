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

// Ensure we're `no_std` when compiling for Wasm.

#![cfg(test)]

use crate::{mock::*, *};
use bifrost_primitives::{TimeUnit, VtokenMintingOperator};
use frame_support::assert_ok;
use sp_arithmetic::per_things::Permill;

#[test]
fn buy_back_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let para_id = 2001u32;
		let zenlink_pair_account_id = init_zenlink(para_id);

		assert_ok!(BuyBack::set_vtoken(
			RuntimeOrigin::signed(ALICE),
			VKSM,
			1_000_000u128,
			Permill::from_percent(2),
			1000,
			1000,
			true
		));
		let buyback_account = <Runtime as Config>::BuyBackAccount::get().into_account_truncating();
		assert_eq!(Tokens::free_balance(VKSM, &buyback_account), 10000);
		assert_eq!(Tokens::free_balance(VKSM, &zenlink_pair_account_id), 2200);
		assert_eq!(Balances::free_balance(&zenlink_pair_account_id), 2000);
		assert_eq!(Balances::free_balance(&buyback_account), 0);
		BuyBack::on_idle(
			<frame_system::Pallet<Runtime>>::block_number(),
			Weight::from_parts(100000000, 0),
		);
		System::set_block_number(System::block_number() + 1);
		assert_eq!(Tokens::free_balance(VKSM, &buyback_account), 0);
		assert_eq!(Tokens::free_balance(VKSM, &zenlink_pair_account_id), 12200);
		assert_eq!(Balances::free_balance(&zenlink_pair_account_id), 362);
		assert_eq!(Balances::free_balance(&buyback_account), 1638);
	});
}

fn init_zenlink(para_id: u32) -> AccountIdOf<Runtime> {
	let asset_0_currency_id: AssetId = AssetId::try_convert_from(BNC, para_id).unwrap();
	let asset_1_currency_id: AssetId = AssetId::try_convert_from(VKSM, para_id).unwrap();
	// let path = vec![asset_0_currency_id, asset_1_currency_id];
	assert_ok!(ZenlinkProtocol::create_pair(
		RuntimeOrigin::root(),
		asset_0_currency_id,
		asset_1_currency_id,
		ALICE
	));
	let deadline: BlockNumberFor<Runtime> =
		<frame_system::Pallet<Runtime>>::block_number() + BlockNumberFor::<Runtime>::from(100u32);
	assert_ok!(ZenlinkProtocol::add_liquidity(
		RuntimeOrigin::signed(ALICE),
		asset_0_currency_id,
		asset_1_currency_id,
		2000,
		2200,
		1,
		1,
		deadline
	));
	ZenlinkProtocol::pair_account_id(asset_0_currency_id, asset_1_currency_id)
}
