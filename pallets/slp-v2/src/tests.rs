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

use crate::{
	astar_dapp_staking::types::{
		AstarDappStakingLedger, AstarDappStakingPendingStatus, AstarUnlockingRecord,
		AstarValidator, DappStaking,
	},
	common::types::{
		Delegator, Ledger, PendingStatus, ProtocolConfiguration, StakingProtocol, Validator,
		XcmFee, XcmTask,
	},
	mock::*,
	DelegatorByStakingProtocolAndDelegatorIndex, DelegatorIndexByStakingProtocolAndDelegator,
	Error as SlpV2Error, Event as SlpV2Event, LastUpdateOngoingTimeUnitBlockNumber,
	LedgerByStakingProtocolAndDelegator, NextDelegatorIndexByStakingProtocol,
	ValidatorsByStakingProtocolAndDelegator,
};
use bifrost_primitives::{CommissionPalletId, TimeUnit, VtokenMintingOperator, VASTR};
use cumulus_primitives_core::Weight;
use frame_support::{assert_noop, assert_ok, traits::fungibles::Mutate};
use orml_traits::MultiCurrency;
use pallet_xcm::Origin as XcmOrigin;
use polkadot_parachain_primitives::primitives::Sibling;
use sp_core::{bytes::to_hex, crypto::Ss58Codec, H160};
use sp_runtime::{
	helpers_128bit::multiply_by_rational_with_rounding, traits::AccountIdConversion, BoundedVec,
	Permill, Rounding,
};
use xcm::{
	latest::{MaybeErrorCode, Parent, Response},
	prelude::{AccountId32, Parachain},
	v4::Location,
};

pub const STAKING_PROTOCOL: StakingProtocol = StakingProtocol::AstarDappStaking;

pub const CONFIGURATION: ProtocolConfiguration<AccountId> = ProtocolConfiguration {
	xcm_task_fee: XcmFee { weight: Weight::zero(), fee: 100 },
	protocol_fee_rate: Permill::from_perthousand(100),
	unlock_period: TimeUnit::Era(9),
	operator: AccountId::new([0u8; 32]),
	max_update_token_exchange_rate: Permill::from_perthousand(1),
	update_time_unit_interval: 100u32,
	update_exchange_rate_interval: 100u32,
};

fn set_protocol_configuration() {
	assert_ok!(SlpV2::set_protocol_configuration(
		RuntimeOrigin::root(),
		STAKING_PROTOCOL,
		CONFIGURATION
	));
}

#[test]
fn derivative_account_id_should_work() {
	new_test_ext().execute_with(|| {
		let sbling2030: AccountId = Sibling::from(2030).into_account_truncating();
		let sub_0_sbling2030 = SlpV2::derivative_account_id(sbling2030.clone(), 0).unwrap();
		let sub_1_sbling2030 = SlpV2::derivative_account_id(sbling2030.clone(), 1).unwrap();
		let sub_2_sbling2030 = SlpV2::derivative_account_id(sbling2030.clone(), 2).unwrap();

		assert_eq!(
			sub_0_sbling2030,
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap()
		);
		assert_eq!(
			sub_1_sbling2030,
			AccountId::from_ss58check("XF713iFjaLwTxvVQv3YJdKhFY4EYpcVh6GzAWR7Lj5aoNHZ").unwrap()
		);
		assert_eq!(
			sub_2_sbling2030,
			AccountId::from_ss58check("YeKP2BdVpFrXbbqkoVhDFZP9u3nUuop7fpMppQczQXBLhD1").unwrap()
		)
	})
}

#[test]
fn set_configuration_should_work() {
	new_test_ext().execute_with(|| {
		set_protocol_configuration();
		expect_event(SlpV2Event::SetConfiguration {
			staking_protocol: STAKING_PROTOCOL,
			configuration: CONFIGURATION,
		});
	})
}

