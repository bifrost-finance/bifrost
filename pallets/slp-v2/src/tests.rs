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
		AstarDappStakingLedger, AstarUnlockingRecord, AstarValidator, DappStaking,
	},
	common::types::{
		Delegator, Ledger, PendingStatus, StakingProtocol, Validator, XcmFee, XcmTask,
		XcmTaskWithParams,
	},
	mock::*,
	DelegatorByStakingProtocolAndDelegatorIndex, DelegatorIndexByStakingProtocolAndDelegator,
	Error as SlpV2Error, Event as SlpV2Event, LastUpdateOngoingTimeUnitBlockNumber,
	LedgerByStakingProtocolAndDelegator, NextDelegatorIndexByStakingProtocol,
	OperatorByStakingProtocol, ProtocolFeeRateByStakingProtocol,
	UpdateOngoingTimeUintIntervalByStakingProtocol, UpdateTokenExchangeRateLimitByStakingProtocol,
	ValidatorsByStakingProtocol, XcmFeeByXcmTask,
};
use bifrost_primitives::{TimeUnit, VtokenMintingOperator, VASTR};
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
			DelegatorIndexByStakingProtocolAndDelegator::<Test>::get(STAKING_PROTOCOL, delegator),
			None
		);
		assert_eq!(NextDelegatorIndexByStakingProtocol::<Test>::get(STAKING_PROTOCOL), 1);
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
fn remove_delegator_delegator_not_found() {
	new_test_ext().execute_with(|| {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);
		DelegatorIndexByStakingProtocolAndDelegator::<Test>::insert(
			STAKING_PROTOCOL,
			delegator.clone(),
			0,
		);
		assert_noop!(
			SlpV2::remove_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, delegator.clone()),
			SlpV2Error::<Test>::DelegatorNotFound
		);
	});
}

#[test]
fn add_validator_should_work() {
	new_test_ext().execute_with(|| {
		let validator = Validator::DappStaking(AstarValidator::Evm(H160::default()));

		assert_ok!(SlpV2::add_validator(
			RuntimeOrigin::root(),
			STAKING_PROTOCOL,
			validator.clone()
		));
		expect_event(SlpV2Event::AddValidator {
			staking_protocol: STAKING_PROTOCOL,
			validator: validator.clone(),
		});
		assert_eq!(
			ValidatorsByStakingProtocol::<Test>::get(STAKING_PROTOCOL).to_vec(),
			vec![validator]
		);
	});
}

#[test]
fn repeat_add_validator_should_work() {
	new_test_ext().execute_with(|| {
		let validator1 = Validator::DappStaking(AstarValidator::Evm(H160::default()));
		let validator2 = Validator::DappStaking(AstarValidator::Wasm(
			AccountId::from_ss58check("YeKP2BdVpFrXbbqkoVhDFZP9u3nUuop7fpMppQczQXBLhD1").unwrap(),
		));

		assert_ok!(SlpV2::add_validator(
			RuntimeOrigin::root(),
			STAKING_PROTOCOL,
			validator1.clone()
		));
		assert_ok!(SlpV2::add_validator(
			RuntimeOrigin::root(),
			STAKING_PROTOCOL,
			validator2.clone()
		));

		expect_event(SlpV2Event::AddValidator {
			staking_protocol: STAKING_PROTOCOL,
			validator: validator2.clone(),
		});
		assert_eq!(
			ValidatorsByStakingProtocol::<Test>::get(STAKING_PROTOCOL).to_vec(),
			vec![validator1, validator2]
		);
	});
}

#[test]
fn remove_validator_should_work() {
	new_test_ext().execute_with(|| {
		let validator = Validator::DappStaking(AstarValidator::Evm(H160::default()));

		assert_ok!(SlpV2::add_validator(
			RuntimeOrigin::root(),
			STAKING_PROTOCOL,
			validator.clone()
		));
		assert_ok!(SlpV2::remove_validator(
			RuntimeOrigin::root(),
			STAKING_PROTOCOL,
			validator.clone()
		));
		expect_event(SlpV2Event::RemoveValidator {
			staking_protocol: STAKING_PROTOCOL,
			validator: validator.clone(),
		});
		assert_eq!(ValidatorsByStakingProtocol::<Test>::get(STAKING_PROTOCOL).to_vec(), vec![]);
	});
}

