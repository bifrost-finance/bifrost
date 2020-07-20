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
use node_primitives::{ConvertPool, TokenSymbol};

#[test]
#[ignore]
fn update_rate_multiple_times() {
	new_test_ext().execute_with(|| {
		run_to_block(5);
		// issue a vtoken
		let symbol = b"aUSD".to_vec();
		let precise = 18;
		assert_ok!(assets::Module::<Test>::create(Origin::root(), symbol, precise));
		let ausd_id = <assets::NextAssetId<Test>>::get() - 1;
		let ausd_type = TokenSymbol::from(ausd_id);

		let convert_rate = 20;
		assert_ok!(Convert::set_convert_price(Origin::root(), ausd_type, convert_rate));
		let update_rate = 2;
		assert_ok!(Convert::set_price_per_block(Origin::root(), ausd_type, update_rate));

		let change_times = 3;
		run_to_block(change_times + 5);
		// 20 - 2 * 3 = 14
		assert_eq!(Convert::convert_price(ausd_type), convert_rate - update_rate * change_times);
	});
}

#[test]
#[ignore]
fn update_rate_multiple_times_until_overflow() {
	new_test_ext().execute_with(|| {
		// issue a vtoken
		run_to_block(1);
		let symbol = b"aUSD".to_vec();
		let precise = 4;
		assert_ok!(assets::Module::<Test>::create(Origin::root(), symbol, precise));
		let ausd_id = <assets::NextAssetId<Test>>::get() - 1;
		let ausd_type = TokenSymbol::from(ausd_id);

		let convert_rate = 20;
		assert_ok!(Convert::set_convert_price(Origin::root(), ausd_type, convert_rate));
		run_to_block(3);
		let update_rate = 2;
		assert_ok!(Convert::set_price_per_block(Origin::root(), ausd_type, update_rate));
		run_to_block(4);

		let token_amount = 100u64;
		let vtoken_amount = 50u64;
		let pool = ConvertPool::new(token_amount, vtoken_amount);

		<Pool<Test>>::insert(ausd_type, pool);

		run_to_block(5);
		run_to_block(6);
		run_to_block(7);
		// 20 - 2 * 3 = 14
		assert_eq!(Convert::convert_price(ausd_type), 0);
	});
}

#[test]
fn convert_token_to_vtoken_should_be_ok() {
	new_test_ext().execute_with(|| {
		run_to_block(2);

		let bob = 1u64;

		// issue a vtoken
		let dot_symbol = b"DOT".to_vec();
		let vdot_symbol = b"vDOT".to_vec();
		let precise = 4;

		assert_ok!(assets::Module::<Test>::create(Origin::root(), b"aUSD".to_vec(), precise)); // let asset id is start from 1

		assert_ok!(assets::Module::<Test>::create(Origin::root(), dot_symbol, precise));
		let dot_id = <assets::NextAssetId<Test>>::get() - 1;
		let dot_type = TokenSymbol::from(dot_id);

		assert_ok!(assets::Module::<Test>::create(Origin::root(), vdot_symbol, precise));
		let vdot_id = <assets::NextAssetId<Test>>::get() - 1;
		let vdot_type = TokenSymbol::from(vdot_id);

		// issue vtoken and token to bob
		let bob_dot_issued = 60;
		let bob_vdot_issued = 20;
		assert_ok!(assets::Module::<Test>::issue(Origin::root(), dot_type, bob, bob_dot_issued)); // 60 vtokens to bob
		assert_ok!(assets::Module::<Test>::issue(Origin::root(), vdot_type, bob, bob_vdot_issued)); // 20 tokens to bob

		// set convert rate, token => vtoken, 1token equals to 2vtoken
		let rate = 2;
		assert_ok!(Convert::set_convert_price(Origin::root(), dot_type, rate));

		// convert
		let bob_dot_convert = 10;
		assert_ok!(Convert::convert_token_to_vtoken(Origin::signed(bob), vdot_type, bob_dot_convert, None));
		assert_eq!(<assets::AccountAssets<Test>>::get((dot_type, bob)).balance, bob_dot_issued - bob_dot_convert); // check bob's token change
		assert_eq!(<assets::AccountAssets<Test>>::get((vdot_type, bob)).balance, bob_vdot_issued + bob_dot_convert * rate); // check bob's token change

		assert_eq!(Convert::pool(vdot_type), ConvertPool::new(bob_dot_convert, bob_dot_convert * rate));
	});
}

