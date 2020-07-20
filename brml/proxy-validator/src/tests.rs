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
	Token, TokenSymbol,
};

fn set_global_asset(token_symbol: TokenSymbol) {
	let redeem_duration = 100;
	let min_reward_per_block = 1;
	ProxyValidator::set_global_asset(Origin::root(), token_symbol, redeem_duration, min_reward_per_block).unwrap();
}

#[test]
fn set_asset_should_work() {
	new_test_ext().execute_with(|| {
		let token_symbol = TokenSymbol::EOS;
		let redeem_duration = 100;
		let min_reward_per_block = 1;
		let asset_config = AssetConfig::new(redeem_duration, min_reward_per_block);

		assert_ok!(
			ProxyValidator::set_global_asset(Origin::root(), token_symbol, redeem_duration, min_reward_per_block)
		);

		assert_eq!(ProxyValidator::asset_configs(token_symbol), asset_config);
	});
}

#[test]
fn stake_should_ok() {
	new_test_ext().execute_with(|| {
		set_global_asset(TokenSymbol::EOS);

		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let token_symbol = TokenSymbol::EOS;
		let need = 1000;
		let reward_per_block = 10;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(ProxyValidator::validator_register(origin, token_symbol, need, reward_per_block, validator_address));

		let target = 1;
		let amount = 100;
		assert_ok!(ProxyValidator::stake(Origin::root(), token_symbol, target, amount));
		let validator = ProxyValidator::validators(token_symbol, origin_id);
		assert_eq!(validator.staking, 100);
		let asset_locked_balance = ProxyValidator::asset_locked_balances(token_symbol);
		assert_eq!(asset_locked_balance, 100);

		let target = 1;
		let amount = 200;
		assert_ok!(ProxyValidator::stake(Origin::root(), token_symbol, target, amount));
		let validator = ProxyValidator::validators(token_symbol, origin_id);
		assert_eq!(validator.staking, 300);
		let asset_locked_balance = ProxyValidator::asset_locked_balances(token_symbol);
		assert_eq!(asset_locked_balance, 300);
	});
}

#[test]
fn stake_not_registered_should_error() {
	new_test_ext().execute_with(|| {
		let token_symbol = TokenSymbol::EOS;
		let target = 1;
		let amount = 100;

		assert_noop!(
			ProxyValidator::stake(Origin::root(), token_symbol, target, amount),
			ProxyValidatorError::ProxyValidatorNotRegistered
		);
	});
}

#[test]
fn stake_amount_exceed_should_error() {
	new_test_ext().execute_with(|| {
		set_global_asset(TokenSymbol::EOS);

		let origin = Origin::signed(1);
		let token_symbol = TokenSymbol::EOS;
		let need = 1000;
		let reward_per_block = 10;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(ProxyValidator::validator_register(origin, token_symbol, need, reward_per_block, validator_address));

		let target = 1;
		let amount = 2000;
		assert_noop!(
			ProxyValidator::stake(Origin::root(), token_symbol, target, amount),
			ProxyValidatorError::StakingAmountExceeded
		);
	});
}

#[test]
fn unstake_should_ok() {
	new_test_ext().execute_with(|| {
		set_global_asset(TokenSymbol::EOS);

		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let token_symbol = TokenSymbol::EOS;
		let need = 1000;
		let reward_per_block = 10;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(ProxyValidator::validator_register(origin, token_symbol, need, reward_per_block, validator_address));

		let target = 1;
		let stake_amount = 500;
		assert_ok!(ProxyValidator::stake(Origin::root(), token_symbol, target, stake_amount));

		let target = 1;
		let unstake_amount = 200;
		assert_ok!(ProxyValidator::unstake(Origin::root(), token_symbol, target, unstake_amount));
		let validator = ProxyValidator::validators(token_symbol, origin_id);
		assert_eq!(validator.staking, 300);
		let asset_locked_balance = ProxyValidator::asset_locked_balances(token_symbol);
		assert_eq!(asset_locked_balance, 300);
	});
}

