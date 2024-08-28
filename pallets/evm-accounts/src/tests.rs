// This file is part of HydraDX-node.

// Copyright (C) 2020-2024  Intergalactic, Limited (GIB).
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};
use hex_literal::hex;

#[test]
fn eth_address_should_convert_to_truncated_address_when_not_bound() {
	ExtBuilder::default().build().execute_with(|| {
		// Arrange
		let evm_address = H160::from(hex!["222222ff7Be76052e023Ec1a306fCca8F9659D80"]);
		let truncated_address = AccountId::from(hex![
			"d2efb4a4ab6b9c7dff8d8e1aa76dc53a0aab3a0d93747eb25db1bb7b08a76a09"
		]);

		assert_eq!(EVMAccounts::convert_account_id(evm_address), truncated_address.clone());

		// Act & Assert
		assert_eq!(EVMAccounts::bound_account_id(evm_address), None);
		assert_eq!(EVMAccounts::account_id(evm_address), truncated_address);
	});
}

#[test]
fn eth_address_should_convert_to_full_address_when_bound() {
	ExtBuilder::default().build().execute_with(|| {
		// Arrange & Act
		assert_ok!(EVMAccounts::bind_evm_address(RuntimeOrigin::signed(ALICE),));

		// Assert
		let evm_address = EVMAccounts::evm_address(&ALICE);

		assert_eq!(EVMAccounts::bound_account_id(evm_address), Some(ALICE));

		assert_eq!(EVMAccounts::account_id(evm_address), ALICE);
	});
}

#[test]
fn bind_address_should_fail_when_nonce_is_not_zero() {
	ExtBuilder::default().with_non_zero_nonce(ALICE).build().execute_with(|| {
		assert_noop!(
			EVMAccounts::bind_evm_address(RuntimeOrigin::signed(ALICE)),
			Error::<Test>::TruncatedAccountAlreadyUsed
		);
	});
}

#[test]
fn bind_address_should_fail_when_already_bound() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(EVMAccounts::bind_evm_address(RuntimeOrigin::signed(ALICE),));
		assert_noop!(
			EVMAccounts::bind_evm_address(RuntimeOrigin::signed(ALICE)),
			Error::<Test>::AddressAlreadyBound
		);
	});
}

#[test]
fn add_contract_deployer_should_store_address_in_the_storage() {
	ExtBuilder::default().build().execute_with(|| {
		// Arrange
		let evm_address = EVMAccounts::evm_address(&ALICE);
		assert!(!EVMAccounts::can_deploy_contracts(evm_address));

		// Act
		assert_ok!(EVMAccounts::add_contract_deployer(RuntimeOrigin::root(), evm_address));

		// Assert
		assert!(EVMAccounts::can_deploy_contracts(evm_address));

		// adding the address again should be ok
		assert_ok!(EVMAccounts::add_contract_deployer(RuntimeOrigin::root(), evm_address));
	});
}

#[test]
fn remove_contract_deployer_should_remove_address_from_the_storage() {
	ExtBuilder::default().build().execute_with(|| {
		// Arrange
		let evm_address = EVMAccounts::evm_address(&ALICE);
		assert_ok!(EVMAccounts::add_contract_deployer(RuntimeOrigin::root(), evm_address));
		assert!(EVMAccounts::can_deploy_contracts(evm_address));

		// Act
		assert_ok!(EVMAccounts::remove_contract_deployer(RuntimeOrigin::root(), evm_address));

		// Assert
		assert!(!EVMAccounts::can_deploy_contracts(evm_address));

		// removing the address again should be ok
		assert_ok!(EVMAccounts::remove_contract_deployer(RuntimeOrigin::root(), evm_address));
	});
}

#[test]
fn renounce_contract_deployer_should_remove_address_from_the_storage() {
	ExtBuilder::default().build().execute_with(|| {
		// Arrange
		let evm_address = EVMAccounts::evm_address(&ALICE);
		assert_ok!(EVMAccounts::add_contract_deployer(RuntimeOrigin::root(), evm_address));
		assert!(EVMAccounts::can_deploy_contracts(evm_address));

		// Act
		assert_ok!(EVMAccounts::renounce_contract_deployer(RuntimeOrigin::signed(ALICE)));

		// Assert
		assert!(!EVMAccounts::can_deploy_contracts(evm_address));

		// ronouncing the address again should be ok
		assert_ok!(EVMAccounts::renounce_contract_deployer(RuntimeOrigin::signed(ALICE)));
	});
}