#[test]
fn add_delegator_should_work() {
	new_test_ext().execute_with(|| {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);
		let delegator_index = 0;

		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));
		expect_event(SlpV2Event::AddDelegator {
			staking_protocol: STAKING_PROTOCOL,
			delegator_index,
			delegator: delegator.clone(),
		});
		assert_eq!(
			DelegatorByStakingProtocolAndDelegatorIndex::<Test>::get(
				STAKING_PROTOCOL,
				delegator_index
			),
			Some(delegator.clone())
		);
		assert_eq!(
			DelegatorIndexByStakingProtocolAndDelegator::<Test>::get(
				STAKING_PROTOCOL,
				delegator.clone()
			),
			Some(delegator_index)
		);
		assert_eq!(NextDelegatorIndexByStakingProtocol::<Test>::get(STAKING_PROTOCOL), 1);
		assert_eq!(
			LedgerByStakingProtocolAndDelegator::<Test>::get(STAKING_PROTOCOL, delegator),
			Some(Ledger::AstarDappStaking(AstarDappStakingLedger {
				locked: 0,
				unlocking: Default::default()
			}))
		);
	});
}

#[test]
fn repeat_add_delegator_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(SlpV2::add_delegator(
			RuntimeOrigin::root(),
			StakingProtocol::AstarDappStaking,
			None
		));

		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("XF713iFjaLwTxvVQv3YJdKhFY4EYpcVh6GzAWR7Lj5aoNHZ").unwrap(),
		);
		let delegator_index = 1;

		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));
		expect_event(SlpV2Event::AddDelegator {
			staking_protocol: STAKING_PROTOCOL,
			delegator_index,
			delegator: delegator.clone(),
		});
		assert_eq!(
			DelegatorByStakingProtocolAndDelegatorIndex::<Test>::get(
				STAKING_PROTOCOL,
				delegator_index
			),
			Some(delegator.clone())
		);
		assert_eq!(
			DelegatorIndexByStakingProtocolAndDelegator::<Test>::get(
				STAKING_PROTOCOL,
				delegator.clone()
			),
			Some(delegator_index)
		);
		assert_eq!(NextDelegatorIndexByStakingProtocol::<Test>::get(STAKING_PROTOCOL), 2);
		assert_eq!(
			LedgerByStakingProtocolAndDelegator::<Test>::get(STAKING_PROTOCOL, delegator),
			Some(Ledger::AstarDappStaking(AstarDappStakingLedger {
				locked: 0,
				unlocking: Default::default()
			}))
		);
	});
}

#[test]
fn add_delegator_delegator_index_over_flow() {
	new_test_ext().execute_with(|| {
		NextDelegatorIndexByStakingProtocol::<Test>::insert(STAKING_PROTOCOL, 65535);
		assert_noop!(
			SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None),
			SlpV2Error::<Test>::DelegatorIndexOverflow
		);
	});
}

#[test]
fn add_delegator_delegator_already_exists() {
	new_test_ext().execute_with(|| {
		let delegator_0 = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);

		DelegatorByStakingProtocolAndDelegatorIndex::<Test>::insert(
			STAKING_PROTOCOL,
			0,
			delegator_0,
		);
		assert_noop!(
			SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None),
			SlpV2Error::<Test>::DelegatorAlreadyExists
		);
	});
}

#[test]
fn add_delegator_delegator_index_already_exists() {
	new_test_ext().execute_with(|| {
		let delegator_0 = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);

		DelegatorIndexByStakingProtocolAndDelegator::<Test>::insert(
			STAKING_PROTOCOL,
			delegator_0,
			0,
		);
		assert_noop!(
			SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None),
			SlpV2Error::<Test>::DelegatorIndexAlreadyExists
		);
	});
}

#[test]
fn remove_delegator_should_work() {
	new_test_ext().execute_with(|| {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);
		let delegator_index = 0;
		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));
		assert_ok!(SlpV2::remove_delegator(
			RuntimeOrigin::root(),
			STAKING_PROTOCOL,
			delegator.clone()
		));
		expect_event(SlpV2Event::RemoveDelegator {
			staking_protocol: STAKING_PROTOCOL,
			delegator_index,
			delegator: delegator.clone(),
		});
		assert_eq!(
			DelegatorByStakingProtocolAndDelegatorIndex::<Test>::get(
				STAKING_PROTOCOL,
				delegator_index
			),
			None
		);
		assert_eq!(
			DelegatorIndexByStakingProtocolAndDelegator::<Test>::get(
				STAKING_PROTOCOL,
				delegator.clone()
			),
			None
		);
		assert_eq!(NextDelegatorIndexByStakingProtocol::<Test>::get(STAKING_PROTOCOL), 1);
		assert_eq!(
			ValidatorsByStakingProtocolAndDelegator::<Test>::get(STAKING_PROTOCOL, delegator)
				.to_vec(),
			vec![]
		);
	});
}

