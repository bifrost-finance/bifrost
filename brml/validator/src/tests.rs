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
use frame_support::{assert_ok, assert_noop};
use node_primitives::{
	Token, TokenPair, TokenType,
};

#[test]
fn set_asset_should_work() {
	new_test_ext().execute_with(|| {
		let asset_symbol = AssetSymbol::EOS;
		let redeem_duration = 100;
		let min_reward_per_block = 1;
		let asset_config = AssetConfig::new(redeem_duration, min_reward_per_block);

		assert_ok!(
			Validator::set_asset(Origin::ROOT, asset_symbol, redeem_duration, min_reward_per_block)
		);

		assert_eq!(Validator::asset_configs(asset_symbol), asset_config);
	});
}

#[test]
fn staking_should_ok() {
	new_test_ext().execute_with(|| {
		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let asset_symbol = AssetSymbol::EOS;
		let need = 1000;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(Validator::register(origin, asset_symbol, need, validator_address));

		let target = 1;
		let amount = 100;
		assert_ok!(Validator::staking(Origin::ROOT, asset_symbol, target, amount));
		let validator = Validator::validators(asset_symbol, origin_id);
		assert_eq!(validator.staking, 100);
		let asset_locked_balance = Validator::asset_locked_balances(asset_symbol);
		assert_eq!(asset_locked_balance, 100);

		let target = 1;
		let amount = 200;
		assert_ok!(Validator::staking(Origin::ROOT, asset_symbol, target, amount));
		let validator = Validator::validators(asset_symbol, origin_id);
		assert_eq!(validator.staking, 300);
		let asset_locked_balance = Validator::asset_locked_balances(asset_symbol);
		assert_eq!(asset_locked_balance, 300);
	});
}

#[test]
fn staking_not_registered_should_error() {
	new_test_ext().execute_with(|| {
		let asset_symbol = AssetSymbol::EOS;
		let target = 1;
		let amount = 100;

		assert_noop!(
			Validator::staking(Origin::ROOT, asset_symbol, target, amount),
			ValidatorError::ValidatorNotRegistered
		);
	});
}

#[test]
fn staking_amount_exceed_should_error() {
	new_test_ext().execute_with(|| {
		let origin = Origin::signed(1);
		let asset_symbol = AssetSymbol::EOS;
		let need = 1000;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(Validator::register(origin, asset_symbol, need, validator_address));

		let target = 1;
		let amount = 2000;
		assert_noop!(
			Validator::staking(Origin::ROOT, asset_symbol, target, amount),
			ValidatorError::StakingAmountExceeded
		);
	});
}

#[test]
fn unstaking_should_ok() {
	new_test_ext().execute_with(|| {
		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let asset_symbol = AssetSymbol::EOS;
		let need = 1000;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(Validator::register(origin, asset_symbol, need, validator_address));

		let target = 1;
		let stake_amount = 500;
		assert_ok!(Validator::staking(Origin::ROOT, asset_symbol, target, stake_amount));

		let target = 1;
		let unstake_amount = 200;
		assert_ok!(Validator::unstaking(Origin::ROOT, asset_symbol, target, unstake_amount));
		let validator = Validator::validators(asset_symbol, origin_id);
		assert_eq!(validator.staking, 300);
		let asset_locked_balance = Validator::asset_locked_balances(asset_symbol);
		assert_eq!(asset_locked_balance, 300);
	});
}

#[test]
fn unstaking_not_registered_should_error() {
	new_test_ext().execute_with(|| {
		let asset_symbol = AssetSymbol::EOS;
		let target = 1;
		let amount = 100;

		assert_noop!(
			Validator::unstaking(Origin::ROOT, asset_symbol, target, amount),
			ValidatorError::ValidatorNotRegistered
		);
	});
}

