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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg(feature = "runtime-benchmarks")]

use crate::*;
use frame_benchmarking::v2::*;
use frame_support::assert_ok;

const DELEGATOR1: MultiLocation =
	MultiLocation { parents: 1, interior: X1(AccountId32 { network: None, id: [1u8; 32] }) };
const DELEGATOR2: MultiLocation =
	MultiLocation { parents: 1, interior: X1(AccountId32 { network: None, id: [2u8; 32] }) };

pub fn set_mins_and_maxs<T: Config>(origin: <T as frame_system::Config>::RuntimeOrigin) {
	let mins_and_maxs = MinimumsMaximums {
		delegator_bonded_minimum: 0u32.into(),

		bond_extra_minimum: 0u32.into(),
		unbond_minimum: 0u32.into(),
		rebond_minimum: 0u32.into(),
		unbond_record_maximum: 5u32,
		validators_back_maximum: 100u32,
		delegator_active_staking_maximum: 1_000_000_000u32.into(),
		validators_reward_maximum: 300u32,
		delegation_amount_minimum: 0u32.into(),
		delegators_maximum: 10,
		validators_maximum: 10,
	};

	// Set minimums and maximums
	assert_ok!(Pallet::<T>::set_minimums_and_maximums(origin, KSM, Some(mins_and_maxs)));
}

pub fn init_bond<T: Config>(origin: <T as frame_system::Config>::RuntimeOrigin) {
	set_mins_and_maxs::<T>(origin.clone());
	DelegatorsMultilocation2Index::<T>::insert(KSM, DELEGATOR1, 0);

	T::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		KSM,
		XcmOperationType::Bond,
		Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
	)
	.unwrap();

	assert_ok!(Pallet::<T>::bond(origin, KSM, Box::new(DELEGATOR1), 10u32.into(), None, None));
}

pub fn init_ongoing_time<T: Config>(origin: <T as frame_system::Config>::RuntimeOrigin) {
	assert_ok!(Pallet::<T>::set_ongoing_time_unit_update_interval(
		origin.clone(),
		KSM,
		Some(0u32.into())
	));

	// Initialize ongoing timeunit as 1.
	assert_ok!(Pallet::<T>::update_ongoing_time_unit(origin.clone(), KSM, TimeUnit::Era(0)));

	let delay =
		Delays { unlock_delay: TimeUnit::Era(0), leave_delegators_delay: Default::default() };
	assert_ok!(Pallet::<T>::set_currency_delays(origin.clone(), KSM, Some(delay)));
}

#[benchmarks(where T: Config + orml_tokens::Config<CurrencyId = CurrencyId> + bifrost_vtoken_minting::Config)]
mod benchmarks {
	use super::*;
	use crate::primitives::{PhalaLedger, SubstrateValidatorsByDelegatorUpdateEntry};
	use bifrost_primitives::VKSM;
	use frame_benchmarking::impl_benchmark_test_suite;
	use sp_arithmetic::traits::SaturatedConversion;