#[test]
fn unstake_not_registered_should_error() {
	new_test_ext().execute_with(|| {
		let token_symbol = TokenSymbol::EOS;
		let target = 1;
		let amount = 100;

		assert_noop!(
			ProxyValidator::unstake(Origin::root(), token_symbol, target, amount),
			ProxyValidatorError::ProxyValidatorNotRegistered
		);
	});
}

#[test]
fn unstake_insufficient_should_error() {
	new_test_ext().execute_with(|| {
		set_global_asset(TokenSymbol::EOS);

		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let token_symbol = TokenSymbol::EOS;
		let need = 1000;
		let reward_per_block = 10;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(ProxyValidator::validator_register(origin, token_symbol, need, reward_per_block, validator_address));

		let target = 1;
		let stake_amount = 500;
		assert_ok!(ProxyValidator::stake(Origin::root(), token_symbol, target, stake_amount));

		let target = 1;
		let unstake_amount = 1000;
		assert_noop!(
			ProxyValidator::unstake(Origin::root(), token_symbol, target, unstake_amount),
			ProxyValidatorError::StakingAmountInsufficient
		);
	});
}

#[test]
fn register_should_work() {
	new_test_ext().execute_with(|| {
		set_global_asset(TokenSymbol::EOS);

		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let token_symbol = TokenSymbol::EOS;
		let need = 1000;
		let reward_per_block = 100;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		let validator = ProxyValidatorRegister::new(need, reward_per_block, validator_address.clone());

		assert_ok!(ProxyValidator::validator_register(origin, token_symbol, need, reward_per_block, validator_address));

		assert_eq!(ProxyValidator::validators(token_symbol, origin_id), validator);
	});
}

#[test]
fn register_asset_not_set_should_error() {
	new_test_ext().execute_with(|| {
		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let token_symbol = TokenSymbol::EOS;
		let need = 1000;
		let reward_per_block = 100;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];

		assert_noop!(
			ProxyValidator::validator_register(origin, token_symbol, need, reward_per_block, validator_address),
			ProxyValidatorError::AssetConfigNotSet
		);
	});
}

#[test]
fn register_twice_should_error() {
	new_test_ext().execute_with(|| {
		set_global_asset(TokenSymbol::EOS);

		let origin = Origin::signed(1);
		let token_symbol = TokenSymbol::EOS;
		let need = 1000;
		let reward_per_block = 10;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];

		assert_ok!(
			ProxyValidator::validator_register(origin.clone(), token_symbol, need, reward_per_block, validator_address.clone())
		);

		assert_noop!(
			ProxyValidator::validator_register(origin, token_symbol, need, Zero::zero(), validator_address),
			ProxyValidatorError::ProxyValidatorRegistered
		);
	});
}

#[test]
fn register_reward_too_low_should_error() {
	new_test_ext().execute_with(|| {
		set_global_asset(TokenSymbol::EOS);

		let origin = Origin::signed(1);
		let token_symbol = TokenSymbol::EOS;
		let need = 1000;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];

		assert_noop!(
			ProxyValidator::validator_register(origin.clone(), token_symbol, need, 0, validator_address.clone()),
			ProxyValidatorError::RewardTooLow
		);
	});
}

#[test]
fn set_need_amount_should_work() {
	new_test_ext().execute_with(|| {
		set_global_asset(TokenSymbol::EOS);

		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let token_symbol = TokenSymbol::EOS;
		let need = 1000;
		let reward_per_block = 10;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(ProxyValidator::validator_register(origin.clone(), token_symbol, need, reward_per_block, validator_address));

		let new_need = 2000;
		assert_ok!(ProxyValidator::set_need_amount(origin, token_symbol, new_need));
		let validator = ProxyValidator::validators(token_symbol, origin_id);
		assert_eq!(validator.need, new_need);
	});
}

#[test]
fn set_need_amount_not_registered_should_error() {
	new_test_ext().execute_with(|| {
		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let token_symbol = TokenSymbol::EOS;
		let new_need = 2000;

		assert_noop!(
			ProxyValidator::set_need_amount(origin, token_symbol, new_need),
			ProxyValidatorError::ProxyValidatorNotRegistered
		);
	});
}