#[test]
fn astar_dapp_staking_lock() {
	new_test_ext().execute_with(|| {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);
		let task = DappStaking::Lock(100);
		let xcm_fee = XcmFee { weight: Default::default(), fee: 100 };
		let pending_status = PendingStatus::AstarDappStakingLock(delegator.clone(), 100);
		let dest_location = STAKING_PROTOCOL.get_dest_location();

		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));
		assert_ok!(SlpV2::set_xcm_task_fee(
			RuntimeOrigin::root(),
			XcmTask::AstarDappStakingLock,
			xcm_fee
		));

		assert_ok!(SlpV2::astar_dapp_staking(
			RuntimeOrigin::root(),
			delegator.clone(),
			task.clone()
		));
		expect_event(SlpV2Event::SendXcmTask {
			query_id: Some(0),
			delegator: delegator.clone(),
			xcm_task_with_params: XcmTaskWithParams::AstarDappStaking(task),
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
		let xcm_fee = XcmFee { weight: Default::default(), fee: 100 };
		let query_id_0 = 0;
		let query_id_1 = 1;
		let pending_status = PendingStatus::AstarDappStakingLock(delegator.clone(), 200);
		let dest_location = STAKING_PROTOCOL.get_dest_location();

		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));
		assert_ok!(SlpV2::set_xcm_task_fee(
			RuntimeOrigin::root(),
			XcmTask::AstarDappStakingLock,
			xcm_fee
		));

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
			xcm_task_with_params: XcmTaskWithParams::AstarDappStaking(task2),
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
		let xcm_fee = XcmFee { weight: Default::default(), fee: 100 };

		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));
		assert_ok!(SlpV2::set_xcm_task_fee(
			RuntimeOrigin::root(),
			XcmTask::AstarDappStakingLock,
			xcm_fee
		));
		assert_ok!(SlpV2::set_xcm_task_fee(
			RuntimeOrigin::root(),
			XcmTask::AstarDappStakingUnLock,
			xcm_fee
		));

		assert_ok!(SlpV2::astar_dapp_staking(RuntimeOrigin::root(), delegator.clone(), task));
		assert_ok!(SlpV2::notify_astar_dapp_staking(
			XcmOrigin::Response(Parent.into()).into(),
			0,
			Response::DispatchResult(MaybeErrorCode::Success)
		));

		let task = DappStaking::Unlock(50);
		assert_ok!(SlpV2::update_ongoing_time_unit(RuntimeOrigin::root(), STAKING_PROTOCOL));
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
		let task = DappStaking::Stake(AstarValidator::Evm(H160::default()), 100);
		let xcm_task_with_params = XcmTaskWithParams::AstarDappStaking(task.clone());
		let xcm_fee = XcmFee { weight: Default::default(), fee: 100 };
		let query_id = None;
		let pending_status = None;
		let dest_location = STAKING_PROTOCOL.get_dest_location();

		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));
		assert_ok!(SlpV2::set_xcm_task_fee(
			RuntimeOrigin::root(),
			XcmTask::AstarDappStakingStake,
			xcm_fee
		));

		assert_ok!(SlpV2::astar_dapp_staking(RuntimeOrigin::root(), delegator.clone(), task));
		expect_event(SlpV2Event::SendXcmTask {
			query_id,
			delegator,
			xcm_task_with_params,
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
		let task = DappStaking::Unstake(AstarValidator::Evm(H160::default()), 100);
		let xcm_task_with_params = XcmTaskWithParams::AstarDappStaking(task.clone());
		let xcm_fee = XcmFee { weight: Default::default(), fee: 100 };
		let query_id = None;
		let pending_status = None;
		let dest_location = STAKING_PROTOCOL.get_dest_location();

		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));
		assert_ok!(SlpV2::set_xcm_task_fee(
			RuntimeOrigin::root(),
			XcmTask::AstarDappStakingUnstake,
			xcm_fee
		));

		assert_ok!(SlpV2::astar_dapp_staking(RuntimeOrigin::root(), delegator.clone(), task));
		expect_event(SlpV2Event::SendXcmTask {
			query_id,
			delegator,
			xcm_task_with_params,
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
		let calldata = SlpV2::wrap_polkadot_xcm_limited_reserve_transfer_assets_call_data(
			&StakingProtocol::AstarDappStaking,
			100,
		)
		.unwrap();

		assert_eq!(to_hex(&calldata, false), "0x330804010100b91f04000101006d6f646c62662f76746b696e0000000000000000000000000000000000000000040400000091010000000000");

		let call_data = SlpV2::wrap_polkadot_xcm_limited_reserve_transfer_assets_call_data(
			&StakingProtocol::PolkadotStaking,
			100,
		)
		.unwrap();
		assert_eq!(to_hex(&call_data, false), "0x630804000100b91f04000101006d6f646c62662f76746b696e0000000000000000000000000000000000000000040400000091010000000000");
	})
}