#[test]
fn unstaking_insufficient_should_error() {
	new_test_ext().execute_with(|| {
		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let asset_symbol = AssetSymbol::EOS;
		let need = 1000;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(Validator::register(origin, asset_symbol, need, validator_address));

		let target = 1;
		let stake_amount = 500;
		assert_ok!(Validator::staking(Origin::ROOT, asset_symbol, target, stake_amount));

		let target = 1;
		let unstake_amount = 1000;
		assert_noop!(
			Validator::unstaking(Origin::ROOT, asset_symbol, target, unstake_amount),
			ValidatorError::StakingAmountInsufficient
		);
	});
}

#[test]
fn register_should_work() {
	new_test_ext().execute_with(|| {
		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let asset_symbol = AssetSymbol::EOS;
		let need = 1000;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		let validator = ValidatorRegister::new(need, validator_address.clone());

		assert_ok!(Validator::register(origin, asset_symbol, need, validator_address));

		assert_eq!(Validator::validators(asset_symbol, origin_id), validator);
	});
}

#[test]
fn register_twice_should_error() {
	new_test_ext().execute_with(|| {
		let origin = Origin::signed(1);
		let asset_symbol = AssetSymbol::EOS;
		let need = 1000;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];

		assert_ok!(
			Validator::register(origin.clone(), asset_symbol, need, validator_address.clone())
		);

		assert_noop!(
			Validator::register(origin, asset_symbol, need, validator_address),
			ValidatorError::ValidatorRegistered
		);
	});
}

#[test]
fn set_need_amount_should_work() {
	new_test_ext().execute_with(|| {
		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let asset_symbol = AssetSymbol::EOS;
		let need = 1000;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(Validator::register(origin.clone(), asset_symbol, need, validator_address));

		let new_need = 2000;
		assert_ok!(Validator::set_need_amount(origin, asset_symbol, new_need));
		let validator = Validator::validators(asset_symbol, origin_id);
		assert_eq!(validator.need, new_need);
	});
}

#[test]
fn set_need_amount_not_registered_should_error() {
	new_test_ext().execute_with(|| {
		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let asset_symbol = AssetSymbol::EOS;
		let new_need = 2000;

		assert_noop!(
			Validator::set_need_amount(origin, asset_symbol, new_need),
			ValidatorError::ValidatorNotRegistered
		);
	});
}

#[test]
fn deposit_should_work() {
	new_test_ext().execute_with(|| {
		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let asset_symbol = AssetSymbol::EOS;
		let asset_id: u32 = asset_symbol.into();
		let token = Token {
			symbol: vec![0x12, 0x34],
			precision: 8,
			total_supply: 0,
		};
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_ok!(Assets::issue(Origin::ROOT, asset_symbol, TokenType::Token, origin_id, 10000));
		assert_eq!(Assets::token_details(asset_id), TokenPair::new(
			Token { total_supply: 10000, ..token.clone() },
			token.clone(),
		));

		let need = 1000;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(Validator::register(origin.clone(), asset_symbol, need, validator_address));

		let deposit_amount = 100;
		assert_ok!(Validator::deposit(origin.clone(), asset_symbol, deposit_amount));
		let validator = Validator::validators(asset_symbol, origin_id);
		assert_eq!(validator.deposit, 100);
		assert_eq!(Assets::token_details(asset_id), TokenPair::new(
			Token { total_supply: 9900, ..token.clone() },
			token.clone(),
		));

		let deposit_amount = 200;
		assert_ok!(Validator::deposit(origin, asset_symbol, deposit_amount));
		let validator = Validator::validators(asset_symbol, origin_id);
		assert_eq!(validator.deposit, 300);
		assert_eq!(Assets::token_details(asset_id), TokenPair::new(
			Token { total_supply: 9700, ..token.clone() },
			token.clone(),
		));
	});
}

