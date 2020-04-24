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
fn update_rate_multiple_times() {
	new_test_ext().execute_with(|| {
		// issue a vtoken
		let vtoken = vec![0x12, 0x34];
		let precise = 4;
		assert_ok!(assets::Module::<Test>::create(Origin::ROOT, vtoken.into(), precise));
		let vtoken_id = <assets::NextAssetId<Test>>::get() - 1;

		let convert_rate = 20;
		assert_ok!(Convert::set_convert_rate(Origin::ROOT, vtoken_id.into(), convert_rate));
		let update_rate = 2;
		assert_ok!(Convert::set_rate_per_block(Origin::ROOT, vtoken_id.into(), update_rate));

		let change_times = 3;
		run_to_block(change_times + 1);
		// 20 - 2 * 3 = 14
		assert_eq!(Convert::convert_rate(vtoken_id), convert_rate - update_rate * change_times);
	});
}

#[test]
fn update_rate_multiple_times_until_overflow() {
	new_test_ext().execute_with(|| {
		// issue a vtoken
		let vtoken = vec![0x12, 0x34];
		let precise = 4;
		assert_ok!(assets::Module::<Test>::create(Origin::ROOT, vtoken.into(), precise));
		let vtoken_id = <assets::NextAssetId<Test>>::get() - 1;

		let convert_rate = 20;
		assert_ok!(Convert::set_convert_rate(Origin::ROOT, vtoken_id.into(), convert_rate));
		let update_rate = 2;
		assert_ok!(Convert::set_rate_per_block(Origin::ROOT, vtoken_id.into(), update_rate));

		let change_times = 3;
		run_to_block(change_times + 20);
		// 20 - 2 * 3 = 14
		assert_eq!(Convert::convert_rate(vtoken_id), 0);
	});
}

#[test]
fn convert_token_to_vtoken_should_be_ok() {
	new_test_ext().execute_with(|| {
		run_to_block(2);

		let bob = 1u64;

		// issue a vtoken
		let vtoken = vec![0x12, 0x34];
		let precise = 4;
		assert_ok!(assets::Module::<Test>::create(Origin::ROOT, vtoken.into(), precise));
		let vtoken_id = <assets::NextAssetId<Test>>::get() - 1;
		let token_id = vtoken_id;

		// issue vtoken and token to bob
		let bob_vtoken_issued = 60;
		let bob_token_issued = 20;
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, token_id.into(), TokenType::VToken, bob, bob_vtoken_issued)); // 60 vtokens to bob
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, token_id.into(), TokenType::Token, bob, bob_token_issued)); // 20 tokens to bob

		// set convert rate, token => vtoken, 1token equals to 2vtoken
		let rate = 2;
		assert_ok!(Convert::set_convert_rate(Origin::ROOT, vtoken_id.into(), rate));

		// convert
		let bob_token_convert = 10;
		assert_ok!(Convert::convert_token_to_vtoken(Origin::signed(bob), bob_token_convert, vtoken_id.into(), None));
		assert_eq!(<assets::AccountAssets<Test>>::get((token_id, TokenType::Token, bob)).balance, bob_token_issued - bob_token_convert); // check bob's token change
		assert_eq!(<assets::AccountAssets<Test>>::get((vtoken_id, TokenType::VToken, bob)).balance, bob_vtoken_issued + bob_token_convert * rate); // check bob's token change
	});
}

#[test]
fn convert_vtoken_to_token_should_be_ok() {
	new_test_ext().execute_with(|| {
		run_to_block(2);

		let bob = 1u64;

		// issue a vtoken
		let vtoken = vec![0x12, 0x34];
		let precise = 4;
		assert_ok!(assets::Module::<Test>::create(Origin::ROOT, vtoken.into(), precise));
		let vtoken_id = <assets::NextAssetId<Test>>::get() - 1;
		let token_id = vtoken_id;

		// issue vtoken and token to bob
		let bob_vtoken_issued = 60;
		let bob_token_issued = 20;
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, token_id.into(), TokenType::VToken, bob, bob_vtoken_issued)); // 60 vtokens to bob
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, token_id.into(), TokenType::Token, bob, bob_token_issued)); // 20 tokens to bob

		// set convert rate, token => vtoken, 1token equals to 2vtoken
		let rate = 2;
		assert_ok!(Convert::set_convert_rate(Origin::ROOT, vtoken_id.into(), rate));

		// convert
		let bob_vtoken_convert = 10;
		assert_ok!(Convert::convert_vtoken_to_token(Origin::signed(bob), bob_vtoken_convert, vtoken_id.into()));
		assert_eq!(<assets::AccountAssets<Test>>::get((token_id, TokenType::VToken, bob)).balance, bob_vtoken_issued - bob_vtoken_convert); // check bob's token change
		assert_eq!(<assets::AccountAssets<Test>>::get((vtoken_id, TokenType::Token, bob)).balance, bob_token_issued + bob_vtoken_convert / rate); // check bob's token change
	});
}