#[test]
fn remove_delegator_delegator_index_not_found() {
	new_test_ext().execute_with(|| {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);
		assert_noop!(
			SlpV2::remove_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, delegator.clone()),
			SlpV2Error::<Test>::DelegatorIndexNotFound
		);
	});
}

#[test]
fn add_validator_should_work() {
	new_test_ext().execute_with(|| {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);
		let validator = Validator::AstarDappStaking(AstarValidator::Evm(H160::default()));

		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));

		assert_ok!(SlpV2::add_validator(
			RuntimeOrigin::root(),
			STAKING_PROTOCOL,
			delegator.clone(),
			validator.clone()
		));
		expect_event(SlpV2Event::AddValidator {
			staking_protocol: STAKING_PROTOCOL,
			delegator: delegator.clone(),
			validator: validator.clone(),
		});
		assert_eq!(
			ValidatorsByStakingProtocolAndDelegator::<Test>::get(STAKING_PROTOCOL, delegator)
				.to_vec(),
			vec![validator]
		);
	});
}

#[test]
fn repeat_add_validator_should_work() {
	new_test_ext().execute_with(|| {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);
		let validator1 = Validator::AstarDappStaking(AstarValidator::Evm(H160::default()));
		let validator2 = Validator::AstarDappStaking(AstarValidator::Wasm(
			AccountId::from_ss58check("YeKP2BdVpFrXbbqkoVhDFZP9u3nUuop7fpMppQczQXBLhD1").unwrap(),
		));

		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));

		assert_ok!(SlpV2::add_validator(
			RuntimeOrigin::root(),
			STAKING_PROTOCOL,
			delegator.clone(),
			validator1.clone()
		));
		assert_ok!(SlpV2::add_validator(
			RuntimeOrigin::root(),
			STAKING_PROTOCOL,
			delegator.clone(),
			validator2.clone()
		));

		expect_event(SlpV2Event::AddValidator {
			staking_protocol: STAKING_PROTOCOL,
			delegator: delegator.clone(),
			validator: validator2.clone(),
		});
		assert_eq!(
			ValidatorsByStakingProtocolAndDelegator::<Test>::get(
				STAKING_PROTOCOL,
				delegator.clone()
			)
			.to_vec(),
			vec![validator1, validator2]
		);
	});
}

#[test]
fn remove_validator_should_work() {
	new_test_ext().execute_with(|| {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);
		let validator = Validator::AstarDappStaking(AstarValidator::Evm(H160::default()));

		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));

		assert_ok!(SlpV2::add_validator(
			RuntimeOrigin::root(),
			STAKING_PROTOCOL,
			delegator.clone(),
			validator.clone()
		));
		assert_ok!(SlpV2::remove_validator(
			RuntimeOrigin::root(),
			STAKING_PROTOCOL,
			delegator.clone(),
			validator.clone()
		));
		expect_event(SlpV2Event::RemoveValidator {
			staking_protocol: STAKING_PROTOCOL,
			delegator: delegator.clone(),
			validator: validator.clone(),
		});
		assert_eq!(
			ValidatorsByStakingProtocolAndDelegator::<Test>::get(
				STAKING_PROTOCOL,
				delegator.clone()
			)
			.to_vec(),
			vec![]
		);
	});
}

#[test]
fn astar_dapp_staking_lock() {
	new_test_ext().execute_with(|| {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);
		let task = DappStaking::Lock(100);
		let pending_status = PendingStatus::AstarDappStaking(AstarDappStakingPendingStatus::Lock(
			delegator.clone(),
			100,
		));
		let dest_location = STAKING_PROTOCOL.info().remote_dest_location;

		set_protocol_configuration();
		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));

		assert_ok!(SlpV2::astar_dapp_staking(
			RuntimeOrigin::root(),
			delegator.clone(),
			task.clone()
		));
		expect_event(SlpV2Event::SendXcmTask {
			query_id: Some(0),
			delegator: delegator.clone(),
			task: XcmTask::AstarDappStaking(task),
			pending_status: Some(pending_status),
			dest_location,
		});
		assert_ok!(SlpV2::notify_astar_dapp_staking(
			XcmOrigin::Response(Parent.into()).into(),
			0,
			Response::DispatchResult(MaybeErrorCode::Success)
		));

		let ledger =
			LedgerByStakingProtocolAndDelegator::<Test>::get(STAKING_PROTOCOL, delegator).unwrap();
		assert_eq!(
			ledger,
			Ledger::AstarDappStaking(AstarDappStakingLedger {
				locked: 100,
				unlocking: Default::default()
			})
		)
	})
}