#[test]
fn deposit_not_enough_free_balance_should_error() {
	new_test_ext().execute_with(|| {
		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let asset_symbol = AssetSymbol::EOS;
		let asset_id: u32 = asset_symbol.into();
		let token = Token {
			symbol: vec![0x12, 0x34],
			precision: 8,
			total_supply: 0,
		};
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_ok!(Assets::issue(Origin::ROOT, asset_symbol, TokenType::Token, origin_id, 10000));
		assert_eq!(Assets::token_details(asset_id), TokenPair::new(
			Token { total_supply: 10000, ..token.clone() },
			token.clone(),
		));

		let need = 1000;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(Validator::register(origin.clone(), asset_symbol, need, validator_address));

		let deposit_amount = 20000;
		assert_noop!(
			Validator::deposit(origin.clone(), asset_symbol, deposit_amount),
			ValidatorError::FreeBalanceNotEnough
		);
	});
}

#[test]
fn deposit_not_registered_should_error() {
	new_test_ext().execute_with(|| {
		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let asset_symbol = AssetSymbol::EOS;
		let deposit_amount = 100;

		assert_noop!(
			Validator::deposit(origin, asset_symbol, deposit_amount),
			ValidatorError::ValidatorNotRegistered
		);
	});
}

#[test]
fn withdraw_should_ok() {
	new_test_ext().execute_with(|| {
		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let asset_symbol = AssetSymbol::EOS;
		let asset_id: u32 = asset_symbol.into();
		let token = Token {
			symbol: vec![0x12, 0x34],
			precision: 8,
			total_supply: 0,
		};
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_ok!(Assets::issue(Origin::ROOT, asset_symbol, TokenType::Token, origin_id, 10000));
		assert_eq!(Assets::token_details(asset_id), TokenPair::new(
			Token { total_supply: 10000, ..token.clone() },
			token.clone(),
		));

		let need = 1000;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(Validator::register(origin.clone(), asset_symbol, need, validator_address));

		let deposit_amount = 500;
		assert_ok!(Validator::deposit(origin.clone(), asset_symbol, deposit_amount));
		let validator = Validator::validators(asset_symbol, origin_id);

		let withdraw_amount = 200;
		assert_ok!(Validator::withdraw(origin, asset_symbol, withdraw_amount));
		let validator = Validator::validators(asset_symbol, origin_id);
		assert_eq!(validator.deposit, 300);
		assert_eq!(Assets::token_details(asset_id), TokenPair::new(
			Token { total_supply: 9700, ..token.clone() },
			token.clone(),
		));
	});
}

#[test]
fn withdraw_not_enough_locked_balance_should_error() {
	new_test_ext().execute_with(|| {
		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let asset_symbol = AssetSymbol::EOS;
		let asset_id: u32 = asset_symbol.into();
		let token = Token {
			symbol: vec![0x12, 0x34],
			precision: 8,
			total_supply: 0,
		};
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_ok!(Assets::issue(Origin::ROOT, asset_symbol, TokenType::Token, origin_id, 10000));
		assert_eq!(Assets::token_details(asset_id), TokenPair::new(
			Token { total_supply: 10000, ..token.clone() },
			token.clone(),
		));

		let need = 1000;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(Validator::register(origin.clone(), asset_symbol, need, validator_address));

		let deposit_amount = 500;
		assert_ok!(Validator::deposit(origin.clone(), asset_symbol, deposit_amount));
		let validator = Validator::validators(asset_symbol, origin_id);

		let withdraw_amount = 1000;
		assert_noop!(
			Validator::withdraw(origin, asset_symbol, withdraw_amount),
			ValidatorError::LockedBalanceNotEnough
		);
	});
}

#[test]
fn withdraw_not_registered_should_error() {
	new_test_ext().execute_with(|| {
		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let asset_symbol = AssetSymbol::EOS;
		let deposit_amount = 100;

		assert_noop!(
			Validator::withdraw(origin, asset_symbol, deposit_amount),
			ValidatorError::ValidatorNotRegistered
		);
	});
}