#[test]
fn set_protocol_fee_rate_should_work() {
	new_test_ext().execute_with(|| {
		let fee_rate = Permill::from_perthousand(1);
		let staking_protocol = StakingProtocol::AstarDappStaking;
		assert_ok!(SlpV2::set_protocol_fee_rate(RuntimeOrigin::root(), staking_protocol, fee_rate));
		expect_event(SlpV2Event::SetProtocolFeeRate { staking_protocol, fee_rate });
		assert_eq!(ProtocolFeeRateByStakingProtocol::<Test>::get(staking_protocol), fee_rate);
	})
}

#[test]
fn set_protocol_fee_rate_invalid_parameter() {
	new_test_ext().execute_with(|| {
		let fee_rate = Permill::from_perthousand(0);
		let staking_protocol = StakingProtocol::AstarDappStaking;
		assert_noop!(
			SlpV2::set_protocol_fee_rate(RuntimeOrigin::root(), staking_protocol, fee_rate),
			SlpV2Error::<Test>::InvalidParameter
		);

		let fee_rate = Permill::from_perthousand(1);
		assert_ok!(SlpV2::set_protocol_fee_rate(RuntimeOrigin::root(), staking_protocol, fee_rate));
		assert_noop!(
			SlpV2::set_protocol_fee_rate(RuntimeOrigin::root(), staking_protocol, fee_rate),
			SlpV2Error::<Test>::InvalidParameter
		);
	})
}

#[test]
fn set_update_ongoing_time_unit_interval_should_work() {
	new_test_ext().execute_with(|| {
		let update_interval = 100u64;
		let staking_protocol = StakingProtocol::AstarDappStaking;
		assert_ok!(SlpV2::set_update_ongoing_time_unit_interval(
			RuntimeOrigin::root(),
			staking_protocol,
			update_interval
		));
		expect_event(SlpV2Event::SetUpdateOngoingTimeUnitInterval {
			staking_protocol,
			update_interval,
		});
		assert_eq!(
			UpdateOngoingTimeUintIntervalByStakingProtocol::<Test>::get(staking_protocol),
			update_interval
		);
	})
}

#[test]
fn set_update_ongoing_time_unit_interval_invalid_parameter() {
	new_test_ext().execute_with(|| {
		let update_interval = 0u64;
		let staking_protocol = StakingProtocol::AstarDappStaking;
		assert_noop!(
			SlpV2::set_update_ongoing_time_unit_interval(
				RuntimeOrigin::root(),
				staking_protocol,
				update_interval
			),
			SlpV2Error::<Test>::InvalidParameter
		);

		let update_interval = 100u64;
		assert_ok!(SlpV2::set_update_ongoing_time_unit_interval(
			RuntimeOrigin::root(),
			staking_protocol,
			update_interval
		));
		assert_noop!(
			SlpV2::set_update_ongoing_time_unit_interval(
				RuntimeOrigin::root(),
				staking_protocol,
				update_interval
			),
			SlpV2Error::<Test>::InvalidParameter
		);
	})
}

#[test]
fn set_update_token_exchange_rate_limit_should_work() {
	new_test_ext().execute_with(|| {
		let update_interval = 100u64;
		let max_update_permill = Permill::from_perthousand(1);
		let staking_protocol = StakingProtocol::AstarDappStaking;
		assert_ok!(SlpV2::set_update_token_exchange_rate_limit(
			RuntimeOrigin::root(),
			staking_protocol,
			update_interval,
			max_update_permill
		));
		expect_event(SlpV2Event::SetUpdateTokenExchangeRateLimit {
			staking_protocol,
			update_interval,
			max_update_permill,
		});
		assert_eq!(
			UpdateTokenExchangeRateLimitByStakingProtocol::<Test>::get(staking_protocol),
			(update_interval, max_update_permill)
		);
	})
}

#[test]
fn set_update_token_exchange_rate_limit_invalid_parameter() {
	new_test_ext().execute_with(|| {
		let update_interval = 0u64;
		let max_update_permill = Permill::from_perthousand(0);
		let staking_protocol = StakingProtocol::AstarDappStaking;
		assert_noop!(
			SlpV2::set_update_token_exchange_rate_limit(
				RuntimeOrigin::root(),
				staking_protocol,
				update_interval,
				max_update_permill
			),
			SlpV2Error::<Test>::InvalidParameter
		);

		let update_interval = 100u64;
		let max_update_permill = Permill::from_perthousand(1);
		assert_ok!(SlpV2::set_update_token_exchange_rate_limit(
			RuntimeOrigin::root(),
			staking_protocol,
			update_interval,
			max_update_permill
		));
		assert_noop!(
			SlpV2::set_update_token_exchange_rate_limit(
				RuntimeOrigin::root(),
				staking_protocol,
				update_interval,
				max_update_permill
			),
			SlpV2Error::<Test>::InvalidParameter
		);
	})
}

