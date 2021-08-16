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

//! Test utilities

#![cfg(test)]
#![allow(non_upper_case_globals)]

use std::convert::TryFrom;

use frame_support::assert_ok;

use crate::{mock::*, *};

pub(crate) fn run_to_block(n: u64) {
	while System::block_number() < n {
		MinterReward::on_finalize(System::block_number());
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		MinterReward::on_initialize(System::block_number());
	}
}

#[test]
fn claim_reward_should_work() {
	ExtBuilder::default().ten_thousand_for_alice_n_bob().build().execute_with(|| {
		crate::UserReward::<Runtime>::insert(&ALICE, 1000);
		assert_eq!(MinterReward::user_reward(&ALICE), 1000);
		// Alice original has 100000 native token.
		assert_eq!(Currencies::free_balance(CurrencyId::Native(TokenSymbol::ASG), &ALICE), 100000);

		assert_ok!(MinterReward::claim_reward(Origin::signed(ALICE)));
		assert_eq!(MinterReward::user_reward(&ALICE), 0);
		assert_eq!(
			Currencies::free_balance(CurrencyId::Native(TokenSymbol::ASG), &ALICE),
			100000 + 1000
		)
	});
}

#[test]
fn minter_reward_should_work() {
	ExtBuilder::default().ten_thousand_for_alice_n_bob().build().execute_with(|| {
		run_to_block(2);

		let to_sell_dot = 20;
		let to_sell_vdot = 20;
		// let to_sell_ksm = 20;

		// create DEX pair
		let ausd_asset_id: AssetId =
			AssetId::try_from(CurrencyId::Stable(TokenSymbol::KUSD)).unwrap();
		let dot_asset_id: AssetId = AssetId::try_from(CurrencyId::Token(TokenSymbol::DOT)).unwrap();
		let vdot_asset_id: AssetId =
			AssetId::try_from(CurrencyId::VToken(TokenSymbol::DOT)).unwrap();
		let ksm_asset_id: AssetId = AssetId::try_from(CurrencyId::Token(TokenSymbol::KSM)).unwrap();
		let vksm_asset_id: AssetId =
			AssetId::try_from(CurrencyId::VToken(TokenSymbol::KSM)).unwrap();

		assert_ok!(ZenlinkProtocol::create_pair(
			Origin::signed(ALICE),
			ausd_asset_id,
			dot_asset_id
		));
		assert_ok!(ZenlinkProtocol::create_pair(
			Origin::signed(ALICE),
			ausd_asset_id,
			vdot_asset_id
		));
		assert_ok!(ZenlinkProtocol::create_pair(
			Origin::signed(ALICE),
			ausd_asset_id,
			ksm_asset_id
		));
		assert_ok!(ZenlinkProtocol::create_pair(
			Origin::signed(ALICE),
			ausd_asset_id,
			vksm_asset_id
		));

		let deadline: BlockNumberFor<Runtime> = <frame_system::Pallet<Runtime>>::block_number() +
			<Runtime as frame_system::Config>::BlockNumber::from(100u32);
		assert_ok!(ZenlinkProtocol::add_liquidity(
			Origin::signed(ALICE),
			ausd_asset_id,
			dot_asset_id,
			1000,
			1000,
			1,
			1,
			deadline
		));
		assert_ok!(ZenlinkProtocol::add_liquidity(
			Origin::signed(ALICE),
			ausd_asset_id,
			vdot_asset_id,
			1000,
			1000,
			1,
			1,
			deadline
		));
		assert_ok!(ZenlinkProtocol::add_liquidity(
			Origin::signed(ALICE),
			ausd_asset_id,
			ksm_asset_id,
			1000,
			1000,
			1,
			1,
			deadline
		));
		assert_ok!(ZenlinkProtocol::add_liquidity(
			Origin::signed(ALICE),
			ausd_asset_id,
			vksm_asset_id,
			1000,
			1000,
			1,
			1,
			deadline
		));

		// set currency staking lock period, 28 blocks for DOT
		assert_ok!(VtokenMint::set_token_staking_lock_period(
			Origin::root(),
			CurrencyId::Token(TokenSymbol::DOT),
			28
		));

		// add some data to the vtoken mint pools
		assert_ok!(VtokenMint::set_vtoken_pool(Origin::root(), DOT, 10000, 10000));
		assert_ok!(VtokenMint::mint(Origin::signed(ALICE), vDOT, to_sell_dot));

		run_to_block(3);
		assert_eq!(MinterReward::current_round_start_at(), 2);
		run_to_block(10);

		assert_eq!(MinterReward::reward_per_block(), 300);
		assert_eq!(
			MinterReward::maximum_vtoken_minted(),
			(2, 19, CurrencyId::VToken(TokenSymbol::DOT))
		);

		run_to_block(12);
		assert_ok!(VtokenMint::mint(Origin::signed(BOB), vDOT, to_sell_vdot + 40));
		assert_eq!(
			MinterReward::maximum_vtoken_minted(),
			(12, 56, CurrencyId::VToken(TokenSymbol::DOT))
		);

		// start block is 2, max extended block is 20, block 21 should still be the last block of
		// maximum_vtoken(12, 56, CurrencyId::VToken(TokenSymbol::DOT))
		run_to_block(21);
		assert_eq!(
			MinterReward::maximum_vtoken_minted(),
			(12, 56, CurrencyId::VToken(TokenSymbol::DOT))
		);

		// 23-2=21 >20, so the previous round will be ended and new round will be started.
		run_to_block(23);
		assert_eq!(MinterReward::current_round_start_at(), 0);
		assert_eq!(
			MinterReward::maximum_vtoken_minted(),
			(0, 0, CurrencyId::Native(TokenSymbol::BNC))
		);

		run_to_block(42);
		assert_ok!(VtokenMint::mint(Origin::signed(ALICE), vDOT, to_sell_vdot + 20));
		assert_eq!(MinterReward::current_round_start_at(), 42);

		// 60 blocks to be a halving reward cycle
		run_to_block(61);
		assert_eq!(MinterReward::reward_per_block(), 150);

		// 42+20=62. When block number reaches 63, the previous round will be ended, since no new
		// minting action is taken
	});
}