#[test]
fn repeat_astar_dapp_staking_lock() {
	new_test_ext().execute_with(|| {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);
		let task1 = DappStaking::Lock(100);
		let task2 = DappStaking::Lock(200);
		let query_id_0 = 0;
		let query_id_1 = 1;
		let pending_status = PendingStatus::AstarDappStaking(AstarDappStakingPendingStatus::Lock(
			delegator.clone(),
			200,
		));
		let dest_location = STAKING_PROTOCOL.info().remote_dest_location;
		set_protocol_configuration();

		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));

		assert_ok!(SlpV2::astar_dapp_staking(RuntimeOrigin::root(), delegator.clone(), task1));
		assert_ok!(SlpV2::notify_astar_dapp_staking(
			XcmOrigin::Response(Parent.into()).into(),
			query_id_0,
			Response::DispatchResult(MaybeErrorCode::Success)
		));

		assert_ok!(SlpV2::astar_dapp_staking(
			RuntimeOrigin::root(),
			delegator.clone(),
			task2.clone()
		));
		expect_event(SlpV2Event::SendXcmTask {
			query_id: Some(query_id_1),
			delegator: delegator.clone(),
			task: XcmTask::AstarDappStaking(task2),
			pending_status: Some(pending_status),
			dest_location,
		});
		assert_ok!(SlpV2::notify_astar_dapp_staking(
			XcmOrigin::Response(Parent.into()).into(),
			query_id_1,
			Response::DispatchResult(MaybeErrorCode::Success)
		));

		let ledger =
			LedgerByStakingProtocolAndDelegator::<Test>::get(STAKING_PROTOCOL, delegator).unwrap();
		assert_eq!(
			ledger,
			Ledger::AstarDappStaking(AstarDappStakingLedger {
				locked: 300,
				unlocking: Default::default()
			})
		)
	})
}

#[test]
fn astar_dapp_staking_unlock() {
	new_test_ext().execute_with(|| {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);
		let task = DappStaking::Lock(100);

		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));
		set_protocol_configuration();

		assert_ok!(SlpV2::astar_dapp_staking(RuntimeOrigin::root(), delegator.clone(), task));
		assert_ok!(SlpV2::notify_astar_dapp_staking(
			XcmOrigin::Response(Parent.into()).into(),
			0,
			Response::DispatchResult(MaybeErrorCode::Success)
		));

		let task = DappStaking::Unlock(50);
		RelaychainBlockNumber::set(100);
		assert_ok!(SlpV2::update_ongoing_time_unit(
			RuntimeOrigin::root(),
			STAKING_PROTOCOL,
			Some(TimeUnit::Era(1))
		));
		assert_ok!(SlpV2::astar_dapp_staking(RuntimeOrigin::root(), delegator.clone(), task));
		assert_ok!(SlpV2::notify_astar_dapp_staking(
			XcmOrigin::Response(Parent.into()).into(),
			1,
			Response::DispatchResult(MaybeErrorCode::Success)
		));

		let ledger =
			LedgerByStakingProtocolAndDelegator::<Test>::get(STAKING_PROTOCOL, delegator).unwrap();
		assert_eq!(
			ledger,
			Ledger::AstarDappStaking(AstarDappStakingLedger {
				locked: 50,
				unlocking: BoundedVec::try_from(vec![AstarUnlockingRecord {
					amount: 50,
					unlock_time: TimeUnit::Era(10)
				}])
				.unwrap()
			})
		)
	})
}