#[test]
fn convert_vtoken_to_token_should_be_ok() {
	new_test_ext().execute_with(|| {
		run_to_block(2);

		let bob = 1u64;

		// issue a vtoken
		let dot_symbol = b"DOT".to_vec();
		let vdot_symbol = b"vDOT".to_vec();
		let precise = 4;

		assert_ok!(assets::Module::<Test>::create(Origin::root(), b"aUSD".to_vec(), precise)); // let asset id is start from 1

		assert_ok!(assets::Module::<Test>::create(Origin::root(), dot_symbol, precise));
		let dot_id = <assets::NextAssetId<Test>>::get() - 1;
		let dot_type = TokenSymbol::from(dot_id);

		assert_ok!(assets::Module::<Test>::create(Origin::root(), vdot_symbol, precise));
		let vdot_id = <assets::NextAssetId<Test>>::get() - 1;
		let vdot_type = TokenSymbol::from(vdot_id);

		// issue vtoken and token to bob
		let bob_vdot_issued = 60;
		let bob_dot_issued = 20;
		assert_ok!(assets::Module::<Test>::issue(Origin::root(), dot_type, bob, bob_dot_issued)); // 20 tokens to bob
		assert_ok!(assets::Module::<Test>::issue(Origin::root(), vdot_type, bob, bob_vdot_issued)); // 60 tokens to bob

		// set convert rate, token => vtoken, 1token equals to 2vtoken
		let rate = 2;
		assert_ok!(Convert::set_convert_price(Origin::root(), dot_type, rate));

		// convert
		let bob_vdot_convert = 10;
		assert_ok!(Convert::convert_vtoken_to_token(Origin::signed(bob), dot_type, bob_vdot_convert));
		assert_eq!(<assets::AccountAssets<Test>>::get((vdot_type, bob)).balance, bob_vdot_issued - bob_vdot_convert); // check bob's token change
		assert_eq!(<assets::AccountAssets<Test>>::get((dot_type, bob)).balance, bob_dot_issued + bob_vdot_convert / rate); // check bob's token change

		assert_eq!(Convert::pool(vdot_type), ConvertPool::new(0, 0));
	});
}

