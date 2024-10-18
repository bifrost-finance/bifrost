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
use bifrost_primitives::IncentivePalletId;
use frame_support::{assert_noop, assert_ok};
use sp_arithmetic::per_things::Permill;

const PARAID: u32 = 2001;
const VALUE: u128 = 1000;
const BUYBACK_DURATION: u64 = 2;
const LIQUID_DURATION: u64 = 1000;
const LIQUID_PROPORTION: Permill = Permill::from_percent(2);

#[test]
fn set_vtoken_should_not_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let destruction_ratio = Some(Permill::from_percent(2));
		let bias: Permill = Permill::from_percent(10);
		assert_noop!(
			BuyBack::set_vtoken(
				RuntimeOrigin::signed(ALICE),
				KSM,
				VALUE,
				LIQUID_PROPORTION,
				BUYBACK_DURATION,
				LIQUID_DURATION,
				true,
				destruction_ratio,
				bias
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
				destruction_ratio,
				bias
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
				destruction_ratio,
				bias
			),
			Error::<Runtime>::ZeroMinSwapValue
		);
	});
}

#[test]
fn buy_back_with_burn_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let zenlink_pair_account_id = init_zenlink(PARAID);
		let destruction_ratio = Some(Permill::from_percent(2));
		let bias: Permill = Permill::from_percent(10);

		assert_ok!(BuyBack::set_vtoken(
			RuntimeOrigin::signed(ALICE),
			VKSM,
			VALUE,
			LIQUID_PROPORTION,
			BUYBACK_DURATION,
			LIQUID_DURATION,
			true,
			destruction_ratio,
			bias
		));
		let buyback_account = <Runtime as Config>::BuyBackAccount::get().into_account_truncating();
		let incentive_account = IncentivePalletId::get().into_account_truncating();
		assert_eq!(Currencies::free_balance(VKSM, &buyback_account), 9000);
		assert_eq!(Currencies::free_balance(VKSM, &zenlink_pair_account_id), 2200);
		assert_eq!(Currencies::free_balance(BNC, &zenlink_pair_account_id), 2000);
		assert_eq!(Currencies::free_balance(BNC, &buyback_account), 0);
		assert_eq!(Currencies::free_balance(BNC, &incentive_account), 0);
		BbBNC::set_incentive(
			BB_BNC_SYSTEM_POOL_ID,
			Some(7 * 86400 / 12),
			Some(buyback_account.clone()),
		);
		assert_ok!(BuyBack::charge(RuntimeOrigin::signed(ALICE), VKSM, 1000));
		let infos = Infos::<Runtime>::get(VKSM).unwrap();
		assert_ok!(BuyBack::buy_back(&buyback_account, VKSM, &infos, 0));
		System::set_block_number(System::block_number() + 1);
		assert_eq!(Currencies::free_balance(VKSM, &buyback_account), 9000);
		assert_eq!(Currencies::free_balance(VKSM, &zenlink_pair_account_id), 3200);
		assert_eq!(Currencies::free_balance(BNC, &zenlink_pair_account_id), 1377);
		assert_eq!(Currencies::free_balance(BNC, &buyback_account), 0);
		assert_eq!(Currencies::free_balance(BNC, &incentive_account), 611);
	});
}

#[test]
fn buy_back_no_burn_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let zenlink_pair_account_id = init_zenlink(PARAID);
		let destruction_ratio = Some(Permill::from_percent(0));
		let bias: Permill = Permill::from_percent(10);

		assert_ok!(BuyBack::set_vtoken(
			RuntimeOrigin::signed(ALICE),
			VKSM,
			VALUE,
			LIQUID_PROPORTION,
			BUYBACK_DURATION,
			LIQUID_DURATION,
			true,
			destruction_ratio,
			bias
		));
		let buyback_account = <Runtime as Config>::BuyBackAccount::get().into_account_truncating();
		let incentive_account = IncentivePalletId::get().into_account_truncating();
		assert_eq!(Currencies::free_balance(VKSM, &buyback_account), 9000);
		assert_eq!(Currencies::free_balance(VKSM, &zenlink_pair_account_id), 2200);
		assert_eq!(Currencies::free_balance(BNC, &zenlink_pair_account_id), 2000);
		assert_eq!(Currencies::free_balance(BNC, &buyback_account), 0);
		assert_eq!(Currencies::free_balance(BNC, &incentive_account), 0);
		BbBNC::set_incentive(
			BB_BNC_SYSTEM_POOL_ID,
			Some(7 * 86400 / 12),
			Some(buyback_account.clone()),
		);
		assert_ok!(BuyBack::charge(RuntimeOrigin::signed(ALICE), VKSM, 1000));
		let infos = Infos::<Runtime>::get(VKSM).unwrap();
		assert_ok!(BuyBack::buy_back(&buyback_account, VKSM, &infos, 0));
		System::set_block_number(System::block_number() + 1);
		assert_eq!(Currencies::free_balance(VKSM, &buyback_account), 9000);
		assert_eq!(Currencies::free_balance(VKSM, &zenlink_pair_account_id), 3200);
		assert_eq!(Currencies::free_balance(BNC, &zenlink_pair_account_id), 1377);
		assert_eq!(Currencies::free_balance(BNC, &buyback_account), 0);
		assert_eq!(Currencies::free_balance(BNC, &incentive_account), 623);
	});
}

