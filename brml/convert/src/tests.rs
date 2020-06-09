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

use alloc::collections::btree_map::BTreeMap;
use crate::*;
use crate::mock::*;
use frame_support::assert_ok;
use node_primitives::{ConvertPool, TokenType};

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
		run_to_block(1);
		let vtoken = vec![0x12, 0x34];
		let precise = 4;
		assert_ok!(assets::Module::<Test>::create(Origin::ROOT, vtoken.into(), precise));
		run_to_block(2);
		let token_id = <assets::NextAssetId<Test>>::get() - 1;

		let convert_rate = 20;
		assert_ok!(Convert::set_convert_rate(Origin::ROOT, token_id.into(), convert_rate));
		run_to_block(3);
		let update_rate = 2;
		assert_ok!(Convert::set_rate_per_block(Origin::ROOT, token_id.into(), update_rate));
		run_to_block(4);

		let token_amount = 100u64;
		let vtoken_amount = 50u64;
		let pool = ConvertPool::new(token_amount, vtoken_amount);

		<Pool<Test>>::insert(token_id, pool);

		run_to_block(5);
		run_to_block(6);
		run_to_block(7);
		// 20 - 2 * 3 = 14
		assert_eq!(Convert::convert_rate(token_id), 0);
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

		assert_eq!(Convert::pool(token_id), ConvertPool::new(bob_token_convert, bob_token_convert * rate));
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

		assert_eq!(Convert::pool(token_id), ConvertPool::new(0, 0));
	});
}