#[test]
fn add_new_refer_channel_should_be_ok() {
	new_test_ext().execute_with(|| {
		run_to_block(2);

		let bob = 1u64;
		let alice = 2u64;

		// issue a vdot
		let dot_symbol = b"DOT".to_vec();
		let vdot_symbol = b"vDOT".to_vec();
		let precise = 4;

		assert_ok!(assets::Module::<Test>::create(Origin::root(), b"aUSD".to_vec(), precise)); // let asset id is start from 1

		assert_ok!(assets::Module::<Test>::create(Origin::root(), dot_symbol, precise));
		let dot_id = <assets::NextAssetId<Test>>::get() - 1;
		let dot_type = TokenSymbol::from(dot_id);

		assert_ok!(assets::Module::<Test>::create(Origin::root(), vdot_symbol, precise));
		let vdot_id = <assets::NextAssetId<Test>>::get() - 1;
		let vdot_type = TokenSymbol::from(vdot_id);

		// issue vdot and dot to bob
		let bob_vdot_issued = 60;
		let bob_dot_issued = 100;
		assert_ok!(assets::Module::<Test>::issue(Origin::root(), dot_type, bob, bob_dot_issued));
		assert_ok!(assets::Module::<Test>::issue(Origin::root(), vdot_type, bob, bob_vdot_issued));

		// set convert rate, dot => vdot, 1dot equals to 2vdot
		let rate = 2;
		assert_ok!(Convert::set_convert_price(Origin::root(), dot_type, rate));

		let referer1 = 10;
		let referer2 = 11;
		let referer3 = 12;
		let referer4 = 13;

		// convert
		let bob_dot_convert1 = (3, referer1);
		let bob_dot_convert2 = (5, referer2);
		let bob_dot_convert3 = (8, referer3);
		let bob_dot_convert4 = (2, 0); // 0 means no referer
		assert_ok!(Convert::convert_token_to_vtoken(Origin::signed(bob), vdot_type, bob_dot_convert1.0, Some(bob_dot_convert1.1)));
		assert_ok!(Convert::convert_token_to_vtoken(Origin::signed(bob), vdot_type, bob_dot_convert2.0, Some(bob_dot_convert2.1)));
		assert_ok!(Convert::convert_token_to_vtoken(Origin::signed(bob), vdot_type, bob_dot_convert2.0, Some(bob_dot_convert2.1))); // recommend referer2 2 times
		assert_ok!(Convert::convert_token_to_vtoken(Origin::signed(bob), vdot_type, bob_dot_convert3.0, Some(bob_dot_convert3.1)));
		assert_ok!(Convert::convert_token_to_vtoken(Origin::signed(bob), vdot_type, bob_dot_convert4.0, None)); // no referer

		// check bob's dot change
		assert_eq!(
			<assets::AccountAssets<Test>>::get((dot_type, bob)).balance,
			bob_dot_issued - bob_dot_convert1.0 - bob_dot_convert2.0 * 2 - bob_dot_convert3.0 - bob_dot_convert4.0
		);
		// check bob's vdot change
		assert_eq!(
			<assets::AccountAssets<Test>>::get((vdot_type, bob)).balance,
			bob_vdot_issued + rate * (bob_dot_convert1.0 + bob_dot_convert2.0 * 2 + bob_dot_convert3.0 + bob_dot_convert4.0)
		);
		// check bob's refers
		assert_eq!(
			ReferrerChannels::<Test>::get(bob),
			(
				vec![(referer1, bob_dot_convert1.0 * rate), (referer2, bob_dot_convert2.0 * 2 * rate), (referer3, bob_dot_convert3.0 * rate)],
				(bob_dot_convert1.0 + bob_dot_convert2.0 * 2 + bob_dot_convert3.0) * rate
			)
		);

		// issue dot/vdot to alice
		let alice_vdot_issued = 50;
		let alice_dot_issued = 80;
		assert_ok!(assets::Module::<Test>::issue(Origin::root(), vdot_type, alice, alice_vdot_issued));
		assert_ok!(assets::Module::<Test>::issue(Origin::root(), dot_type, alice, alice_dot_issued));

		let alice_dot_convert1 = (2, referer2);
		let alice_dot_convert2 = (4, referer4);
		let alice_dot_convert3 = (3, 0); // 0 means no referer

		assert_ok!(Convert::convert_token_to_vtoken(Origin::signed(alice), vdot_type, alice_dot_convert1.0, Some(alice_dot_convert1.1)));
		assert_ok!(Convert::convert_token_to_vtoken(Origin::signed(alice), vdot_type, alice_dot_convert2.0, Some(alice_dot_convert2.1)));
		assert_ok!(Convert::convert_token_to_vtoken(Origin::signed(alice), vdot_type, alice_dot_convert3.0, None)); // no referer

		// check alice's dot change
		assert_eq!(
			<assets::AccountAssets<Test>>::get((dot_type, alice)).balance,
			alice_dot_issued - alice_dot_convert1.0 - alice_dot_convert2.0 - alice_dot_convert3.0
		);
		// check alice's vdot change
		assert_eq!(
			<assets::AccountAssets<Test>>::get((vdot_type, alice)).balance,
			alice_vdot_issued + rate * (alice_dot_convert1.0 + alice_dot_convert2.0 + alice_dot_convert3.0)
		);
		// check alice's refers
		assert_eq!(
			ReferrerChannels::<Test>::get(alice),
			(
				vec![(referer2, alice_dot_convert1.0 * rate), (referer4, alice_dot_convert2.0 * rate)],
				(alice_dot_convert1.0 + alice_dot_convert2.0) * rate
			)
		);

		let all_channels: BTreeMap<u64, u64> = [
			(referer1, bob_dot_convert1.0 * rate),
			(referer2, (bob_dot_convert2.0 * 2 + alice_dot_convert1.0) * rate),
			(referer3, bob_dot_convert3.0 * rate),
			(referer4, alice_dot_convert2.0 * rate)
		].iter().cloned().collect();

		// check all channels
		assert_eq!(
			AllReferrerChannels::<Test>::get(),
			(
				all_channels,
				(bob_dot_convert1.0 + bob_dot_convert2.0 * 2 + bob_dot_convert3.0 + alice_dot_convert1.0 + alice_dot_convert2.0) * rate
			)
		);

		// now convert vdot to dot
		let alice_vdot = 5;
		assert_ok!(Convert::convert_vtoken_to_token(Origin::signed(alice), dot_type, alice_vdot));
		let all_channels: BTreeMap<u64, u64> = [
			(referer1, bob_dot_convert1.0 * rate),
			(referer2, (bob_dot_convert2.0 * 2 + alice_dot_convert1.0) * rate - alice_vdot),
			(referer3, bob_dot_convert3.0 * rate),
			(referer4, alice_dot_convert2.0 * rate)
		].iter().cloned().collect();

		assert_eq!(
			AllReferrerChannels::<Test>::get(),
			(
				all_channels,
				(bob_dot_convert1.0 + bob_dot_convert2.0 * 2 + bob_dot_convert3.0 + alice_dot_convert1.0 + alice_dot_convert2.0) * rate - alice_vdot
			)
		);

		assert_eq!(
			ReferrerChannels::<Test>::get(alice),
			(
				vec![(referer2, 0), (referer4, alice_dot_convert2.0 * rate - 1)], // 5 = 4 + 1, 4 - 4 = 0, 8 - 1 = 7
				(alice_dot_convert1.0 + alice_dot_convert2.0) * rate - alice_vdot
			)
		);
	});
}
