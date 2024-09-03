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

#![cfg(feature = "runtime-benchmarks")]
use super::*;
use crate::{
	astar_dapp_staking::types::{AstarDappStakingLedger, AstarValidator, DappStaking},
	common::types::{Ledger, PendingStatus, StakingProtocol, Validator, XcmFee, XcmTask},
	Pallet as SlpV2,
};
use cumulus_primitives_core::BlockT;
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_core::{crypto::Ss58Codec, H160};
use sp_runtime::AccountId32 as AccountId;
use xcm::v4::MaybeErrorCode;

pub const STAKING_PROTOCOL: StakingProtocol = StakingProtocol::AstarDappStaking;

#[benchmarks(where <T as frame_system::Config>::AccountId: From<sp_runtime::AccountId32> , <<<T as frame_system::Config>::Block as BlockT>::Header as sp_runtime::traits::Header>::Number: From<u32>)]
mod benchmarks {
	use super::*;
	use crate::astar_dapp_staking::types::{
		AstarDappStakingPendingStatus, AstarDappStakingXcmTask,
	};

	#[benchmark]
	fn add_delegator() {
		#[extrinsic_call]
		_(RawOrigin::Root, STAKING_PROTOCOL, None);
	}

	#[benchmark]
	fn remove_delegator() -> Result<(), BenchmarkError> {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o")
				.unwrap()
				.into(),
		);
		assert_ok!(SlpV2::<T>::add_delegator(RawOrigin::Root.into(), STAKING_PROTOCOL, None));
		#[extrinsic_call]
		_(RawOrigin::Root, STAKING_PROTOCOL, delegator);

