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
use node_primitives::{TimeUnit, TokenInfo, VtokenMintingOperator};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

#[test]
fn token_config_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_ok!(SystemStaking::token_config(
			RuntimeOrigin::root(),
			KSM,
			Some(1),
			Some(Permill::from_percent(80)),
			Some(false),
			Some(100),
			None,
			None,
		));
		let token_info = <TokenStatus<Runtime>>::get(KSM).unwrap();
		assert_eq!(token_info.new_config.add_or_sub, false);
		assert_eq!(token_info.new_config.exec_delay, 1);
		assert_eq!(token_info.new_config.system_stakable_farming_rate, Permill::from_percent(80));
		assert_eq!(token_info.new_config.system_stakable_base, 100);
		assert_eq!(token_info.new_config.farming_poolids, Vec::<PoolId>::new());
		assert_eq!(token_info.new_config.lptoken_rates, Vec::<Perbill>::new());
	});
}

#[test]
fn delete_token_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_ok!(SystemStaking::token_config(
			RuntimeOrigin::root(),
			KSM,
			Some(1),
			Some(Permill::from_percent(80)),
			Some(false),
			Some(100),
			None,
			None,
		));

		assert_ok!(SystemStaking::token_config(
			RuntimeOrigin::root(),
			MOVR,
			Some(2),
			Some(Permill::from_percent(80)),
			Some(false),
			Some(100),
			None,
			None,
		));

		assert_ok!(SystemStaking::token_config(
			RuntimeOrigin::root(),
			MOVR,
			Some(2),
			Some(Permill::from_percent(80)),
			Some(false),
			Some(100),
			None,
			None,
		));

		assert_ok!(SystemStaking::delete_token(RuntimeOrigin::root(), MOVR,));

		assert!(<TokenStatus<Runtime>>::get(MOVR).is_none());
		assert!(<TokenStatus<Runtime>>::get(KSM).is_some());
		let token_list = <TokenList<Runtime>>::get();
		assert_eq!(token_list.len(), 1);
		assert!(!token_list.clone().into_inner().contains(&MOVR));
		assert!(token_list.into_inner().contains(&KSM));
	});
}

#[test]
fn round_info_should_correct() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		System::set_block_number(System::block_number() + 1000);
		assert_ok!(SystemStaking::token_config(
			RuntimeOrigin::root(),
			KSM,
			Some(1),
			Some(Permill::from_percent(80)),
			Some(false),
			Some(100),
			None,
			None,
		));
		roll_one_block();
		assert_eq!(SystemStaking::round().unwrap().length, 5);
		assert_eq!(SystemStaking::round().unwrap().current, 1);
		assert_eq!(SystemStaking::round().unwrap().first, 1001);
	});
}

#[test]
fn refresh_token_info_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let (pid, _tokens) = init_farming_no_gauge();
		asset_registry();
		assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(ALICE), KSM, 10));
		pub const FEE: Permill = Permill::from_percent(5);
		assert_ok!(VtokenMinting::set_fees(RuntimeOrigin::root(), FEE, FEE));
		assert_ok!(VtokenMinting::set_unlock_duration(
			RuntimeOrigin::signed(ALICE),
			KSM,
			TimeUnit::Era(1)
		));
		assert_ok!(VtokenMinting::increase_token_pool(KSM, 1000));
		assert_ok!(VtokenMinting::update_ongoing_time_unit(KSM, TimeUnit::Era(1)));
		assert_ok!(VtokenMinting::set_minimum_redeem(RuntimeOrigin::signed(ALICE), vKSM, 10));

		assert_ok!(SystemStaking::token_config(
			RuntimeOrigin::root(),
			KSM,
			Some(1),
			Some(Permill::from_percent(80)),
			Some(false),
			Some(100),
			Some(vec![pid]),
			Some(vec![Perbill::from_percent(100)]),
		));

		assert_ok!(SystemStaking::refresh_token_info(RuntimeOrigin::root(), KSM));
		let token_info = <TokenStatus<Runtime>>::get(KSM).unwrap();
		assert_eq!(token_info.new_config, token_info.current_config);
	});
}