#[test]
fn astar_dapp_staking_stake() {
	new_test_ext().execute_with(|| {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);
		let validator = Validator::AstarDappStaking(AstarValidator::Evm(H160::default()));
		let task = DappStaking::Stake(AstarValidator::Evm(H160::default()), 100);
		let query_id = None;
		let pending_status = None;
		let dest_location = STAKING_PROTOCOL.info().remote_dest_location;

		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));
		assert_ok!(SlpV2::add_validator(
			RuntimeOrigin::root(),
			STAKING_PROTOCOL,
			delegator.clone(),
			validator.clone()
		));
		set_protocol_configuration();

		assert_ok!(SlpV2::astar_dapp_staking(
			RuntimeOrigin::root(),
			delegator.clone(),
			task.clone()
		));
		expect_event(SlpV2Event::SendXcmTask {
			query_id,
			delegator,
			task: XcmTask::AstarDappStaking(task.clone()),
			pending_status,
			dest_location,
		})
	})
}

#[test]
fn astar_dapp_staking_unstake() {
	new_test_ext().execute_with(|| {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);
		let validator = Validator::AstarDappStaking(AstarValidator::Evm(H160::default()));
		let task = DappStaking::Unstake(AstarValidator::Evm(H160::default()), 100);
		let query_id = None;
		let pending_status = None;
		let dest_location = STAKING_PROTOCOL.info().remote_dest_location;

		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));
		assert_ok!(SlpV2::add_validator(
			RuntimeOrigin::root(),
			STAKING_PROTOCOL,
			delegator.clone(),
			validator.clone()
		));
		set_protocol_configuration();

		assert_ok!(SlpV2::astar_dapp_staking(
			RuntimeOrigin::root(),
			delegator.clone(),
			task.clone()
		));
		expect_event(SlpV2Event::SendXcmTask {
			query_id,
			delegator,
			task: XcmTask::AstarDappStaking(task.clone()),
			pending_status,
			dest_location,
		})
	})
}

#[test]
fn staking_protocol_get_dest_beneficiary_location() {
	new_test_ext().execute_with(|| {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		let account_id =
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap();
		let delegator = Delegator::Substrate(account_id.clone());
		assert_eq!(
			staking_protocol.get_dest_beneficiary_location::<Test>(delegator),
			Some(Location::new(
				1,
				[Parachain(2006), AccountId32 { network: None, id: account_id.into() }]
			))
		);
	})
}

#[test]
fn astar_polkadot_xcm_call() {
	new_test_ext().execute_with(|| {
		let (to, _) = VtokenMinting::get_entrance_and_exit_accounts();
		let calldata = SlpV2::wrap_polkadot_xcm_limited_reserve_transfer_assets_call_data(
			&StakingProtocol::AstarDappStaking,
			100,
			to.clone()
		)
		.unwrap();

		assert_eq!(to_hex(&calldata, false), "0x330804010100b91f04000101006d6f646c62662f76746b696e0000000000000000000000000000000000000000040400000091010000000000");

		let call_data = SlpV2::wrap_polkadot_xcm_limited_reserve_transfer_assets_call_data(
			&StakingProtocol::PolkadotStaking,
			100,
			to
		)
		.unwrap();
		assert_eq!(to_hex(&call_data, false), "0x630804000100b91f04000101006d6f646c62662f76746b696e0000000000000000000000000000000000000000040400000091010000000000");
	})
}

#[test]
fn set_ledger_should_work() {
	new_test_ext().execute_with(|| {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);
		let ledger = Ledger::AstarDappStaking(AstarDappStakingLedger {
			locked: 100,
			unlocking: Default::default(),
		});
		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), staking_protocol, None));
		assert_ok!(SlpV2::set_ledger(
			RuntimeOrigin::root(),
			staking_protocol,
			delegator.clone(),
			ledger.clone()
		));

		expect_event(SlpV2Event::SetLedger {
			staking_protocol,
			delegator: delegator.clone(),
			ledger: ledger.clone(),
		});
		assert_eq!(
			LedgerByStakingProtocolAndDelegator::<Test>::get(staking_protocol, delegator.clone()),
			Some(ledger)
		);
	})
}