		Ok(())
	}

	#[benchmark]
	fn add_validator() -> Result<(), BenchmarkError> {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o")
				.unwrap()
				.into(),
		);
		let validator = Validator::AstarDappStaking(AstarValidator::Evm(H160::zero()));
		#[extrinsic_call]
		_(RawOrigin::Root, STAKING_PROTOCOL, delegator, validator);
		Ok(())
	}

	#[benchmark]
	fn remove_validator() -> Result<(), BenchmarkError> {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o")
				.unwrap()
				.into(),
		);
		let validator = Validator::AstarDappStaking(AstarValidator::Evm(H160::zero()));
		assert_ok!(SlpV2::<T>::add_validator(
			RawOrigin::Root.into(),
			STAKING_PROTOCOL,
			delegator.clone(),
			validator.clone()
		));
		#[extrinsic_call]
		_(RawOrigin::Root, STAKING_PROTOCOL, delegator, validator);
		Ok(())
	}

	#[benchmark]
	fn set_xcm_task_fee() -> Result<(), BenchmarkError> {
		let xcm_task = XcmTask::AstarDappStaking(AstarDappStakingXcmTask::Lock);
		let xcm_fee = XcmFee { weight: Default::default(), fee: 100 };
		#[extrinsic_call]
		_(RawOrigin::Root, xcm_task, xcm_fee);
		Ok(())
	}

	#[benchmark]
	fn set_protocol_fee_rate() -> Result<(), BenchmarkError> {
		let fee_rate = Permill::from_perthousand(1);
		let staking_protocol = StakingProtocol::AstarDappStaking;
		#[extrinsic_call]
		_(RawOrigin::Root, staking_protocol, fee_rate);
		Ok(())
	}

	#[benchmark]
	fn set_update_ongoing_time_unit_interval() -> Result<(), BenchmarkError> {
		let update_interval = 100u32.into();
		let staking_protocol = StakingProtocol::AstarDappStaking;
		#[extrinsic_call]
		_(RawOrigin::Root, staking_protocol, update_interval);
		Ok(())
	}

	#[benchmark]
	fn set_update_token_exchange_rate_limit() -> Result<(), BenchmarkError> {
		let update_interval = 100u32.into();
		let max_update_permill = Permill::from_perthousand(1);
		let staking_protocol = StakingProtocol::AstarDappStaking;
		#[extrinsic_call]
		_(RawOrigin::Root, staking_protocol, update_interval, max_update_permill);
		Ok(())
	}

	#[benchmark]
	fn set_ledger() -> Result<(), BenchmarkError> {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o")
				.unwrap()
				.into(),
		);
		let ledger = Ledger::AstarDappStaking(AstarDappStakingLedger {
			locked: 100,
			unlocking: Default::default(),
		});
		assert_ok!(SlpV2::<T>::add_delegator(RawOrigin::Root.into(), staking_protocol, None));

		#[extrinsic_call]
		_(RawOrigin::Root, staking_protocol, delegator, ledger);
		Ok(())
	}

	#[benchmark]
	fn set_operator() -> Result<(), BenchmarkError> {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		let operator = AccountId::from([0u8; 32]).into();
		#[extrinsic_call]
		_(RawOrigin::Root, staking_protocol, operator);
		Ok(())
	}

	#[benchmark]
	fn transfer_to() -> Result<(), BenchmarkError> {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o")
				.unwrap()
				.into(),
		);
		assert_ok!(SlpV2::<T>::add_delegator(RawOrigin::Root.into(), staking_protocol, None));
		#[extrinsic_call]
		_(RawOrigin::Root, staking_protocol, delegator);
		Ok(())
	}

	#[benchmark]
	fn transfer_back() -> Result<(), BenchmarkError> {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o")
				.unwrap()
				.into(),
		);
		assert_ok!(SlpV2::<T>::add_delegator(RawOrigin::Root.into(), staking_protocol, None));
		let xcm_task = XcmTask::AstarDappStaking(AstarDappStakingXcmTask::TransferBack);
		let xcm_fee = XcmFee { weight: Default::default(), fee: 100 };
		assert_ok!(SlpV2::<T>::set_xcm_task_fee(RawOrigin::Root.into(), xcm_task, xcm_fee));
		#[extrinsic_call]
		_(RawOrigin::Root, staking_protocol, delegator, 1000);
		Ok(())
	}

	#[benchmark]
	fn update_ongoing_time_unit() -> Result<(), BenchmarkError> {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		#[extrinsic_call]
		_(RawOrigin::Root, staking_protocol, Some(TimeUnit::Era(1)));
		Ok(())
	}

	#[benchmark]
	fn update_token_exchange_rate() -> Result<(), BenchmarkError> {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o")
				.unwrap()
				.into(),
		);
		assert_ok!(SlpV2::<T>::add_delegator(RawOrigin::Root.into(), staking_protocol, None));
		#[extrinsic_call]
		_(RawOrigin::Root, staking_protocol, delegator, 1000);
		Ok(())
	}

	#[benchmark]
	fn astar_dapp_staking() -> Result<(), BenchmarkError> {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o")
				.unwrap()
				.into(),
		);
		assert_ok!(SlpV2::<T>::add_delegator(RawOrigin::Root.into(), staking_protocol, None));
		let xcm_task = XcmTask::AstarDappStaking(AstarDappStakingXcmTask::Lock);
		let xcm_fee = XcmFee { weight: Default::default(), fee: 100 };
		assert_ok!(SlpV2::<T>::set_xcm_task_fee(RawOrigin::Root.into(), xcm_task, xcm_fee));
		let task = DappStaking::Lock(100);
		#[extrinsic_call]
		_(RawOrigin::Root, delegator, task);
		Ok(())
	}

	#[benchmark]
	fn notify_astar_dapp_staking() -> Result<(), BenchmarkError> {
		let staking_protocol = StakingProtocol::AstarDappStaking;
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o")
				.unwrap()
				.into(),
		);
		assert_ok!(SlpV2::<T>::add_delegator(RawOrigin::Root.into(), staking_protocol, None));
		let xcm_task = XcmTask::AstarDappStaking(AstarDappStakingXcmTask::Lock);
		let xcm_fee = XcmFee { weight: Default::default(), fee: 100 };
		assert_ok!(SlpV2::<T>::set_xcm_task_fee(RawOrigin::Root.into(), xcm_task, xcm_fee));

		PendingStatusByQueryId::<T>::insert(
			0,
			PendingStatus::AstarDappStaking(AstarDappStakingPendingStatus::Lock(
				delegator.clone(),
				100,
			)),
		);
		#[extrinsic_call]
		_(RawOrigin::Root, 0, xcm::v4::Response::DispatchResult(MaybeErrorCode::Success));
		Ok(())
	}

	impl_benchmark_test_suite!(SlpV2, crate::mock::new_test_ext(), crate::mock::Test);
}
