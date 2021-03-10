// Copyright 2019-2021 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

//! Tests for the module.
#![cfg(test)]

use crate::*;
use crate::mock::*;
use frame_support::{assert_ok, assert_noop};
use node_primitives::Balance;

#[test]
fn to_vtoken_should_be_ok() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			let (dot_pool, vdot_pool) = (100, 200);
			assert_ok!(VtokenMint::expand_mint_pool(DOT, dot_pool));
			assert_ok!(VtokenMint::expand_mint_pool(vDOT, vdot_pool));

			let alice_dot = Assets::free_balance(DOT, &ALICE);
			let alice_vdot = Assets::free_balance(vDOT, &ALICE);

			let dot_price = vdot_pool / dot_pool;
			let to_sell_dot = 20;
			let minted_vdot = to_sell_dot * dot_price;

			System::set_block_number(1);

			// Alice sell 20 DOTs to mint vDOT.
			assert_ok!(VtokenMint::to_vtoken(Origin::signed(ALICE), vDOT, to_sell_dot));

			// Check event
			let mint_vtoken_event = mock::Event::vtoken_mint(crate::Event::MintedVToken(ALICE, vDOT, minted_vdot));
			assert!(System::events().iter().any(|record| record.event == mint_vtoken_event));

			// check Alice DOTs and vDOTs.
			assert_eq!(Assets::free_balance(DOT, &ALICE), alice_dot - to_sell_dot);
			assert_eq!(Assets::free_balance(vDOT, &ALICE), alice_vdot + minted_vdot);

			// check total DOTs and vDOTs.
			assert_eq!(VtokenMint::get_mint_pool(DOT), dot_pool + to_sell_dot);
			assert_eq!(VtokenMint::get_mint_pool(vDOT), vdot_pool + minted_vdot);

			// Alice selling BNC should not work.
			assert_noop!(
				VtokenMint::to_vtoken(Origin::signed(ALICE), BNC, to_sell_dot),
				Error::<Runtime>::NotSupportTokenType
			);

			// Alice selling DOT should not work due to it only support minting vDOT(vtoken).
			assert_noop!(
				VtokenMint::to_vtoken(Origin::signed(ALICE), DOT, to_sell_dot),
				Error::<Runtime>::NotSupportTokenType
			);

			// Alice selling 0 DOTs should not work.
			assert_noop!(
				VtokenMint::to_vtoken(Origin::signed(ALICE), vDOT, 0),
				Error::<Runtime>::BalanceZero
			);

			// Alice selling amount of DOTs exceeds all she has.
			assert_noop!(
				VtokenMint::to_vtoken(Origin::signed(ALICE), vDOT, Balance::max_value()),
				Error::<Runtime>::BalanceLow
			);
		});
}

#[test]
fn to_token_should_be_ok() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			let (dot_pool, vdot_pool) = (100, 200);
			assert_ok!(VtokenMint::expand_mint_pool(DOT, dot_pool));
			assert_ok!(VtokenMint::expand_mint_pool(vDOT, vdot_pool));

			let alice_dot = Assets::free_balance(DOT, &ALICE);
			let alice_vdot = Assets::free_balance(vDOT, &ALICE);

			let vdot_price = vdot_pool / dot_pool;
			let to_sell_vdot = 20;
			let minted_dot = to_sell_vdot / vdot_price;

			System::set_block_number(1);

			// Alice sell 20 vDOTs to mint DOT.
			assert_ok!(VtokenMint::to_token(Origin::signed(ALICE), DOT, to_sell_vdot));

			// Check event
			let mint_token_event = mock::Event::vtoken_mint(crate::Event::MintedToken(ALICE, DOT, minted_dot));
			assert!(System::events().iter().any(|record| record.event == mint_token_event));

			// check Alice DOTs and vDOTs.
			assert_eq!(Assets::free_balance(DOT, &ALICE), alice_dot + minted_dot);
			assert_eq!(Assets::free_balance(vDOT, &ALICE), alice_vdot - to_sell_vdot);

			// check total DOTs and vDOTs.
			assert_eq!(VtokenMint::get_mint_pool(DOT), dot_pool - minted_dot);
			assert_eq!(VtokenMint::get_mint_pool(vDOT), vdot_pool - to_sell_vdot);

			// Alice selling aUSD should not work.
			assert_noop!(
				VtokenMint::to_token(Origin::signed(ALICE), aUSD, to_sell_vdot),
				Error::<Runtime>::NotSupportTokenType
			);

			// Alice selling vDOT should not work due to it only support minting DOT(token).
			assert_noop!(
				VtokenMint::to_token(Origin::signed(ALICE), vDOT, to_sell_vdot),
				Error::<Runtime>::NotSupportTokenType
			);

			// Alice selling 0 vDOTs should not work.
			assert_noop!(
				VtokenMint::to_token(Origin::signed(ALICE), DOT, 0),
				Error::<Runtime>::BalanceZero
			);

			// Alice selling amount of DOTs exceeds all she has.
			assert_noop!(
				VtokenMint::to_token(Origin::signed(ALICE), DOT, Balance::max_value()),
				Error::<Runtime>::BalanceLow
			);
		});
}

#[test]
fn zero_token_pool_should_not_work() {
	ExtBuilder::default()
		.zero_for_alice_n_bob()
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			let to_sell_vdot = 20;
			let to_sell_ksm = 20;

			// Alice sell 20 vDOTs to mint DOT.
			assert_noop!(
				VtokenMint::to_token(Origin::signed(ALICE), DOT, to_sell_vdot),
				Error::<Runtime>::EmptyVtokenPool
			);

			// Alice sell 20 KSMs to mint vKSM.
			assert_noop!(
				VtokenMint::to_vtoken(Origin::signed(BOB), vKSM, to_sell_ksm),
				Error::<Runtime>::EmptyVtokenPool
			);
		});
}