	#[benchmark]
	fn initialize_delegator() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		set_mins_and_maxs::<T>(origin.clone());

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, None);

		Ok(())
	}

	#[benchmark]
	fn bond() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		set_mins_and_maxs::<T>(origin.clone());
		DelegatorsMultilocation2Index::<T>::insert(KSM, DELEGATOR1, 0);

		T::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
			KSM,
			XcmOperationType::Bond,
			Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
		)?;

		#[extrinsic_call]
		_(
			origin as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Box::new(DELEGATOR1),
			10u32.into(),
			None,
			None,
		);

		Ok(())
	}

	#[benchmark]
	fn bond_extra() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		init_bond::<T>(origin.clone());

		T::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
			KSM,
			XcmOperationType::BondExtra,
			Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
		)?;

		#[extrinsic_call]
		_(
			origin as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Box::new(DELEGATOR1),
			None,
			10u32.into(),
			None,
		);

		Ok(())
	}

	#[benchmark]
	fn unbond() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		init_bond::<T>(origin.clone());
		init_ongoing_time::<T>(origin.clone());

		T::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
			KSM,
			XcmOperationType::Unbond,
			Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
		)?;

		#[extrinsic_call]
		_(
			origin as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Box::new(DELEGATOR1),
			None,
			0u32.into(),
			None,
		);

		Ok(())
	}

	#[benchmark]
	fn unbond_all() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		init_bond::<T>(origin.clone());
		init_ongoing_time::<T>(origin.clone());

		T::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
			KSM,
			XcmOperationType::Unbond,
			Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
		)?;

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, Box::new(DELEGATOR1), None);

		Ok(())
	}

	#[benchmark]
	fn rebond() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		init_bond::<T>(origin.clone());
		init_ongoing_time::<T>(origin.clone());

		T::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
			KSM,
			XcmOperationType::Rebond,
			Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
		)?;

		#[extrinsic_call]
		_(
			origin as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Box::new(DELEGATOR1),
			None,
			Some(0u32.into()),
			None,
		);

		Ok(())
	}

	#[benchmark]
	fn delegate() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		init_bond::<T>(origin.clone());
		Validators::<T>::insert(KSM, BoundedVec::try_from(vec![DELEGATOR1]).unwrap());

		T::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
			KSM,
			XcmOperationType::Delegate,
			Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
		)?;

		#[extrinsic_call]
		_(
			origin as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Box::new(DELEGATOR1),
			vec![DELEGATOR1],
			None,
		);

		Ok(())
	}

	#[benchmark]
	fn undelegate() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		init_bond::<T>(origin.clone());
		ValidatorsByDelegator::<T>::insert(
			KSM,
			DELEGATOR1,
			BoundedVec::try_from(vec![DELEGATOR1, DELEGATOR2]).unwrap(),
		);

		T::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
			KSM,
			XcmOperationType::Delegate,
			Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
		)?;

		#[extrinsic_call]
		_(
			origin as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Box::new(DELEGATOR1),
			vec![DELEGATOR1],
			None,
		);

		Ok(())
	}

	#[benchmark]
	fn redelegate() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		init_bond::<T>(origin.clone());
		Validators::<T>::insert(KSM, BoundedVec::try_from(vec![DELEGATOR1]).unwrap());

		T::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
			KSM,
			XcmOperationType::Delegate,
			Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
		)?;

		#[extrinsic_call]
		_(
			origin as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Box::new(DELEGATOR1),
			Some(vec![DELEGATOR1]),
			None,
		);

		Ok(())
	}

	#[benchmark]
	fn payout() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		DelegatorsMultilocation2Index::<T>::insert(KSM, DELEGATOR1, 0);

		T::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
			KSM,
			XcmOperationType::Payout,
			Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
		)?;

		#[extrinsic_call]
		_(
			origin as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Box::new(DELEGATOR1),
			Box::new(DELEGATOR1),
			Some(TimeUnit::Era(0)),
			None,
		);

		Ok(())
	}

	#[benchmark]
	fn liquidize() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		set_mins_and_maxs::<T>(origin.clone());
		DelegatorsMultilocation2Index::<T>::insert(KSM, DELEGATOR1, 0);

		T::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
			KSM,
			XcmOperationType::Liquidize,
			Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
		)?;

		#[extrinsic_call]
		_(
			origin as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Box::new(DELEGATOR1),
			Some(TimeUnit::SlashingSpan(0)),
			None,
			None,
			None,
		);

		Ok(())
	}

	#[benchmark]
	fn chill() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		init_bond::<T>(origin.clone());
		init_ongoing_time::<T>(origin.clone());

		T::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
			KSM,
			XcmOperationType::Chill,
			Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
		)?;

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, Box::new(DELEGATOR1), None);

		Ok(())
	}

	#[benchmark]
	fn transfer_back() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		set_mins_and_maxs::<T>(origin.clone());
		DelegatorsMultilocation2Index::<T>::insert(KSM, DELEGATOR1, 0);

		T::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
			KSM,
			XcmOperationType::TransferBack,
			Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
		)?;

		let (_, exit_account) = <T as Config>::VtokenMinting::get_entrance_and_exit_accounts();
		let exit_account_32 = Pallet::<T>::account_id_to_account_32(exit_account).unwrap();
		let to = Pallet::<T>::account_32_to_parent_location(exit_account_32).unwrap();

		#[extrinsic_call]
		_(
			origin as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Box::new(DELEGATOR1),
			Box::new(to),
			10u32.into(),
			None,
		);

		Ok(())
	}

	#[benchmark]
	fn transfer_to() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		set_mins_and_maxs::<T>(origin.clone());
		DelegatorsMultilocation2Index::<T>::insert(KSM, DELEGATOR1, 0);

		T::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
			KSM,
			XcmOperationType::TransferTo,
			Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
		)?;

		let (entrance_account, _) = <T as Config>::VtokenMinting::get_entrance_and_exit_accounts();
		let entrance_account_32 = Pallet::<T>::account_id_to_account_32(entrance_account).unwrap();
		let from = Pallet::<T>::account_32_to_local_location(entrance_account_32).unwrap();

		#[extrinsic_call]
		_(
			origin as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Box::new(from),
			Box::new(DELEGATOR1),
			10u32.into(),
		);

		Ok(())
	}

	#[benchmark]
	fn convert_asset() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		DelegatorsMultilocation2Index::<T>::insert(PHA, DELEGATOR1, 0);

		T::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
			PHA,
			XcmOperationType::ConvertAsset,
			Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
		)?;

		DelegatorLedgers::<T>::insert(
			PHA,
			DELEGATOR1,
			Ledger::Phala(PhalaLedger {
				account: DELEGATOR1,
				active_shares: 10u32.into(),
				unlocking_shares: 10u32.into(),
				unlocking_time_unit: None,
				bonded_pool_id: None,
				bonded_pool_collection_id: None,
				bonded_is_vault: None,
			}),
		);

		#[extrinsic_call]
		_(
			origin as <T as frame_system::Config>::RuntimeOrigin,
			PHA,
			Box::new(DELEGATOR1),
			10u32.into(),
			true,
			None,
		);

		Ok(())
	}

	#[benchmark]
	fn increase_token_pool() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, 10u32.into());

		Ok(())
	}

	#[benchmark]
	fn decrease_token_pool() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;

		assert_ok!(Pallet::<T>::increase_token_pool(
			origin.clone() as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			10u32.into()
		));

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, 10u32.into());

		Ok(())
	}

	#[benchmark]
	fn update_ongoing_time_unit() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;

		OngoingTimeUnitUpdateInterval::<T>::insert(KSM, BlockNumberFor::<T>::from(0u32));
		LastTimeUpdatedOngoingTimeUnit::<T>::insert(KSM, BlockNumberFor::<T>::from(0u32));

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, TimeUnit::Era(0));

		Ok(())
	}

	#[benchmark]
	fn refund_currency_due_unbond() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		init_ongoing_time::<T>(origin.clone());

		let (_, exit_account) = T::VtokenMinting::get_entrance_and_exit_accounts();
		orml_tokens::Pallet::<T>::deposit(
			KSM,
			&exit_account,
			<T as orml_tokens::Config>::Balance::saturated_from(1_000_000_000_000u128),
		)
		.unwrap();

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM);

		Ok(())
	}

	#[benchmark]
	fn supplement_fee_reserve() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		set_mins_and_maxs::<T>(origin.clone());

		let (entrance_account, _) = <T as Config>::VtokenMinting::get_entrance_and_exit_accounts();
		let entrance_account_32 = Pallet::<T>::account_id_to_account_32(entrance_account).unwrap();
		let from = Pallet::<T>::account_32_to_local_location(entrance_account_32).unwrap();

		DelegatorsMultilocation2Index::<T>::insert(KSM, DELEGATOR1, 0);

		FeeSources::<T>::insert(KSM, (from, BalanceOf::<T>::from(1000u32)));

		T::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
			KSM,
			XcmOperationType::TransferTo,
			Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
		)?;

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, Box::new(DELEGATOR1));
		Ok(())
	}

	#[benchmark]
	fn charge_host_fee_and_tune_vtoken_exchange_rate() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		init_bond::<T>(origin.clone());
		init_ongoing_time::<T>(origin.clone());

		assert_ok!(Pallet::<T>::increase_token_pool(
			origin.clone() as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			1000u32.into()
		));

		CurrencyTuneExchangeRateLimit::<T>::insert(
			KSM,
			(1000u32, Permill::from_parts(100_0000u32)),
		);
		HostingFees::<T>::insert(KSM, (Permill::from_parts(100_0000u32), DELEGATOR1));

		orml_tokens::Pallet::<T>::deposit(
			VKSM,
			&whitelisted_caller(),
			<T as orml_tokens::Config>::Balance::saturated_from(1_000_000_000_000u128),
		)
		.unwrap();

		#[extrinsic_call]
		_(
			origin as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			10u32.into(),
			Some(DELEGATOR1),
		);

		Ok(())
	}

	#[benchmark]
	fn set_operate_origin() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, Some(whitelisted_caller()));

		Ok(())
	}

	#[benchmark]
	fn set_fee_source() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(
			origin as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Some((DELEGATOR1, 10u32.into())),
		);

		Ok(())
	}

	#[benchmark]
	fn add_delegator() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		set_mins_and_maxs::<T>(origin.clone());

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, 0u16, Box::new(DELEGATOR1));

		Ok(())
	}

	#[benchmark]
	fn remove_delegator() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		init_bond::<T>(origin.clone());

		assert_ok!(Pallet::<T>::add_delegator(
			origin.clone() as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			0,
			Box::new(DELEGATOR1)
		));

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, Box::new(DELEGATOR1));

		Ok(())
	}

	#[benchmark]
	fn add_validator() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		set_mins_and_maxs::<T>(origin.clone());

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, Box::new(DELEGATOR1));

		Ok(())
	}

	#[benchmark]
	fn remove_validator() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		set_mins_and_maxs::<T>(origin.clone());

		assert_ok!(Pallet::<T>::add_validator(
			origin.clone() as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Box::new(DELEGATOR1)
		));

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, Box::new(DELEGATOR1));

		Ok(())
	}

	#[benchmark]
	fn set_validators_by_delegator() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		init_bond::<T>(origin.clone());
		Validators::<T>::insert(KSM, BoundedVec::try_from(vec![DELEGATOR2]).unwrap());

		#[extrinsic_call]
		_(
			origin as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Box::new(DELEGATOR1),
			vec![DELEGATOR2],
		);

		Ok(())
	}

	#[benchmark]
	fn set_delegator_ledger() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		init_bond::<T>(origin.clone());

		let ledger = Box::new(Some(Ledger::Substrate(SubstrateLedger {
			account: Default::default(),
			total: 1000u32.into(),
			active: 1000u32.into(),
			unlocking: vec![],
		})));

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, Box::new(DELEGATOR1), ledger);

		Ok(())
	}

	#[benchmark]
	fn set_minimums_and_maximums() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		let mins_and_maxs = MinimumsMaximums {
			delegator_bonded_minimum: 0u32.into(),

			bond_extra_minimum: 0u32.into(),
			unbond_minimum: 0u32.into(),
			rebond_minimum: 0u32.into(),
			unbond_record_maximum: 5u32,
			validators_back_maximum: 100u32,
			delegator_active_staking_maximum: 1_000_000_000u32.into(),
			validators_reward_maximum: 300u32,
			delegation_amount_minimum: 0u32.into(),
			delegators_maximum: 10,
			validators_maximum: 10,
		};

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, Some(mins_and_maxs));

		Ok(())
	}

	#[benchmark]
	fn set_currency_delays() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		let delay =
			Delays { unlock_delay: TimeUnit::Era(0), leave_delegators_delay: Default::default() };

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, Some(delay));

		Ok(())
	}

	#[benchmark]
	fn set_hosting_fees() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(
			origin as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Some((Permill::from_parts(100_0000u32), DELEGATOR1)),
		);

		Ok(())
	}

	#[benchmark]
	fn set_currency_tune_exchange_rate_limit() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(
			origin as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Some((1000u32, Permill::from_parts(100_0000u32))),
		);

		Ok(())
	}

	#[benchmark]
	fn set_ongoing_time_unit_update_interval() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(
			origin as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Some(BlockNumberFor::<T>::from(100u32)),
		);

		Ok(())
	}

	#[benchmark]
	fn add_supplement_fee_account_to_whitelist() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, Box::new(DELEGATOR1));

		Ok(())
	}

	#[benchmark]
	fn remove_supplement_fee_account_from_whitelist() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;

		assert_ok!(Pallet::<T>::add_supplement_fee_account_to_whitelist(
			origin.clone() as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Box::new(DELEGATOR1)
		));
		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, Box::new(DELEGATOR1));

		Ok(())
	}

	#[benchmark]
	fn confirm_delegator_ledger_query_response() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		init_bond::<T>(origin.clone());

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, 0u64);

		Ok(())
	}

	#[benchmark]
	fn fail_delegator_ledger_query_response() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		init_bond::<T>(origin.clone());

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, 0u64);

		Ok(())
	}

	#[benchmark]
	fn confirm_validators_by_delegator_query_response() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		init_bond::<T>(origin.clone());
		Validators::<T>::insert(KSM, BoundedVec::try_from(vec![DELEGATOR1]).unwrap());

		T::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
			KSM,
			XcmOperationType::Delegate,
			Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
		)?;

		assert_ok!(Pallet::<T>::delegate(
			origin.clone() as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Box::new(DELEGATOR1),
			vec![DELEGATOR1],
			None
		));
		ValidatorsByDelegatorXcmUpdateQueue::<T>::insert(
			1u64,
			(
				ValidatorsByDelegatorUpdateEntry::Substrate(
					SubstrateValidatorsByDelegatorUpdateEntry {
						currency_id: KSM,
						delegator_id: Default::default(),
						validators: vec![],
					},
				),
				BlockNumberFor::<T>::from(10u32),
			),
		);
		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, 1u64);

		Ok(())
	}

	#[benchmark]
	fn fail_validators_by_delegator_query_response() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		init_bond::<T>(origin.clone());
		Validators::<T>::insert(KSM, BoundedVec::try_from(vec![DELEGATOR1]).unwrap());

		T::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
			KSM,
			XcmOperationType::Delegate,
			Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
		)?;

		assert_ok!(Pallet::<T>::delegate(
			origin.clone() as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Box::new(DELEGATOR1),
			vec![DELEGATOR1],
			None
		));
		ValidatorsByDelegatorXcmUpdateQueue::<T>::insert(
			1u64,
			(
				ValidatorsByDelegatorUpdateEntry::Substrate(
					SubstrateValidatorsByDelegatorUpdateEntry {
						currency_id: KSM,
						delegator_id: Default::default(),
						validators: vec![],
					},
				),
				BlockNumberFor::<T>::from(10u32),
			),
		);
		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, 1u64);

		Ok(())
	}

	#[benchmark]
	fn reset_validators() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		set_mins_and_maxs::<T>(origin.clone());

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, vec![DELEGATOR1]);

		Ok(())
	}

	#[benchmark]
	fn set_validator_boost_list() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		set_mins_and_maxs::<T>(origin.clone());

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, vec![DELEGATOR1]);

		Ok(())
	}

	#[benchmark]
	fn add_to_validator_boost_list() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		set_mins_and_maxs::<T>(origin.clone());

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, Box::new(DELEGATOR1));

		Ok(())
	}

	#[benchmark]
	fn remove_from_validator_boot_list() -> Result<(), BenchmarkError> {
		let origin = <T as Config>::ControlOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		set_mins_and_maxs::<T>(origin.clone());

		assert_ok!(Pallet::<T>::add_to_validator_boost_list(
			origin.clone() as <T as frame_system::Config>::RuntimeOrigin,
			KSM,
			Box::new(DELEGATOR1)
		));

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, KSM, Box::new(DELEGATOR1));

		Ok(())
	}

	//   `cargo test -p bifrost-slp --all-features`
	impl_benchmark_test_suite!(
		Pallet,
		crate::mocks::mock_kusama::ExtBuilder::default().build(),
		crate::mocks::mock_kusama::Runtime
	);
}