#[test]
fn set_ledger_error() {
	new_test_ext().execute_with(|| {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);
		let ledger = Ledger::AstarDappStaking(AstarDappStakingLedger {
			locked: 100,
			unlocking: Default::default(),
		});
		assert_noop!(
			SlpV2::set_ledger(
				RuntimeOrigin::root(),
				staking_protocol,
				delegator.clone(),
				ledger.clone()
			),
			SlpV2Error::<Test>::DelegatorIndexNotFound
		);

		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), staking_protocol, None));
		assert_ok!(SlpV2::set_ledger(
			RuntimeOrigin::root(),
			staking_protocol,
			delegator.clone(),
			ledger.clone()
		));

		assert_noop!(
			SlpV2::set_ledger(
				RuntimeOrigin::root(),
				staking_protocol,
				delegator.clone(),
				ledger.clone()
			),
			SlpV2Error::<Test>::InvalidParameter
		);
	})
}

#[test]
fn update_ongoing_time_unit_should_work() {
	new_test_ext().execute_with(|| {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		let currency_id = staking_protocol.info().currency_id;
		set_protocol_configuration();
		RelaychainDataProvider::set_block_number(100);
		assert_ok!(SlpV2::update_ongoing_time_unit(
			RuntimeOrigin::root(),
			staking_protocol,
			Some(TimeUnit::Era(1))
		));
		expect_event(SlpV2Event::TimeUnitUpdated { staking_protocol, time_unit: TimeUnit::Era(1) });
		assert_eq!(VtokenMinting::get_ongoing_time_unit(currency_id), Some(TimeUnit::Era(1)));
		assert_eq!(LastUpdateOngoingTimeUnitBlockNumber::<Test>::get(staking_protocol), 100);

		RelaychainDataProvider::set_block_number(200);

		assert_ok!(SlpV2::update_ongoing_time_unit(RuntimeOrigin::root(), staking_protocol, None));
		expect_event(SlpV2Event::TimeUnitUpdated { staking_protocol, time_unit: TimeUnit::Era(2) });
		assert_eq!(VtokenMinting::get_ongoing_time_unit(currency_id), Some(TimeUnit::Era(2)));
		assert_eq!(LastUpdateOngoingTimeUnitBlockNumber::<Test>::get(staking_protocol), 200);
	});
}

#[test]
fn update_ongoing_time_unit_update_interval_too_short() {
	new_test_ext().execute_with(|| {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		set_protocol_configuration();

		// current relaychain block number 1 < update_interval 100 + last update block number 0 =>
		// Error
		assert_noop!(
			SlpV2::update_ongoing_time_unit(
				RuntimeOrigin::root(),
				staking_protocol,
				Some(TimeUnit::Era(1))
			),
			SlpV2Error::<Test>::UpdateIntervalTooShort
		);

		RelaychainDataProvider::set_block_number(100);
		// current relaychain block number 100 = update_interval 100 + last update block number 0 =>
		// Ok
		assert_noop!(
			SlpV2::update_ongoing_time_unit(RuntimeOrigin::root(), staking_protocol, None),
			SlpV2Error::<Test>::TimeUnitNotFound
		);

		assert_ok!(SlpV2::update_ongoing_time_unit(
			RuntimeOrigin::root(),
			staking_protocol,
			Some(TimeUnit::Era(1))
		));

		RelaychainDataProvider::set_block_number(199);
		// current relaychain block number 199 < update_interval 100 + last update block number 100
		// => Error
		assert_noop!(
			SlpV2::update_ongoing_time_unit(RuntimeOrigin::root(), staking_protocol, None),
			SlpV2Error::<Test>::UpdateIntervalTooShort
		);
		RelaychainDataProvider::set_block_number(200);
		// current relaychain block number 200 = update_interval 100 + last update block number 100
		// => Ok
		assert_ok!(SlpV2::update_ongoing_time_unit(RuntimeOrigin::root(), staking_protocol, None));
	});
}

