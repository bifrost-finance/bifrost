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
use frame_support::assert_ok;
use node_primitives::{TimeUnit, VtokenMintingOperator};
use sp_arithmetic::per_things::Permill;

#[test]
fn on_idle() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let para_id = 2001u32;
		let zenlink_pair_account_id = init_zenlink(para_id);

		assert_ok!(SystemMaker::set_config(
			RuntimeOrigin::signed(ALICE),
			RelayCurrencyId::get(),
			Info {
				vcurrency_id: vKSM,
				annualization: 600_000u32,
				granularity: 1000,
				minimum_redeem: 20000
			},
		));
		let system_maker =
			<Runtime as Config>::SystemMakerPalletId::get().into_account_truncating();
		assert_eq!(Tokens::free_balance(KSM, &system_maker), 10000);
		SystemMaker::on_idle(
			<frame_system::Pallet<Runtime>>::block_number(),
			Weight::from_ref_time(100000000),
		);
		System::set_block_number(System::block_number() + 1);
		assert_eq!(Tokens::free_balance(vKSM, &system_maker), 10731);
		assert_eq!(Tokens::free_balance(KSM, &zenlink_pair_account_id), 3000);
		assert_eq!(Tokens::free_balance(vKSM, &zenlink_pair_account_id), 1469);
		init_vtoken_minting();
		SystemMaker::on_idle(<frame_system::Pallet<Runtime>>::block_number(), Weight::zero());
		assert_eq!(Tokens::free_balance(vKSM, &system_maker), 10731);
		assert_ok!(SystemMaker::set_config(
			RuntimeOrigin::signed(ALICE),
			RelayCurrencyId::get(),
			Info {
				vcurrency_id: vKSM,
				annualization: 600_000u32,
				granularity: 1000,
				minimum_redeem: 2000
			},
		));
		SystemMaker::on_idle(<frame_system::Pallet<Runtime>>::block_number(), Weight::zero());
		assert_eq!(Tokens::free_balance(vKSM, &system_maker), 0);
	});
}

fn init_zenlink(para_id: u32) -> AccountIdOf<Runtime> {
	let asset_0_currency_id: AssetId =
		AssetId::try_convert_from(RelayCurrencyId::get(), para_id).unwrap();
	let asset_1_currency_id: AssetId = AssetId::try_convert_from(vKSM, para_id).unwrap();
	// let path = vec![asset_0_currency_id, asset_1_currency_id];
	assert_ok!(ZenlinkProtocol::create_pair(
		RuntimeOrigin::root(),
		asset_0_currency_id,
		asset_1_currency_id
	));
	let deadline: BlockNumberFor<Runtime> = <frame_system::Pallet<Runtime>>::block_number() +
		<Runtime as frame_system::Config>::BlockNumber::from(100u32);
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

fn init_vtoken_minting() {
	pub const FEE: Permill = Permill::from_percent(2);
	assert_ok!(VtokenMinting::set_fees(RuntimeOrigin::root(), FEE, FEE));
	assert_ok!(VtokenMinting::set_unlock_duration(
		RuntimeOrigin::signed(ALICE),
		KSM,
		TimeUnit::Era(1)
	));
	assert_ok!(VtokenMinting::increase_token_pool(KSM, 1000));
	assert_ok!(VtokenMinting::update_ongoing_time_unit(KSM, TimeUnit::Era(1)));
	assert_ok!(VtokenMinting::set_minimum_redeem(RuntimeOrigin::signed(ALICE), vKSM, 90));
}
