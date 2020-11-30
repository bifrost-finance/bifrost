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
use node_primitives::{TokenSymbol, RewardTrait};
use frame_support::{assert_noop, assert_ok};

#[test]
fn query_vtoken_should_be_ok() {
	new_test_ext().execute_with(|| {
		common();
		let (vtoken_symbol,
			referer_one,
			referer_two,
			staking_profit
		) = (TokenSymbol::vDOT, 11111111 as u64, 22222222 as u64, 60 as u64);
		
		// The first query asset
		let referer_one_vtoken_amount = crate::Point::<Test>::get((vtoken_symbol, referer_one));
		assert_eq!(100, referer_one_vtoken_amount);
		let referer_two_vtoken_amount = crate::Point::<Test>::get((vtoken_symbol, referer_two));
		assert_eq!(200, referer_two_vtoken_amount);
		let referer_one_vtoken_amount = crate::Point::<Test>::get((TokenSymbol::vEOS, referer_one));
		assert_eq!(100, referer_one_vtoken_amount);
		
		// Dispatch TokenSymbol::vDOT reward Success:
		assert_ok!(crate::Module::<Test>::dispatch_reward(vtoken_symbol, staking_profit));
		
		let referer_one_vtoken_amount = crate::Point::<Test>::get((vtoken_symbol, referer_one));
		assert_eq!(0, referer_one_vtoken_amount);
		let referer_two_vtoken_amount = crate::Point::<Test>::get((vtoken_symbol, referer_two));
		assert_eq!(0, referer_two_vtoken_amount);
	});
}

#[test]
fn record_reward_should_be_ok() {
	new_test_ext().execute_with(|| {
		common();
	});
}

pub fn common() {
	// Ready data
	let vdot_id = assets::Module::<Test>::next_asset_id();
	let vtoken_symbol = TokenSymbol::from(vdot_id + 2);
	assert_eq!(vtoken_symbol, TokenSymbol::vDOT);
	// Bind value:" convert_amount、 referer_one、referer_two "
	let convert_amount = 100 as u64;
	let (referer_one, referer_two) = (11111111 as u64, 22222222 as u64);
	
	// Add new referer
	assert_ok!(<crate::Module<Test>>::record_reward(vtoken_symbol, convert_amount, referer_one));
	assert_eq!(1, crate::Module::<Test>::vtoken_reward(vtoken_symbol).len());
	assert_eq!(100, crate::Module::<Test>::vtoken_reward(vtoken_symbol)[0].record_amount);
	
	// Increate different referer
	assert_ok!(<crate::Module<Test>>::record_reward(vtoken_symbol, convert_amount, referer_two));
	assert_eq!(2, crate::Reward::<Test>::get(vtoken_symbol).len());
	assert_eq!(100, crate::Reward::<Test>::get(vtoken_symbol)[1].record_amount);
	
	// Append exist referer
	assert_ok!(<crate::Module<Test>>::record_reward(vtoken_symbol, convert_amount, referer_two));
	assert_eq!(2, <crate::Reward<Test>>::get(vtoken_symbol).len());
	assert_eq!(200, crate::Reward::<Test>::get(vtoken_symbol)[0].record_amount);
	
	// Increate Different vtoken （another one vec）
	let vtoken_symbol = TokenSymbol::vEOS;
	assert_ok!(<crate::Module<Test>>::record_reward(vtoken_symbol, convert_amount, referer_one));
	assert_eq!(1, <crate::Module<Test>>::vtoken_reward(vtoken_symbol).len());
	assert_eq!(100, crate::Module::<Test>::vtoken_reward(vtoken_symbol)[0].record_amount);
}

#[test]
fn dispatch_reward_is_be_ok() {
	new_test_ext().execute_with(|| {
		// Condition initial
		common();
		let (vtoken_symbol,
			referer_one,
			referer_two,
			staking_profit
		) = (TokenSymbol::vDOT, 11111111 as u64, 22222222 as u64, 60 as u64);
		
		// The first query asset
		let referer_one_assets = assets::Module::<Test>::account_assets((vtoken_symbol, referer_one));
		assert_eq!(0, referer_one_assets.balance);
		let referer_two_assets = assets::Module::<Test>::account_assets((vtoken_symbol, referer_two));
		assert_eq!(0, referer_two_assets.balance);
		
		// Dispatch TokenSymbol::vDOT reward Success:
		assert_ok!(crate::Module::<Test>::dispatch_reward(vtoken_symbol, staking_profit));
		
		// Dispatch TokenSymbol::vIOST reward Failure:
		assert_noop!(
			crate::Module::<Test>::dispatch_reward(TokenSymbol::vIOST, staking_profit),
			Error::<Test>::RefererNotExist,
		);
		
		// The second query asset
		let referer_one_assets = assets::Module::<Test>::account_assets((vtoken_symbol, referer_one));
		assert_eq!(20, referer_one_assets.balance);
		let referer_two_assets = assets::Module::<Test>::account_assets((vtoken_symbol, referer_two));
		assert_eq!(40, referer_two_assets.balance);
		
		// Judge vtoken table whether be clear
		assert!(<crate::Module<Test>>::vtoken_reward(vtoken_symbol).is_empty());
	});
}

#[test]
fn more_than_256_dispatch_reward_is_be_ok() {
	new_test_ext().execute_with(|| {
		// Condition initial
		let (vtoken_symbol,
			referer_one,
			referer_two,
			staking_profit
		) = (TokenSymbol::vDOT, 11111111 as u64, 22222222 as u64, 2560 as u64);
		
		// The first query asset
		let referer_one_assets = assets::Module::<Test>::account_assets((vtoken_symbol, referer_one));
		assert_eq!(0, referer_one_assets.balance);
		let referer_two_assets = assets::Module::<Test>::account_assets((vtoken_symbol, referer_two));
		assert_eq!(0, referer_two_assets.balance);
		
		// Add referer_one referer_two data
		assert_ok!(<crate::Module<Test>>::record_reward(vtoken_symbol, 100, referer_one));
		assert_ok!(<crate::Module<Test>>::record_reward(vtoken_symbol, 100, referer_two));
		// Add other referer data
		for i in 1..300 {
			assert_ok!(<crate::Module<Test>>::record_reward(vtoken_symbol, 100, referer_two + i));
		}
		
		// Dispatch TokenSymbol::vDOT reward Success:
		assert_ok!(crate::Module::<Test>::dispatch_reward(vtoken_symbol, staking_profit));
		
		// The second query asset
		let referer_one_assets = assets::Module::<Test>::account_assets((vtoken_symbol, referer_one));
		assert_eq!(10, referer_one_assets.balance);
		let referer_two_assets = assets::Module::<Test>::account_assets((vtoken_symbol, referer_two));
		assert_eq!(10, referer_two_assets.balance);
		
		// The referer After 256 doesn't reward
		let referer_254_assets = assets::Module::<Test>::account_assets((vtoken_symbol, referer_two + 255));
		assert_eq!(0, referer_254_assets.balance);
		
		// Judge vtoken table whether be clear
		assert!(<crate::Module<Test>>::vtoken_reward(vtoken_symbol).is_empty());
	});
}
