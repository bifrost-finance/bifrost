// Copyright 2019-2020 Liebi Technologies.
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
use frame_support::assert_ok;
use node_primitives::TokenType;

#[test]
fn swap_vtoken_to_token_should_be_ok() {
	new_test_ext().execute_with(|| {
		run_to_block(2);

		let alice = 1u64;
		let bob = 2u64;

		// issue a vtoken to alice
		let vtoken = vec![0x12, 0x34];
		let precise = 4;
		let vtoken_amount = 50;
		assert_ok!(assets::Module::<Test>::create(Origin::ROOT, vtoken, precise));
		let vtoken_id = <assets::NextAssetId<Test>>::get() - 1;
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, vtoken_id, TokenType::VToken, alice, vtoken_amount));

		// issue a token balances to alice
		let token_amount = 30;
		let token_id = vtoken_id;
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, token_id, TokenType::Token, alice, token_amount));

		// issue vtoken balances to bob
		let bob_vtoken_amount = 10;
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, vtoken_id, TokenType::VToken, bob, bob_vtoken_amount));

		// set exchange rate
		let fee = 2;
		assert_ok!(Swap::set_fee(Origin::ROOT, vtoken_id, fee));
		assert_eq!(<Fee<Test>>::get(token_id, vtoken_id), fee);

		// alice provide the transaction pool
		let token_pool = 20;
		let vtoken_pool = 20;
		assert_ok!(Swap::add_liquidity(Origin::ROOT, alice, token_pool, vtoken_id, vtoken_pool));
		assert_eq!(<assets::Balances<Test>>::get((token_id, TokenType::Token, alice)), token_amount - token_pool);
		assert_eq!(<assets::Balances<Test>>::get((vtoken_id, TokenType::VToken, alice)), vtoken_amount - vtoken_pool);
		assert_eq!(<InVariant<Test>>::get(token_id, vtoken_id), (token_pool, vtoken_pool, token_pool * vtoken_pool));

		// swap
		let bob_vtoken_out = 5;
		assert_ok!(Swap::swap_vtoken_to_token(Origin::signed(bob), bob_vtoken_out, vtoken_id));
		assert_eq!(<assets::Balances<Test>>::get((vtoken_id, TokenType::VToken, bob)), bob_vtoken_amount - bob_vtoken_out); // check bob's vtoken change
		assert_eq!(<assets::Balances<Test>>::get((token_id, TokenType::Token, bob)), 4); // check bob get token amount
		assert_eq!(<InVariant<Test>>::get(token_id, vtoken_id), (16, 25, token_pool * vtoken_pool)); // check pool change
	});
}

#[test]
fn swap_token_to_vtoken_should_be_ok() {
	new_test_ext().execute_with(|| {
		run_to_block(2);

		let alice = 1u64;
		let bob = 2u64;

		// issue a vtoken to alice
		let vtoken = vec![0x12, 0x34];
		let precise = 4;
		let vtoken_amount = 50;
		assert_ok!(assets::Module::<Test>::create(Origin::ROOT, vtoken, precise));
		let vtoken_id = <assets::NextAssetId<Test>>::get() - 1;
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, vtoken_id, TokenType::VToken, alice, vtoken_amount));

		// issue a token balances to alice
		let token_amount = 30;
		let token_id = vtoken_id;
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, token_id, TokenType::Token, alice, token_amount));

		// issue token balances to bob
		let bob_token_amount = 20;
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, token_id, TokenType::Token, bob, bob_token_amount));

		// set exchange rate
		let fee = 2;
		assert_ok!(Swap::set_fee(Origin::ROOT, vtoken_id, fee));
		assert_eq!(<Fee<Test>>::get(token_id, vtoken_id), fee);

		// add pool
		let token_pool = 20;
		let vtoken_pool = 30;
		assert_ok!(Swap::add_liquidity(Origin::ROOT, alice, token_pool, vtoken_id, vtoken_pool));
		assert_eq!(<assets::Balances<Test>>::get((token_id, TokenType::Token, alice)), token_amount - token_pool);
		assert_eq!(<assets::Balances<Test>>::get((vtoken_id, TokenType::VToken, alice)), vtoken_amount - vtoken_pool);
		assert_eq!(<InVariant<Test>>::get(token_id, vtoken_id), (token_pool, vtoken_pool, token_pool * vtoken_pool));

		// swap
		let bob_token_out = 10;
		assert_ok!(Swap::swap_token_to_vtoken(Origin::signed(bob), bob_token_out, vtoken_id));
		assert_eq!(<assets::Balances<Test>>::get((token_id, TokenType::Token, bob)), bob_token_amount - bob_token_out); // check bob's token change
		assert_eq!(<assets::Balances<Test>>::get((vtoken_id, TokenType::VToken, bob)), 10); // check bob get vtoken amount
		assert_eq!(<InVariant<Test>>::get(token_id, vtoken_id), (30, 20, token_pool * vtoken_pool)); // check pool change
	});
}
