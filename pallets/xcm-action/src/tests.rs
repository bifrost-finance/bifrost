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

use crate::{mock::*, *};
use bifrost_asset_registry::AssetMetadata;
use bifrost_runtime_common::milli;
use frame_support::{
	assert_ok,
	sp_runtime::{Perbill, Permill},
};
use frame_system::pallet_prelude::BlockNumberFor;
use hex_literal::hex;
use node_primitives::{TimeUnit, TokenInfo, TokenSymbol, TryConvertFrom, VtokenMintingOperator};
use sp_runtime::traits::AccountIdConversion;
use sp_std::{collections::btree_map::BTreeMap, prelude::*};
use xcm::VersionedMultiLocation;
use zenlink_protocol::AssetId;

const EVM_ADDR: [u8; 20] = hex!["573394b77fC17F91E9E67F147A9ECe24d67C5073"];

// #[test]
// fn vtoken_convert_error() {
// 	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
// 		asset_registry();
// 		assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(ALICE), KSM, 10));
// 		assert_eq!(Currencies::free_balance(CurrencyId::Token(TokenSymbol::KSM), &ALICE), 30000);
// 		assert_eq!(Currencies::free_balance(CurrencyId::VToken(TokenSymbol::KSM), &ALICE), 30000);
// 		let address = H160::from_slice(&EVM_ADDR);
// 		let account = XcmAction::h160_to_account_id(address).unwrap();
// 		assert_ok!(XcmAction::mint(
// 			RuntimeOrigin::signed(ALICE),
// 			vKSM,
// 			10,
// 			TargetChain::Astar,
// 			address
// 		));

// 		// ALice vKSM balance ===> 0
// 		// account vKSM balnace ===> 30000
// 		assert_eq!(Currencies::free_balance(vKSM, &ALICE), 0);
// 		assert_eq!(Currencies::free_balance(vKSM, &account), 30000);
// 	});
// }

// #[test]
// fn vtoken_mint_error() {
// 	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
// 		asset_registry();
// 		assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(ALICE), KSM, 10));
// 		assert_eq!(Currencies::free_balance(CurrencyId::Token(TokenSymbol::KSM), &ALICE), 30000);
// 		assert_eq!(Currencies::free_balance(CurrencyId::VToken(TokenSymbol::KSM), &ALICE), 30000);
// 		let address = H160::from_slice(&EVM_ADDR);
// 		let account = XcmAction::h160_to_account_id(address).unwrap();
// 		assert_ok!(XcmAction::mint(
// 			RuntimeOrigin::signed(ALICE),
// 			KSM,
// 			5,
// 			TargetChain::Astar,
// 			address
// 		));

// 		// ALice KSM balance ===> 0
// 		// account KSM balnace ===> 30000
// 		assert_eq!(Currencies::free_balance(KSM, &ALICE), 0);
// 		assert_eq!(Currencies::free_balance(KSM, &account), 30000);
// 	});
// }

// #[test]
// fn swap_error() {
// 	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
// 		asset_registry();
// 		assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(ALICE), KSM, 10));
// 		assert_eq!(Currencies::free_balance(CurrencyId::Token(TokenSymbol::KSM), &ALICE), 30000);
// 		assert_eq!(Currencies::free_balance(CurrencyId::VToken(TokenSymbol::KSM), &ALICE), 30000);

// 		init_zenlink(2001);
// 		let address = H160::from_slice(&EVM_ADDR);
// 		let account = XcmAction::h160_to_account_id(address);
// 		XcmAction::swap(RuntimeOrigin::signed(ALICE), KSM, vKSM, TargetChain::Astar, address);

// 		// ALice KSM balance ===> 0
// 		// account KSM balnace ===> 30000
// 		assert_eq!(Currencies::free_balance(KSM, &ALICE), 0);
// 		assert_eq!(Currencies::free_balance(KSM, &account), 30000);
// 	});
// }

#[test]
fn test_xcm_action_util() {
	ExtBuilder::default().build().execute_with(|| {
		let address = H160::from_slice(&EVM_ADDR);
		let account_id = XcmAction::h160_to_account_id(address);
		assert_eq!(
			account_id,
			sp_runtime::AccountId32::new(hex!(
				"b1c2dde9e562a738e264a554e467b30e5cd58e95ab98459946fb8e518cfe71c2"
			))
		);
		let public_key: [u8; 32] = account_id.encode().try_into().unwrap();
		assert_eq!(
			public_key,
			hex!("b1c2dde9e562a738e264a554e467b30e5cd58e95ab98459946fb8e518cfe71c2")
		);
	});
}

fn asset_registry() {
	let items = vec![(KSM, 10 * milli::<Test>(KSM)), (vKSM, 10 * milli::<Test>(vKSM))];
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
		RuntimeOrigin::root(),
		asset_0_currency_id,
		asset_1_currency_id
	));
	let deadline: BlockNumberFor<Test> = <frame_system::Pallet<Test>>::block_number() +
		<Test as frame_system::Config>::BlockNumber::from(100u32);
	assert_ok!(ZenlinkProtocol::add_liquidity(
		RuntimeOrigin::signed(ALICE),
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