#[test]
fn add_new_refer_channel_should_be_ok() {
	new_test_ext().execute_with(|| {
		run_to_block(2);

		let bob = 1u64;
		let alice = 2u64;

		// issue a vtoken
		let vtoken = vec![0x12, 0x34];
		let precise = 4;
		assert_ok!(assets::Module::<Test>::create(Origin::ROOT, vtoken.into(), precise));
		let vtoken_id = <assets::NextAssetId<Test>>::get() - 1;
		let token_id = vtoken_id;

		// issue vtoken and token to bob
		let bob_vtoken_issued = 60;
		let bob_token_issued = 100;
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, token_id.into(), TokenType::VToken, bob, bob_vtoken_issued));
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, token_id.into(), TokenType::Token, bob, bob_token_issued));

		// set convert rate, token => vtoken, 1token equals to 2vtoken
		let rate = 2;
		assert_ok!(Convert::set_convert_rate(Origin::ROOT, vtoken_id.into(), rate));

		let referer1 = 10;
		let referer2 = 11;
		let referer3 = 12;
		let referer4 = 13;

		// convert
		let bob_token_convert1 = (3, referer1);
		let bob_token_convert2 = (5, referer2);
		let bob_token_convert3 = (8, referer3);
		let bob_token_convert4 = (2, 0); // 0 means no referer
		assert_ok!(Convert::convert_token_to_vtoken(Origin::signed(bob), bob_token_convert1.0, vtoken_id.into(), Some(bob_token_convert1.1)));
		assert_ok!(Convert::convert_token_to_vtoken(Origin::signed(bob), bob_token_convert2.0, vtoken_id.into(), Some(bob_token_convert2.1)));
		assert_ok!(Convert::convert_token_to_vtoken(Origin::signed(bob), bob_token_convert2.0, vtoken_id.into(), Some(bob_token_convert2.1))); // recommend referer2 2 times
		assert_ok!(Convert::convert_token_to_vtoken(Origin::signed(bob), bob_token_convert3.0, vtoken_id.into(), Some(bob_token_convert3.1)));
		assert_ok!(Convert::convert_token_to_vtoken(Origin::signed(bob), bob_token_convert4.0, vtoken_id.into(), None)); // no referer

		// check bob's token change
		assert_eq!(
			<assets::AccountAssets<Test>>::get((token_id, TokenType::Token, bob)).balance,
			bob_token_issued - bob_token_convert1.0 - bob_token_convert2.0 * 2 - bob_token_convert3.0 - bob_token_convert4.0
		);
		// check bob's vtoken change
		assert_eq!(
			<assets::AccountAssets<Test>>::get((vtoken_id, TokenType::VToken, bob)).balance,
			bob_vtoken_issued + rate * (bob_token_convert1.0 + bob_token_convert2.0 * 2 + bob_token_convert3.0 + bob_token_convert4.0)
		);
		// check bob's refers
		assert_eq!(
			ReferrerChannels::<Test>::get(bob),
			(
				vec![(referer1, bob_token_convert1.0 * rate), (referer2, bob_token_convert2.0 * 2 * rate), (referer3, bob_token_convert3.0 * rate)],
				(bob_token_convert1.0 + bob_token_convert2.0 * 2 + bob_token_convert3.0) * rate
			)
		);

		// issue token/vtoken to alice
		let alice_vtoken_issued = 50;
		let alice_token_issued = 80;
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, token_id.into(), TokenType::VToken, alice, alice_vtoken_issued));
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, token_id.into(), TokenType::Token, alice, alice_token_issued));

		let alice_token_convert1 = (2, referer2);
		let alice_token_convert2 = (4, referer4);
		let alice_token_convert3 = (3, 0); // 0 means no referer

		assert_ok!(Convert::convert_token_to_vtoken(Origin::signed(alice), alice_token_convert1.0, vtoken_id.into(), Some(alice_token_convert1.1)));
		assert_ok!(Convert::convert_token_to_vtoken(Origin::signed(alice), alice_token_convert2.0, vtoken_id.into(), Some(alice_token_convert2.1)));
		assert_ok!(Convert::convert_token_to_vtoken(Origin::signed(alice), alice_token_convert3.0, vtoken_id.into(), None)); // no referer

		// check alice's token change
		assert_eq!(
			<assets::AccountAssets<Test>>::get((token_id, TokenType::Token, alice)).balance,
			alice_token_issued - alice_token_convert1.0 - alice_token_convert2.0 - alice_token_convert3.0
		);
		// check alice's vtoken change
		assert_eq!(
			<assets::AccountAssets<Test>>::get((vtoken_id, TokenType::VToken, alice)).balance,
			alice_vtoken_issued + rate * (alice_token_convert1.0 + alice_token_convert2.0 + alice_token_convert3.0)
		);
		// check alice's refers
		assert_eq!(
			ReferrerChannels::<Test>::get(alice),
			(
				vec![(referer2, alice_token_convert1.0 * rate), (referer4, alice_token_convert2.0 * rate)],
				(alice_token_convert1.0 + alice_token_convert2.0) * rate
			)
		);

		let all_channels: BTreeMap<u64, u64> = [
			(referer1, bob_token_convert1.0 * rate),
			(referer2, (bob_token_convert2.0 * 2 + alice_token_convert1.0) * rate),
			(referer3, bob_token_convert3.0 * rate),
			(referer4, alice_token_convert2.0 * rate)
		].iter().cloned().collect();

		// check all channels
		assert_eq!(
			AllReferrerChannels::<Test>::get(),
			(
				all_channels,
				(bob_token_convert1.0 + bob_token_convert2.0 * 2 + bob_token_convert3.0 + alice_token_convert1.0 + alice_token_convert2.0) * rate
			)
		);

		// now convert vtoken to token
		let alice_vtoken = 5;
		assert_ok!(Convert::convert_vtoken_to_token(Origin::signed(alice), alice_vtoken, vtoken_id.into()));
		let all_channels: BTreeMap<u64, u64> = [
			(referer1, bob_token_convert1.0 * rate),
			(referer2, (bob_token_convert2.0 * 2 + alice_token_convert1.0) * rate - alice_vtoken),
			(referer3, bob_token_convert3.0 * rate),
			(referer4, alice_token_convert2.0 * rate)
		].iter().cloned().collect();

		assert_eq!(
			AllReferrerChannels::<Test>::get(),
			(
				all_channels,
				(bob_token_convert1.0 + bob_token_convert2.0 * 2 + bob_token_convert3.0 + alice_token_convert1.0 + alice_token_convert2.0) * rate - alice_vtoken
			)
		);

		assert_eq!(
			ReferrerChannels::<Test>::get(alice),
			(
				vec![(referer2, 0), (referer4, alice_token_convert2.0 * rate - 1)], // 5 = 4 + 1, 4 - 4 = 0, 8 - 1 = 7
				(alice_token_convert1.0 + alice_token_convert2.0) * rate - alice_vtoken
			)
		);
	});
}