#[test]
fn update_token_exchange_rate_should_work() {
	new_test_ext().execute_with(|| {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		let currency_id = staking_protocol.info().currency_id;
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);
		let amount = 10_059_807_133_828_175_000_000u128;
		let token_pool = 24_597_119_664_064_597_684_680_531u128;
		let vtoken_total_issuance = 21_728_134_208_272_171_009_169_962u128;

		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));
		Currencies::set_balance(VASTR, &AccountId::from([0u8; 32]), vtoken_total_issuance);
		assert_eq!(Currencies::total_issuance(VASTR), vtoken_total_issuance);
		assert_ok!(VtokenMinting::increase_token_pool(currency_id, token_pool));

		set_protocol_configuration();
		assert_eq!(VtokenMinting::get_token_pool(currency_id), token_pool);

		RelaychainDataProvider::set_block_number(100);

		// Set protocol fee rate is 10%
		let protocol_fee_rate = Permill::from_perthousand(100);
		assert_ok!(SlpV2::update_token_exchange_rate(
			RuntimeOrigin::root(),
			staking_protocol,
			delegator.clone(),
			amount
		));
		// The protocol_fee is 888.644046532367789159 VASTR.
		let protocol_fee = multiply_by_rational_with_rounding(
			protocol_fee_rate * amount,
			vtoken_total_issuance,
			token_pool,
			Rounding::Down,
		)
		.unwrap();
		expect_event(SlpV2Event::TokenExchangeRateUpdated {
			staking_protocol,
			delegator: delegator.clone(),
			protocol_fee_currency_id: VASTR,
			protocol_fee,
			amount,
		});
		let vtoken_total_issuance = vtoken_total_issuance + protocol_fee;
		let token_pool = token_pool + amount;
		assert_eq!(VtokenMinting::get_token_pool(currency_id), token_pool);
		assert_eq!(Currencies::total_issuance(VASTR), vtoken_total_issuance);
		assert_eq!(
			Currencies::free_balance(VASTR, &CommissionPalletId::get().into_account_truncating()),
			protocol_fee
		);

		RelaychainDataProvider::set_block_number(200);
		// current relaychain block number 300 = update_interval 100 + last update block number 200
		// => Ok
		assert_ok!(SlpV2::update_token_exchange_rate(
			RuntimeOrigin::root(),
			staking_protocol,
			delegator.clone(),
			amount
		));

		// The protocol_fee is 888.317083868634496826 VASTR.
		let protocol_fee_1 = multiply_by_rational_with_rounding(
			protocol_fee_rate * amount,
			vtoken_total_issuance,
			token_pool,
			Rounding::Down,
		)
		.unwrap();
		expect_event(SlpV2Event::TokenExchangeRateUpdated {
			staking_protocol,
			delegator: delegator.clone(),
			protocol_fee_currency_id: VASTR,
			protocol_fee: protocol_fee_1,
			amount,
		});
		let vtoken_total_issuance = vtoken_total_issuance + protocol_fee_1;
		let token_pool = token_pool + amount;
		assert_eq!(VtokenMinting::get_token_pool(currency_id), token_pool);
		assert_eq!(Currencies::total_issuance(VASTR), vtoken_total_issuance);
		assert_eq!(
			Currencies::free_balance(VASTR, &CommissionPalletId::get().into_account_truncating()),
			protocol_fee + protocol_fee_1
		);
	})
}

#[test]
fn update_token_exchange_rate_limt_error() {
	new_test_ext().execute_with(|| {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		let currency_id = staking_protocol.info().currency_id;
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);
		let amount = 1000u128;
		let token_pool = 12000u128;
		let vtoken_total_issuance = 10000u128;

		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));
		Currencies::set_balance(VASTR, &AccountId::from([0u8; 32]), vtoken_total_issuance);
		assert_ok!(VtokenMinting::increase_token_pool(currency_id, token_pool));

		set_protocol_configuration();

		// current relaychain block number 1 < update_interval 100 + last update block number 0 =>
		// Error
		assert_noop!(
			SlpV2::update_token_exchange_rate(
				RuntimeOrigin::root(),
				staking_protocol,
				delegator.clone(),
				amount
			),
			SlpV2Error::<Test>::UpdateIntervalTooShort
		);

		RelaychainDataProvider::set_block_number(101);
		// current relaychain block number 101 < update_interval 100 + last update block number 0 =>
		// Ok amount 13 < max_update_amount 12 => Error
		assert_noop!(
			SlpV2::update_token_exchange_rate(
				RuntimeOrigin::root(),
				staking_protocol,
				delegator.clone(),
				amount
			),
			SlpV2Error::<Test>::UpdateTokenExchangeRateAmountTooLarge
		);
	})
}