#[test]
fn on_initialize_no_burn_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let zenlink_pair_account_id = init_zenlink(PARAID);
		let destruction_ratio = None;
		let bias: Permill = Permill::from_percent(10);

		assert_ok!(BuyBack::set_vtoken(
			RuntimeOrigin::signed(ALICE),
			VKSM,
			VALUE,
			LIQUID_PROPORTION,
			BUYBACK_DURATION,
			LIQUID_DURATION,
			true,
			destruction_ratio,
			bias
		));
		let buyback_account = <Runtime as Config>::BuyBackAccount::get().into_account_truncating();
		let incentive_account = IncentivePalletId::get().into_account_truncating();
		assert_eq!(Currencies::free_balance(VKSM, &buyback_account), 9000);
		assert_eq!(Currencies::free_balance(VKSM, &zenlink_pair_account_id), 2200);
		assert_eq!(Currencies::free_balance(BNC, &zenlink_pair_account_id), 2000);
		assert_eq!(Currencies::free_balance(BNC, &buyback_account), 0);
		assert_eq!(Currencies::free_balance(BNC, &incentive_account), 0);
		BbBNC::set_incentive(
			BB_BNC_SYSTEM_POOL_ID,
			Some(7 * 86400 / 12),
			Some(buyback_account.clone()),
		);
		assert_ok!(BuyBack::charge(RuntimeOrigin::signed(ALICE), VKSM, 1000));
		BuyBack::on_initialize(1);
		BuyBack::on_initialize(2);
		BuyBack::on_initialize(3);
		assert_eq!(Currencies::free_balance(VKSM, &buyback_account), 9000);
		assert_eq!(Currencies::free_balance(VKSM, &zenlink_pair_account_id), 3200);
		assert_eq!(Currencies::free_balance(BNC, &zenlink_pair_account_id), 1377); // 362
		assert_eq!(Currencies::free_balance(BNC, &buyback_account), 0);
		assert_eq!(Currencies::free_balance(BNC, &incentive_account), 623);
	});
}

#[test]
fn on_initialize_with_burn_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let zenlink_pair_account_id = init_zenlink(PARAID);
		let destruction_ratio = Some(Permill::from_percent(10));
		let bias: Permill = Permill::from_percent(10);

		assert_ok!(BuyBack::set_vtoken(
			RuntimeOrigin::signed(ALICE),
			VKSM,
			VALUE,
			LIQUID_PROPORTION,
			BUYBACK_DURATION,
			LIQUID_DURATION,
			true,
			destruction_ratio,
			bias
		));
		let buyback_account = <Runtime as Config>::BuyBackAccount::get().into_account_truncating();
		let incentive_account = IncentivePalletId::get().into_account_truncating();
		assert_eq!(Currencies::free_balance(VKSM, &buyback_account), 9000);
		assert_eq!(Currencies::free_balance(VKSM, &zenlink_pair_account_id), 2200);
		assert_eq!(Currencies::free_balance(BNC, &zenlink_pair_account_id), 2000);
		assert_eq!(Currencies::free_balance(BNC, &buyback_account), 0);
		assert_eq!(Currencies::free_balance(BNC, &incentive_account), 0);
		BbBNC::set_incentive(
			BB_BNC_SYSTEM_POOL_ID,
			Some(7 * 86400 / 12),
			Some(buyback_account.clone()),
		);
		assert_ok!(BuyBack::charge(RuntimeOrigin::signed(ALICE), VKSM, 1000));
		BuyBack::on_initialize(<frame_system::Pallet<Runtime>>::block_number() + 1);
		System::set_block_number(System::block_number() + 1);
		BuyBack::on_initialize(<frame_system::Pallet<Runtime>>::block_number() + 1);
		System::set_block_number(System::block_number() + 1);
		assert_eq!(Currencies::free_balance(VKSM, &buyback_account), 9000);
		assert_eq!(Currencies::free_balance(VKSM, &zenlink_pair_account_id), 3200);
		assert_eq!(Currencies::free_balance(BNC, &zenlink_pair_account_id), 1377);
		assert_eq!(Currencies::free_balance(BNC, &buyback_account), 0);
		assert_eq!(Currencies::free_balance(BNC, &incentive_account), 561); // 623 - 62
	});
}

