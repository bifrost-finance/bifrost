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
use bifrost_asset_registry::AssetMetadata;
use bifrost_primitives::TokenInfo;
use bifrost_runtime_common::milli;
use frame_support::{assert_noop, assert_ok};
use sp_arithmetic::per_things::Permill;

const PARAID : u32 = 2001;
const VALUE : u128 = 1000;
const BUYBACK_DURATION : u64 = 1000;
const LIQUID_DURATION : u64 = 1000;
const LIQUID_PROPORTION : Permill = Permill::from_percent(2);

#[test]
fn set_vtoken_should_not_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		assert_noop!(
			BuyBack::set_vtoken(
				RuntimeOrigin::signed(ALICE),
				KSM,
				VALUE,
				LIQUID_PROPORTION,
				BUYBACK_DURATION,
				LIQUID_DURATION,
				true,
			),
			Error::<Runtime>::CurrencyIdError
		);

		assert_noop!(
			BuyBack::set_vtoken(
				RuntimeOrigin::signed(ALICE),
				VKSM,
				VALUE,
				LIQUID_PROPORTION,
				0,
				LIQUID_DURATION,
				true,
			),
			Error::<Runtime>::ZeroDuration
		);

		assert_noop!(
			BuyBack::set_vtoken(
				RuntimeOrigin::signed(ALICE),
				VKSM,
				0,
				LIQUID_PROPORTION,
				BUYBACK_DURATION,
				LIQUID_DURATION,
				true,
			),
			Error::<Runtime>::ZeroMinSwapValue
		);
	});
}

#[test]
fn buy_back_no_burn_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		let zenlink_pair_account_id = init_zenlink(PARAID);

		assert_ok!(BuyBack::set_vtoken(
			RuntimeOrigin::signed(ALICE),
			VKSM,
			VALUE,
			LIQUID_PROPORTION,
			BUYBACK_DURATION,
			LIQUID_DURATION,
			true,
		));
		let buyback_account = <Runtime as Config>::BuyBackAccount::get().into_account_truncating();
		let incentive_account = IncentivePalletId::get().into_account_truncating();
		assert_eq!(Currencies::free_balance(VKSM, &buyback_account), 9000);
		assert_eq!(Currencies::free_balance(VKSM, &zenlink_pair_account_id), 2200);
		assert_eq!(Currencies::free_balance(BNC, &zenlink_pair_account_id), 2000);
		assert_eq!(Currencies::free_balance(BNC, &buyback_account), 0);
		assert_eq!(Currencies::free_balance(BNC, &incentive_account), 0);
		VeMinting::set_incentive(0, Some(7 * 86400 / 12), Some(buyback_account.clone()));
		assert_ok!(BuyBack::charge(RuntimeOrigin::signed(ALICE), VKSM, 1000));
		let infos = Infos::<Runtime>::get(VKSM).unwrap();
		assert_ok!(BuyBack::buy_back(&buyback_account, VKSM, &infos));
		System::set_block_number(System::block_number() + 1);
		assert_eq!(Currencies::free_balance(VKSM, &buyback_account), 9000);
		assert_eq!(Currencies::free_balance(VKSM, &zenlink_pair_account_id), 3200);
		assert_eq!(Currencies::free_balance(BNC, &zenlink_pair_account_id), 1377);
		assert_eq!(Currencies::free_balance(BNC, &buyback_account), 0);
		assert_eq!(Currencies::free_balance(BNC, &incentive_account), 623);
	});
}

#[test]
fn on_idle_no_burn_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		let zenlink_pair_account_id = init_zenlink(PARAID);

		assert_ok!(BuyBack::set_vtoken(
			RuntimeOrigin::signed(ALICE),
			VKSM,
			1_000_000u128,
			LIQUID_PROPORTION,
			BUYBACK_DURATION,
			LIQUID_DURATION,
			true,
		));
		let buyback_account = <Runtime as Config>::BuyBackAccount::get().into_account_truncating();
		let incentive_account = IncentivePalletId::get().into_account_truncating();
		assert_eq!(Currencies::free_balance(VKSM, &buyback_account), 9000);
		assert_eq!(Currencies::free_balance(VKSM, &zenlink_pair_account_id), 2200);
		assert_eq!(Currencies::free_balance(BNC, &zenlink_pair_account_id), 2000);
		assert_eq!(Currencies::free_balance(BNC, &buyback_account), 0);
		assert_eq!(Currencies::free_balance(BNC, &incentive_account), 0);
		VeMinting::set_incentive(0, Some(7 * 86400 / 12), Some(buyback_account.clone()));
		assert_ok!(BuyBack::charge(RuntimeOrigin::signed(ALICE), VKSM, 1000));
		BuyBack::on_idle(
			<frame_system::Pallet<Runtime>>::block_number(),
			Weight::from_parts(100000000, 0),
		);
		System::set_block_number(System::block_number() + 1);
		assert_eq!(Currencies::free_balance(VKSM, &buyback_account), 0);
		assert_eq!(Currencies::free_balance(VKSM, &zenlink_pair_account_id), 12200);
		assert_eq!(Currencies::free_balance(BNC, &zenlink_pair_account_id), 362);
		assert_eq!(Currencies::free_balance(BNC, &buyback_account), 0);
		assert_eq!(Currencies::free_balance(BNC, &incentive_account), 1638);
	});
}

fn init_zenlink(_para_id: u32) -> AccountIdOf<Runtime> {
	let asset_0_currency_id: AssetId = AssetId::try_convert_from(BNC, PARAID).unwrap();
	let asset_1_currency_id: AssetId = AssetId::try_convert_from(VKSM, PARAID).unwrap();
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

fn asset_registry() {
	let items = vec![(VKSM, 10 * milli::<Runtime>(VKSM))];
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