#[test]
fn set_xcm_task_fee_should_work() {
	new_test_ext().execute_with(|| {
		let xcm_task = XcmTask::AstarDappStakingLock;
		let xcm_fee = XcmFee { weight: Default::default(), fee: 100 };
		assert_ok!(SlpV2::set_xcm_task_fee(RuntimeOrigin::root(), xcm_task, xcm_fee));

		expect_event(SlpV2Event::SetXcmFee { xcm_task, xcm_fee });
		assert_eq!(XcmFeeByXcmTask::<Test>::get(xcm_task), Some(xcm_fee));
	})
}

#[test]
fn set_xcm_task_fee_invalid_parameter() {
	new_test_ext().execute_with(|| {
		let xcm_task = XcmTask::AstarDappStakingLock;
		let xcm_fee = XcmFee { weight: Default::default(), fee: 100 };
		assert_ok!(SlpV2::set_xcm_task_fee(RuntimeOrigin::root(), xcm_task, xcm_fee));
		assert_noop!(
			SlpV2::set_xcm_task_fee(RuntimeOrigin::root(), xcm_task, xcm_fee),
			SlpV2Error::<Test>::InvalidParameter
		);
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
fn set_operator_should_work() {
	new_test_ext().execute_with(|| {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		let operator = AccountId::from([0u8; 32]);
		assert_ok!(SlpV2::set_operator(RuntimeOrigin::root(), staking_protocol, operator.clone()));
		expect_event(SlpV2Event::SetOperator { staking_protocol, operator: operator.clone() });
		assert_eq!(OperatorByStakingProtocol::<Test>::get(staking_protocol), Some(operator));
	})
}

#[test]
fn set_operator_invaild_parameter() {
	new_test_ext().execute_with(|| {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		let operator = AccountId::from([0u8; 32]);
		assert_ok!(SlpV2::set_operator(RuntimeOrigin::root(), staking_protocol, operator.clone()));
		assert_noop!(
			SlpV2::set_operator(RuntimeOrigin::root(), staking_protocol, operator),
			SlpV2Error::<Test>::InvalidParameter
		);
	})
}

#[test]
fn update_ongoing_time_unit_should_work() {
	new_test_ext().execute_with(|| {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		let currency_id = staking_protocol.get_currency_id();
		assert_ok!(SlpV2::update_ongoing_time_unit(RuntimeOrigin::root(), staking_protocol));
		expect_event(SlpV2Event::TimeUnitUpdated { staking_protocol, time_unit: TimeUnit::Era(1) });
		assert_eq!(VtokenMinting::get_ongoing_time_unit(currency_id), Some(TimeUnit::Era(1)));
		assert_eq!(LastUpdateOngoingTimeUnitBlockNumber::<Test>::get(staking_protocol), 1);

		RelaychainDataProvider::set_block_number(2);

		assert_ok!(SlpV2::update_ongoing_time_unit(RuntimeOrigin::root(), staking_protocol));
		expect_event(SlpV2Event::TimeUnitUpdated { staking_protocol, time_unit: TimeUnit::Era(2) });
		assert_eq!(VtokenMinting::get_ongoing_time_unit(currency_id), Some(TimeUnit::Era(2)));
		assert_eq!(LastUpdateOngoingTimeUnitBlockNumber::<Test>::get(staking_protocol), 2);
	});
}

#[test]
fn update_ongoing_time_unit_update_interval_too_short() {
	new_test_ext().execute_with(|| {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		let update_interval = 100u64;

		assert_ok!(SlpV2::set_update_ongoing_time_unit_interval(
			RuntimeOrigin::root(),
			staking_protocol,
			update_interval
		));

		// current relaychain block number 1 < update_interval 100 + last update block number 0 =>
		// Error
		assert_noop!(
			SlpV2::update_ongoing_time_unit(RuntimeOrigin::root(), staking_protocol),
			SlpV2Error::<Test>::UpdateOngoingTimeUnitIntervalTooShort
		);

		RelaychainDataProvider::set_block_number(100);
		// current relaychain block number 100 = update_interval 100 + last update block number 0 =>
		// Ok
		assert_ok!(SlpV2::update_ongoing_time_unit(RuntimeOrigin::root(), staking_protocol));

		RelaychainDataProvider::set_block_number(199);
		// current relaychain block number 199 < update_interval 100 + last update block number 100
		// => Error
		assert_noop!(
			SlpV2::update_ongoing_time_unit(RuntimeOrigin::root(), staking_protocol),
			SlpV2Error::<Test>::UpdateOngoingTimeUnitIntervalTooShort
		);
		RelaychainDataProvider::set_block_number(200);
		// current relaychain block number 200 = update_interval 100 + last update block number 100
		// => Ok
		assert_ok!(SlpV2::update_ongoing_time_unit(RuntimeOrigin::root(), staking_protocol));
	});
}

#[test]
fn update_token_exchange_rate_should_work() {
	new_test_ext().execute_with(|| {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		let currency_id = staking_protocol.get_currency_id();
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);
		let amount = 10_059_807_133_828_175_000_000u128;
		let token_pool = 24_597_119_664_064_597_684_680_531u128;
		let vtoken_total_issuance = 21_728_134_208_272_171_009_169_962u128;

		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));
		Currencies::set_balance(VASTR, &AccountId::from([0u8; 32]), vtoken_total_issuance);
		assert_eq!(Currencies::total_issuance(VASTR), vtoken_total_issuance);

		// No protocol fee rate
		assert_ok!(SlpV2::update_token_exchange_rate(
			RuntimeOrigin::root(),
			staking_protocol,
			delegator.clone(),
			token_pool
		));
		assert_eq!(VtokenMinting::get_token_pool(currency_id), token_pool);
		expect_event(SlpV2Event::TokenExchangeRateUpdated {
			staking_protocol,
			delegator: delegator.clone(),
			protocol_fee_currency_id: VASTR,
			protocol_fee: 0,
			amount: token_pool,
		});

		RelaychainDataProvider::set_block_number(2);

		// Set protocol fee rate is 10%
		let protocol_fee_rate = Permill::from_perthousand(100);
		assert_ok!(SlpV2::set_protocol_fee_rate(
			RuntimeOrigin::root(),
			staking_protocol,
			protocol_fee_rate
		));

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

		// Set update token exchange rate limit(update_interval 100, max_update_permill 0.10%)
		let update_interval = 100u64;
		let max_update_permill = Permill::from_perthousand(1);
		// The max_update_amount is 24597.119664064597684680 ASTR
		let max_update_amount = max_update_permill.mul_floor(token_pool);
		println!("max_update_amount: {:?}", max_update_amount);
		assert_ok!(SlpV2::set_update_token_exchange_rate_limit(
			RuntimeOrigin::root(),
			staking_protocol,
			update_interval,
			max_update_permill
		));

		RelaychainDataProvider::set_block_number(102);
		// current relaychain block number 102 = update_interval 100 + last update block number 2 =>
		// Ok
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
		let currency_id = staking_protocol.get_currency_id();
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o").unwrap(),
		);
		let amount = 1000u128;
		let token_pool = 12000u128;
		let vtoken_total_issuance = 10000u128;

		assert_ok!(SlpV2::add_delegator(RuntimeOrigin::root(), STAKING_PROTOCOL, None));
		Currencies::set_balance(VASTR, &AccountId::from([0u8; 32]), vtoken_total_issuance);
		assert_ok!(VtokenMinting::increase_token_pool(currency_id, token_pool));

		// Set protocol fee rate is 10%
		let protocol_fee_rate = Permill::from_perthousand(100);
		assert_ok!(SlpV2::set_protocol_fee_rate(
			RuntimeOrigin::root(),
			staking_protocol,
			protocol_fee_rate
		));
		// Set update token exchange rate limit(update_interval 100, max_update_permill 0.10%)
		let update_interval = 100u64;
		let max_update_permill = Permill::from_perthousand(1);
		// 12000 * 0.001 = 12
		let max_update_amount = max_update_permill.mul_floor(token_pool);
		println!("max_update_amount: {:?}", max_update_amount);
		assert_ok!(SlpV2::set_update_token_exchange_rate_limit(
			RuntimeOrigin::root(),
			staking_protocol,
			update_interval,
			max_update_permill
		));

		// current relaychain block number 1 < update_interval 100 + last update block number 0 =>
		// Error
		assert_noop!(
			SlpV2::update_token_exchange_rate(
				RuntimeOrigin::root(),
				staking_protocol,
				delegator.clone(),
				amount
			),
			SlpV2Error::<Test>::UpdateTokenExchangeRateIntervalTooShort
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
