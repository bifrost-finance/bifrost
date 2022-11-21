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
#![cfg(test)]
#![allow(unused_imports)]
use crate::{mock::*, *};
use bifrost_asset_registry::AssetMetadata;
use bifrost_runtime_common::milli;
use frame_support::{
	assert_ok,
	sp_runtime::{Perbill, Permill},
};
use frame_system::pallet_prelude::BlockNumberFor;
use node_primitives::{TimeUnit, TokenInfo, TryConvertFrom, VtokenMintingOperator};
use sp_runtime::traits::AccountIdConversion;
use sp_std::{collections::btree_map::BTreeMap, prelude::*};
use zenlink_protocol::AssetId;

#[test]
fn test_xcm_action() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		assert_ok!(VtokenMinting::set_minimum_mint(Origin::signed(ALICE), KSM, 10));
		pub const FEE: Permill = Permill::from_percent(5);
		assert_ok!(VtokenMinting::set_fees(Origin::root(), FEE, FEE));
		assert_ok!(VtokenMinting::set_unlock_duration(
			Origin::signed(ALICE),
			KSM,
			TimeUnit::Era(1)
		));
		assert_ok!(VtokenMinting::increase_token_pool(KSM, 1000));
		assert_ok!(VtokenMinting::update_ongoing_time_unit(KSM, TimeUnit::Era(1)));
		assert_ok!(VtokenMinting::set_minimum_redeem(Origin::signed(ALICE), vKSM, 10));

		init_zenlink(2001u32);

		// let weight = 4_000_000_000u64;
		// let addr: [u8; 20] =
		// 	hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();
		// let receiver = H160::from(addr);
		// assert_ok!(XcmAction::mint(Origin::signed(ALICE), receiver, KSM, 1000, weight));

		// assert_ok!(XcmAction::redeem(Origin::signed(ALICE), vKSM, 100));

		// assert_ok!(XcmAction::swap(
		// 	Origin::signed(ALICE),
		// 	receiver,
		// 	1,
		// 	1,
		// 	KSM,
		// 	vKSM,
		// ));
	});
}

fn asset_registry() {
	let items = vec![(KSM, 10 * milli::<Test>(KSM))];
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

fn init_zenlink(para_id: u32) -> AccountIdOf<Test> {
	let asset_0_currency_id: AssetId =
		AssetId::try_convert_from(RelayCurrencyId::get(), para_id).unwrap();
	let asset_1_currency_id: AssetId = AssetId::try_convert_from(vKSM, para_id).unwrap();
	// let path = vec![asset_0_currency_id, asset_1_currency_id];
	assert_ok!(ZenlinkProtocol::create_pair(
		Origin::root(),
		asset_0_currency_id,
		asset_1_currency_id
	));
	let deadline: BlockNumberFor<Test> = <frame_system::Pallet<Test>>::block_number() +
		<Test as frame_system::Config>::BlockNumber::from(100u32);
	assert_ok!(ZenlinkProtocol::add_liquidity(
		Origin::signed(ALICE),
		asset_0_currency_id,
		asset_1_currency_id,
		50,
		50,
		1,
		1,
		deadline
	));
	ZenlinkProtocol::pair_account_id(asset_0_currency_id, asset_1_currency_id)
}
