// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
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

use crate::{
	agents::{AstarCall, AstarDappsStakingCall, SmartContract},
	mocks::mock_kusama::*,
	*,
};

const SUBACCOUNT_0_32: [u8; 32] =
	hex_literal::hex!["5a53736d8e96f1c007cf0d630acf5209b20611617af23ce924c8e25328eb5d28"];
const SUBACCOUNT_0_LOCATION: MultiLocation =
	MultiLocation { parents: 1, interior: X1(AccountId32 { network: None, id: SUBACCOUNT_0_32 }) };

#[test]
fn test_construct_lock_xcm() {
	ExtBuilder::default().build().execute_with(|| {
		DelegatorsMultilocation2Index::<Runtime>::insert(ASTR, SUBACCOUNT_0_LOCATION, 0);
		let call =
			AstarCall::Staking(AstarDappsStakingCall::<Runtime>::Lock(1e12 as u128)).encode();
		let transact_call_data =
			Pallet::<Runtime>::prepare_send_as_subaccount_call(call, &SUBACCOUNT_0_LOCATION, ASTR)
				.unwrap();
		let hex_string = hex::encode(&transact_call_data);
		assert_eq!(hex_string, "0b0100002207070010a5d4e8");
	});
}

#[test]
fn test_construct_unlock_xcm() {
	ExtBuilder::default().build().execute_with(|| {
		DelegatorsMultilocation2Index::<Runtime>::insert(ASTR, SUBACCOUNT_0_LOCATION, 0);
		let call =
			AstarCall::Staking(AstarDappsStakingCall::<Runtime>::Unlock(1e12 as u128)).encode();
		let transact_call_data =
			Pallet::<Runtime>::prepare_send_as_subaccount_call(call, &SUBACCOUNT_0_LOCATION, ASTR)
				.unwrap();
		let hex_string = hex::encode(&transact_call_data);
		assert_eq!(hex_string, "0b0100002208070010a5d4e8");
	});
}

#[test]
fn test_construct_stake_xcm() {
	ExtBuilder::default().build().execute_with(|| {
		DelegatorsMultilocation2Index::<Runtime>::insert(ASTR, SUBACCOUNT_0_LOCATION, 0);
		let call = AstarCall::Staking(AstarDappsStakingCall::<Runtime>::Stake(
			SmartContract::Evm(H160::default()),
			1e12 as u128,
		))
		.encode();
		let transact_call_data =
			Pallet::<Runtime>::prepare_send_as_subaccount_call(call, &SUBACCOUNT_0_LOCATION, ASTR)
				.unwrap();
		let hex_string = hex::encode(&transact_call_data);
		assert_eq!(
			hex_string,
			"0b010000220b000000000000000000000000000000000000000000070010a5d4e8"
		);
	});
}

#[test]
fn test_construct_unstake_xcm() {
	ExtBuilder::default().build().execute_with(|| {
		DelegatorsMultilocation2Index::<Runtime>::insert(ASTR, SUBACCOUNT_0_LOCATION, 0);
		let call = AstarCall::Staking(AstarDappsStakingCall::<Runtime>::Unstake(
			SmartContract::Evm(H160::default()),
			1e12 as u128,
		))
		.encode();
		let transact_call_data =
			Pallet::<Runtime>::prepare_send_as_subaccount_call(call, &SUBACCOUNT_0_LOCATION, ASTR)
				.unwrap();
		let hex_string = hex::encode(&transact_call_data);
		assert_eq!(
			hex_string,
			"0b010000220c000000000000000000000000000000000000000000070010a5d4e8"
		);
	});
}

#[test]
fn test_construct_claim_unlock_xcm() {
	ExtBuilder::default().build().execute_with(|| {
		DelegatorsMultilocation2Index::<Runtime>::insert(ASTR, SUBACCOUNT_0_LOCATION, 0);
		let call = AstarCall::Staking(AstarDappsStakingCall::<Runtime>::ClaimUnlocked).encode();
		let transact_call_data =
			Pallet::<Runtime>::prepare_send_as_subaccount_call(call, &SUBACCOUNT_0_LOCATION, ASTR)
				.unwrap();
		let hex_string = hex::encode(&transact_call_data);
		assert_eq!(hex_string, "0b0100002209");
	});
}

#[test]
fn test_construct_claim_staker_rewards_xcm() {
	ExtBuilder::default().build().execute_with(|| {
		DelegatorsMultilocation2Index::<Runtime>::insert(ASTR, SUBACCOUNT_0_LOCATION, 0);
		let call =
			AstarCall::Staking(AstarDappsStakingCall::<Runtime>::ClaimStakerRewards).encode();
		let transact_call_data =
			Pallet::<Runtime>::prepare_send_as_subaccount_call(call, &SUBACCOUNT_0_LOCATION, ASTR)
				.unwrap();
		let hex_string = hex::encode(&transact_call_data);
		assert_eq!(hex_string, "0b010000220d");
	});
}

#[test]
fn test_construct_claim_bonus_rewards_xcm() {
	ExtBuilder::default().build().execute_with(|| {
		DelegatorsMultilocation2Index::<Runtime>::insert(ASTR, SUBACCOUNT_0_LOCATION, 0);
		let call = AstarCall::Staking(AstarDappsStakingCall::<Runtime>::ClaimBonusReward(
			SmartContract::Evm(H160::default()),
		))
		.encode();
		let transact_call_data =
			Pallet::<Runtime>::prepare_send_as_subaccount_call(call, &SUBACCOUNT_0_LOCATION, ASTR)
				.unwrap();
		let hex_string = hex::encode(&transact_call_data);
		assert_eq!(hex_string, "0b010000220e000000000000000000000000000000000000000000");
	});
}
