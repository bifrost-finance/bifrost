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
	astar_dapp_staking::types::{
		AstarDappStakingLedger, AstarDappStakingPendingStatus, AstarValidator, DappStaking,
	},
	common::types::{Ledger, PendingStatus, StakingProtocol, Validator, XcmFee},
	Pallet as SlpV2,
};
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_core::{crypto::Ss58Codec, H160};
use sp_runtime::{AccountId32 as AccountId, Permill};
use xcm::v4::MaybeErrorCode;

pub const STAKING_PROTOCOL: StakingProtocol = StakingProtocol::AstarDappStaking;

fn do_set_protocol_configuration<T: Config>()
where
	<T as frame_system::Config>::AccountId: From<sp_runtime::AccountId32>,
{
	assert_ok!(SlpV2::<T>::set_protocol_configuration(
		RawOrigin::Root.into(),
		STAKING_PROTOCOL,
		ProtocolConfiguration {
			xcm_task_fee: XcmFee { weight: Weight::zero(), fee: 100 },
			protocol_fee_rate: Permill::from_perthousand(100),
			unlock_period: TimeUnit::Era(9),
			operator: AccountId::from([0u8; 32]).into(),
			max_update_token_exchange_rate: Permill::from_perthousand(1),
			update_time_unit_interval: 100u32,
			update_exchange_rate_interval: 100u32,
		}
	));
}

#[benchmarks(where <T as frame_system::Config>::AccountId: From<sp_runtime::AccountId32>)]
mod benchmarks {
	use super::*;

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
		assert_ok!(SlpV2::<T>::add_delegator(RawOrigin::Root.into(), STAKING_PROTOCOL, None));
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
		assert_ok!(SlpV2::<T>::add_delegator(RawOrigin::Root.into(), STAKING_PROTOCOL, None));
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
	fn set_protocol_configuration() -> Result<(), BenchmarkError> {
		#[extrinsic_call]
		_(
			RawOrigin::Root,
			STAKING_PROTOCOL,
			ProtocolConfiguration {
				xcm_task_fee: XcmFee { weight: Weight::zero(), fee: 100 },
				protocol_fee_rate: Permill::from_perthousand(100),
				unlock_period: TimeUnit::Era(9),
				operator: AccountId::from([0u8; 32]).into(),
				max_update_token_exchange_rate: Permill::from_perthousand(1),
				update_time_unit_interval: 100u32,
				update_exchange_rate_interval: 100u32,
			},
		);
		Ok(())
	}

	#[benchmark]
	fn set_ledger() -> Result<(), BenchmarkError> {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o")
				.unwrap()
				.into(),
		);
		let ledger = Ledger::AstarDappStaking(AstarDappStakingLedger {
			locked: 100,
			unlocking: Default::default(),
		});
		assert_ok!(SlpV2::<T>::add_delegator(RawOrigin::Root.into(), STAKING_PROTOCOL, None));

		#[extrinsic_call]
		_(RawOrigin::Root, STAKING_PROTOCOL, delegator, ledger);
		Ok(())
	}

	#[benchmark]
	fn transfer_to() -> Result<(), BenchmarkError> {
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
	fn transfer_back() -> Result<(), BenchmarkError> {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o")
				.unwrap()
				.into(),
		);
		assert_ok!(SlpV2::<T>::add_delegator(RawOrigin::Root.into(), STAKING_PROTOCOL, None));
		do_set_protocol_configuration::<T>();
		#[extrinsic_call]
		_(RawOrigin::Root, STAKING_PROTOCOL, delegator, 1000);
		Ok(())
	}

	#[benchmark]
	fn update_ongoing_time_unit() -> Result<(), BenchmarkError> {
		#[extrinsic_call]
		_(RawOrigin::Root, STAKING_PROTOCOL, Some(TimeUnit::Era(1)));
		Ok(())
	}

	#[benchmark]
	fn update_token_exchange_rate() -> Result<(), BenchmarkError> {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o")
				.unwrap()
				.into(),
		);
		assert_ok!(SlpV2::<T>::add_delegator(RawOrigin::Root.into(), STAKING_PROTOCOL, None));
		#[extrinsic_call]
		_(RawOrigin::Root, STAKING_PROTOCOL, delegator, 1000);
		Ok(())
	}

	#[benchmark]
	fn astar_dapp_staking() -> Result<(), BenchmarkError> {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o")
				.unwrap()
				.into(),
		);
		assert_ok!(SlpV2::<T>::add_delegator(RawOrigin::Root.into(), STAKING_PROTOCOL, None));
		do_set_protocol_configuration::<T>();
		let task = DappStaking::Lock(100);
		#[extrinsic_call]
		_(RawOrigin::Root, delegator, task);
		Ok(())
	}

	#[benchmark]
	fn notify_astar_dapp_staking() -> Result<(), BenchmarkError> {
		let delegator = Delegator::Substrate(
			AccountId::from_ss58check("YLF9AnL6V1vQRfuiB832NXNGZYCPAWkKLLkh7cf3KwXhB9o")
				.unwrap()
				.into(),
		);
		assert_ok!(SlpV2::<T>::add_delegator(RawOrigin::Root.into(), STAKING_PROTOCOL, None));
		do_set_protocol_configuration::<T>();

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