#[test]
fn round_process_token() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let (pid, _tokens) = init_farming_no_gauge();
		asset_registry();
		assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(ALICE), KSM, 10));
		pub const FEE: Permill = Permill::from_percent(5);
		assert_ok!(VtokenMinting::set_fees(RuntimeOrigin::root(), FEE, FEE));
		assert_ok!(VtokenMinting::set_unlock_duration(
			RuntimeOrigin::signed(ALICE),
			KSM,
			TimeUnit::Era(1)
		));
		assert_ok!(VtokenMinting::increase_token_pool(KSM, 1000));
		assert_ok!(VtokenMinting::update_ongoing_time_unit(KSM, TimeUnit::Era(1)));
		assert_ok!(VtokenMinting::set_minimum_redeem(RuntimeOrigin::signed(ALICE), vKSM, 10));

		assert_ok!(SystemStaking::token_config(
			RuntimeOrigin::root(),
			KSM,
			Some(1),
			Some(Permill::from_percent(80)),
			Some(false),
			Some(100),
			Some(vec![pid]),
			Some(vec![Perbill::from_percent(100)]),
		));

		roll_to(5); // round start
		roll_to(6); // delay exec

		let token_info = <TokenStatus<Runtime>>::get(KSM).unwrap();
		assert!(token_info.system_shadow_amount > 0);
	});
}

#[test]
fn round_process_token_rollback() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let (pid, _tokens) = init_farming_no_gauge();
		asset_registry();
		assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(ALICE), KSM, 10000));
		pub const FEE: Permill = Permill::from_percent(5);
		assert_ok!(VtokenMinting::set_fees(RuntimeOrigin::root(), FEE, FEE));
		assert_ok!(VtokenMinting::set_unlock_duration(
			RuntimeOrigin::signed(ALICE),
			KSM,
			TimeUnit::Era(1)
		));
		assert_ok!(VtokenMinting::increase_token_pool(KSM, 1000));
		assert_ok!(VtokenMinting::update_ongoing_time_unit(KSM, TimeUnit::Era(1)));
		assert_ok!(VtokenMinting::set_minimum_redeem(RuntimeOrigin::signed(ALICE), vKSM, 10000));

		assert_ok!(SystemStaking::token_config(
			RuntimeOrigin::root(),
			KSM,
			Some(1),
			Some(Permill::from_percent(80)),
			Some(false),
			Some(100),
			Some(vec![pid]),
			Some(vec![Perbill::from_percent(100)]),
		));

		roll_to(5); // round start
		roll_to(6); // delay exec

		let token_info = <TokenStatus<Runtime>>::get(KSM).unwrap();
		assert!(token_info.system_shadow_amount == 0);
	});
}

fn init_farming_no_gauge() -> (PoolId, BalanceOf<Runtime>) {
	let mut tokens_proportion_map = BTreeMap::<CurrencyIdOf<Runtime>, Perbill>::new();
	tokens_proportion_map.entry(KSM).or_insert(Perbill::from_percent(100));
	let tokens_proportion = vec![(KSM, Perbill::from_percent(100))];
	let tokens = 1000;
	let basic_rewards = vec![(KSM, 1000)];
	let gauge_basic_rewards = vec![(KSM, 1000)];

	assert_ok!(Farming::create_farming_pool(
		RuntimeOrigin::signed(ALICE),
		tokens_proportion.clone(),
		basic_rewards.clone(),
		Some((KSM, 1000, gauge_basic_rewards)),
		0,
		0,
		10,
		0,
		1
	));

	let pid = 0;
	let charge_rewards = vec![(KSM, 100000)];
	assert_ok!(Farming::charge(RuntimeOrigin::signed(BOB), pid, charge_rewards));
	assert_ok!(Farming::deposit(RuntimeOrigin::signed(ALICE), pid, tokens.clone(), None));
	(pid, tokens)
}

fn increase_farming_no_gauge(pid: u32) {
	assert_ok!(Farming::deposit(RuntimeOrigin::signed(ALICE), pid, 1000, None));
}

fn asset_registry() {
	let items = vec![(KSM, 10 * milli::<Runtime>(KSM))];
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