#[test]
fn set_reward_per_block_should_work() {
	new_test_ext().execute_with(|| {
		set_global_asset(TokenSymbol::EOS);

		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let token_symbol = TokenSymbol::EOS;
		let need = 1000;
		let reward_per_block = 10;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(ProxyValidator::validator_register(origin.clone(), token_symbol, need, reward_per_block, validator_address));

		let reward_per_block = 200000;
		assert_ok!(ProxyValidator::set_reward_per_block(origin, token_symbol, reward_per_block));
		let validator = ProxyValidator::validators(token_symbol, origin_id);
		assert_eq!(validator.reward_per_block, reward_per_block);
	});
}

#[test]
fn set_reward_per_block_not_registered_should_error() {
	new_test_ext().execute_with(|| {
		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let token_symbol = TokenSymbol::EOS;
		let reward_per_block = 200000;

		assert_noop!(
			ProxyValidator::set_reward_per_block(origin, token_symbol, reward_per_block),
			ProxyValidatorError::ProxyValidatorNotRegistered
		);
	});
}

#[test]
fn set_reward_per_block_reward_too_low_should_error() {
	new_test_ext().execute_with(|| {
		set_global_asset(TokenSymbol::EOS);

		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let token_symbol = TokenSymbol::EOS;
		let need = 1000;
		let reward_per_block = 10;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(ProxyValidator::validator_register(origin.clone(), token_symbol, need, reward_per_block, validator_address));

		let reward_per_block = 0;
		assert_noop!(
			ProxyValidator::set_reward_per_block(origin, token_symbol, reward_per_block),
			ProxyValidatorError::RewardTooLow
		);
	});
}


#[test]
fn deposit_should_work() {
	new_test_ext().execute_with(|| {
		set_global_asset(TokenSymbol::DOT);

		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let precision = 8;

		assert_ok!(Assets::create(Origin::root(), b"aUSD".to_vec(), 18)); // let dot start from 1

		assert_ok!(Assets::create(Origin::root(), b"DOT".to_vec(), precision));
		let dot_id = Assets::next_asset_id() - 1;
		let dot_type = TokenSymbol::from(dot_id);
		assert_ok!(Assets::issue(Origin::root(), dot_type, origin_id, 10000));
		assert_eq!(Assets::token_details(dot_type), Token::new(b"DOT".to_vec(), 8, 10000));

		let need = 1000;
		let reward_per_block = 10;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(ProxyValidator::validator_register(origin.clone(), dot_type, need, reward_per_block, validator_address));

		let deposit_amount = 100;
		assert_ok!(ProxyValidator::deposit(origin.clone(), dot_type, deposit_amount));
		let validator = ProxyValidator::validators(dot_type, origin_id);
		assert_eq!(validator.deposit, 100);
		assert_eq!(Assets::token_details(dot_type), Token::new(b"DOT".to_vec(), 8, 9900));

		let deposit_amount = 200;
		assert_ok!(ProxyValidator::deposit(origin, dot_type, deposit_amount));
		let validator = ProxyValidator::validators(dot_type, origin_id);
		assert_eq!(validator.deposit, 300);
		assert_eq!(Assets::token_details(dot_type), Token::new(b"DOT".to_vec(), 8, 9700));
	});
}

#[test]
fn deposit_not_enough_free_balance_should_error() {
	new_test_ext().execute_with(|| {
		set_global_asset(TokenSymbol::DOT);

		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let precision = 8;

		assert_ok!(Assets::create(Origin::root(), b"aUSD".to_vec(), 18)); // let dot start from 1
		assert_ok!(Assets::create(Origin::root(), b"DOT".to_vec(), precision));

		let dot_id = Assets::next_asset_id() - 1;
		let dot_type = TokenSymbol::from(dot_id);

		assert_ok!(Assets::issue(Origin::root(), dot_type, origin_id, 10000));
		assert_eq!(Assets::token_details(dot_type), Token::new(b"DOT".to_vec(), 8, 10000));

		let need = 1000;
		let reward_per_block = 10;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(ProxyValidator::validator_register(origin.clone(), dot_type, need, reward_per_block, validator_address));

		let deposit_amount = 20000;
		assert_noop!(
			ProxyValidator::deposit(origin.clone(), dot_type, deposit_amount),
			ProxyValidatorError::FreeBalanceNotEnough
		);
	});
}