#[test]
fn on_initialize_with_bias_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let zenlink_pair_account_id = init_zenlink(PARAID);
		let destruction_ratio = None;
		let bias: Permill = Permill::from_percent(10);

		assert_ok!(BuyBack::set_vtoken(
			RuntimeOrigin::signed(ALICE),
			VKSM,
			VALUE,
			LIQUID_PROPORTION,
			BUYBACK_DURATION,
			LIQUID_DURATION,
			true,
			destruction_ratio,
			bias
		));
		let buyback_account = <Runtime as Config>::BuyBackAccount::get().into_account_truncating();
		let incentive_account = IncentivePalletId::get().into_account_truncating();
		assert_eq!(Currencies::free_balance(VKSM, &buyback_account), 9000);
		assert_eq!(Currencies::free_balance(VKSM, &zenlink_pair_account_id), 2200);
		assert_eq!(Currencies::free_balance(BNC, &zenlink_pair_account_id), 2000);
		assert_eq!(Currencies::free_balance(BNC, &buyback_account), 0);
		assert_eq!(Currencies::free_balance(BNC, &incentive_account), 0);
		BbBNC::set_incentive(
			BB_BNC_SYSTEM_POOL_ID,
			Some(7 * 86400 / 12),
			Some(buyback_account.clone()),
		);
		assert_ok!(BuyBack::charge(RuntimeOrigin::signed(ALICE), VKSM, 1000));
		BuyBack::on_initialize(1);
		let path = vec![
			AssetId::try_convert_from(VKSM, PARAID).unwrap(),
			AssetId::try_convert_from(BNC, PARAID).unwrap(),
		];
		assert_ok!(ZenlinkProtocol::swap_exact_assets_for_assets(
			RuntimeOrigin::signed(ALICE),
			100,
			0,
			path,
			ALICE,
			<frame_system::Pallet<Runtime>>::block_number() + BlockNumberFor::<Runtime>::from(1u32)
		));
		BuyBack::on_initialize(2);
		BuyBack::on_initialize(3);
		assert_eq!(Currencies::free_balance(VKSM, &buyback_account), 9000);
		assert_eq!(Currencies::free_balance(VKSM, &zenlink_pair_account_id), 3300);
		assert_eq!(Currencies::free_balance(BNC, &zenlink_pair_account_id), 1336);
		assert_eq!(Currencies::free_balance(BNC, &buyback_account), 0);
		assert_eq!(Currencies::free_balance(BNC, &incentive_account), 578);
	});
}

#[test]
fn on_initialize_with_bias_should_not_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let zenlink_pair_account_id = init_zenlink(PARAID);
		let destruction_ratio = None;
		let bias: Permill = Permill::from_percent(5);

		assert_ok!(BuyBack::set_vtoken(
			RuntimeOrigin::signed(ALICE),
			VKSM,
			VALUE,
			LIQUID_PROPORTION,
			BUYBACK_DURATION,
			LIQUID_DURATION,
			true,
			destruction_ratio,
			bias
		));
		let buyback_account = <Runtime as Config>::BuyBackAccount::get().into_account_truncating();
		let incentive_account = IncentivePalletId::get().into_account_truncating();
		assert_eq!(Currencies::free_balance(VKSM, &buyback_account), 9000);
		assert_eq!(Currencies::free_balance(VKSM, &zenlink_pair_account_id), 2200);
		assert_eq!(Currencies::free_balance(BNC, &zenlink_pair_account_id), 2000);
		assert_eq!(Currencies::free_balance(BNC, &buyback_account), 0);
		assert_eq!(Currencies::free_balance(BNC, &incentive_account), 0);
		BbBNC::set_incentive(
			BB_BNC_SYSTEM_POOL_ID,
			Some(7 * 86400 / 12),
			Some(buyback_account.clone()),
		);
		assert_ok!(BuyBack::charge(RuntimeOrigin::signed(ALICE), VKSM, 1000));
		BuyBack::on_initialize(1);
		let path = vec![
			AssetId::try_convert_from(VKSM, PARAID).unwrap(),
			AssetId::try_convert_from(BNC, PARAID).unwrap(),
		];
		assert_ok!(ZenlinkProtocol::swap_exact_assets_for_assets(
			RuntimeOrigin::signed(ALICE),
			100,
			0,
			path,
			ALICE,
			<frame_system::Pallet<Runtime>>::block_number() + BlockNumberFor::<Runtime>::from(1u32)
		));
		BuyBack::on_initialize(2);
		BuyBack::on_initialize(3);
		assert_eq!(Currencies::free_balance(VKSM, &buyback_account), 10000);
		assert_eq!(Currencies::free_balance(VKSM, &zenlink_pair_account_id), 2300);
		assert_eq!(Currencies::free_balance(BNC, &zenlink_pair_account_id), 1914);
		assert_eq!(Currencies::free_balance(BNC, &buyback_account), 0);
		assert_eq!(Currencies::free_balance(BNC, &incentive_account), 0);
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