#[test]
fn deposit_not_registered_should_error() {
	new_test_ext().execute_with(|| {
		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let token_symbol = TokenSymbol::EOS;
		let deposit_amount = 100;

		assert_noop!(
			ProxyValidator::deposit(origin, token_symbol, deposit_amount),
			ProxyValidatorError::ProxyValidatorNotRegistered
		);
	});
}

#[test]
fn withdraw_should_ok() {
	new_test_ext().execute_with(|| {
		set_global_asset(TokenSymbol::DOT);

		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let precision = 8;

		assert_ok!(Assets::create(Origin::root(), b"aUSD".to_vec(), 18)); // let dot start from 1
		assert_ok!(Assets::create(Origin::root(), b"DOT".to_vec(), precision));

		let dot_id = Assets::next_asset_id() - 1;
		let dot_type = TokenSymbol::from(dot_id);

		assert_ok!(Assets::issue(Origin::root(), dot_type, origin_id, 10000));
		assert_eq!(Assets::token_details(dot_type), Token::new(b"DOT".to_vec(), 8, 10000));

		let need = 1000;
		let reward_per_block = 10;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(ProxyValidator::validator_register(origin.clone(), dot_type, need, reward_per_block, validator_address));

		let deposit_amount = 500;
		assert_ok!(ProxyValidator::deposit(origin.clone(), dot_type, deposit_amount));

		let withdraw_amount = 200;
		assert_ok!(ProxyValidator::withdraw(origin, dot_type, withdraw_amount));
		let validator = ProxyValidator::validators(dot_type, origin_id);
		assert_eq!(validator.deposit, 300);
		assert_eq!(Assets::token_details(dot_type), Token::new(b"DOT".to_vec(), 8, 9700));
	});
}

#[test]
fn withdraw_not_enough_locked_balance_should_error() {
	new_test_ext().execute_with(|| {
		set_global_asset(TokenSymbol::DOT);

		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let precision = 8;

		assert_ok!(Assets::create(Origin::root(), b"aUSD".to_vec(), 18)); // let dot start from 1
		assert_ok!(Assets::create(Origin::root(), b"DOT".to_vec(), precision));

		let dot_id = Assets::next_asset_id() - 1;
		let dot_type = TokenSymbol::from(dot_id);

		assert_ok!(Assets::issue(Origin::root(), dot_type, origin_id, 10000));
		assert_eq!(Assets::token_details(dot_type), Token::new(b"DOT".to_vec(), 8, 10000));

		let need = 1000;
		let reward_per_block = 10;
		let validator_address = vec![0x12, 0x34, 0x56, 0x78];
		assert_ok!(ProxyValidator::validator_register(origin.clone(), dot_type, need, reward_per_block, validator_address));

		let deposit_amount = 500;
		assert_ok!(ProxyValidator::deposit(origin.clone(), dot_type, deposit_amount));

		let withdraw_amount = 1000;
		assert_noop!(
			ProxyValidator::withdraw(origin, dot_type, withdraw_amount),
			ProxyValidatorError::LockedBalanceNotEnough
		);
	});
}

#[test]
fn withdraw_not_registered_should_error() {
	new_test_ext().execute_with(|| {
		let origin_id = 1;
		let origin = Origin::signed(origin_id);
		let token_symbol = TokenSymbol::EOS;
		let deposit_amount = 100;

		assert_noop!(
			ProxyValidator::withdraw(origin, token_symbol, deposit_amount),
			ProxyValidatorError::ProxyValidatorNotRegistered
		);
	});
}
