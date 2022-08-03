// This file is part of Bifrost.

// Copyright (C) 2019-2022 Liebi Technologies (UK) Ltd.
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

use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::{assert_ok, dispatch::UnfilteredDispatchable};
use frame_system::RawOrigin;
use sp_runtime::traits::UniqueSaturatedFrom;

#[allow(unused_imports)]
pub use crate::{Pallet as Slp, *};

use crate::KSM;

// pub fn lookup_of_account<T: Config>(who: T::AccountId) -> <<T as frame_system::Config>::Lookup as
// StaticLookup>::Source { 	<T as frame_system::Config>::Lookup::unlookup(who)
// }

fn kusama_setup<T: Config>() -> DispatchResult {
	let origin = T::ControlOrigin::successful_origin();
	let caller: T::AccountId = whitelisted_caller();

	let validator_0_account_id_20: [u8; 20] =
		hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();

	let validator_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
		),
	};

	let treasury_account_id_32: [u8; 32] =
		hex_literal::hex!["6d6f646c62662f74727372790000000000000000000000000000000000000000"]
			.into();
	// let treasury_account :T::AccountId = sp_runtime::AccountId32::from(treasury_account_id_32);
	let treasury_location = MultiLocation {
		parents: 0,
		interior: X1(AccountId32 { network: Any, id: treasury_account_id_32 }),
	};

	// set operate_origins
	assert_ok!(Slp::<T>::set_operate_origin(origin.clone(), KSM, Some(caller.clone())));

	// Set OngoingTimeUnitUpdateInterval as 1/3 round(600 blocks per round, 12 seconds per block)
	assert_ok!(Slp::<T>::set_ongoing_time_unit_update_interval(
		origin.clone(),
		KSM,
		Some(BlockNumberFor::<T>::from(200u32))
	));

	<frame_system::Pallet<T>>::set_block_number(BlockNumberFor::<T>::from(300u32));

	// Initialize ongoing timeunit as 1.
	assert_ok!(Slp::<T>::update_ongoing_time_unit(
		RawOrigin::Signed(caller.clone()).into(),
		KSM,
		TimeUnit::Era(0)
	));

	// Initialize currency delays.
	let delay =
		Delays { unlock_delay: TimeUnit::Era(0), leave_delegators_delay: Default::default() };
	assert_ok!(Slp::<T>::set_currency_delays(origin.clone(), KSM, Some(delay)));

	// First to setup index-multilocation relationship of subaccount_0
	assert_ok!(Slp::<T>::initialize_delegator(origin.clone(), KSM));
	//DelegatorNotExist

	// // update some KSM balance to treasury account
	// assert_ok!(Tokens::Pallet::<T>::set_balance(
	// 	RawOrigin::Root.into(),
	// 	lookup_of_account(treasury_account.clone()),
	// 	KSM,
	// 	1_000_000_000_000_000_000.into(),
	// 	0.into()
	// ));

	// Set fee source
	assert_ok!(Slp::<T>::set_fee_source(
		origin.clone(),
		KSM,
		Some((treasury_location, BalanceOf::<T>::unique_saturated_from(1_000_000_000_000u128))),
	));

	assert_ok!(Slp::<T>::set_xcm_dest_weight_and_fee(
		origin.clone(),
		KSM,
		XcmOperation::Bond,
		Some((0, BalanceOf::<T>::unique_saturated_from(0u128))),
	));

	assert_ok!(Slp::<T>::set_xcm_dest_weight_and_fee(
		origin.clone(),
		KSM,
		XcmOperation::BondExtra,
		Some((20_000_000_000, BalanceOf::<T>::unique_saturated_from(10_000_000_000u128))),
	));

	assert_ok!(Slp::<T>::set_xcm_dest_weight_and_fee(
		origin.clone(),
		KSM,
		XcmOperation::Unbond,
		Some((20_000_000_000, BalanceOf::<T>::unique_saturated_from(10_000_000_000u128))),
	));

	assert_ok!(Slp::<T>::set_xcm_dest_weight_and_fee(
		origin.clone(),
		KSM,
		XcmOperation::Chill,
		Some((20_000_000_000, BalanceOf::<T>::unique_saturated_from(10_000_000_000u128))),
	));

	assert_ok!(Slp::<T>::set_xcm_dest_weight_and_fee(
		origin.clone(),
		KSM,
		XcmOperation::Rebond,
		Some((20_000_000_000, BalanceOf::<T>::unique_saturated_from(10_000_000_000u128))),
	));

	assert_ok!(Slp::<T>::set_xcm_dest_weight_and_fee(
		origin.clone(),
		KSM,
		XcmOperation::Undelegate,
		Some((20_000_000_000, BalanceOf::<T>::unique_saturated_from(10_000_000_000u128))),
	));

	assert_ok!(Slp::<T>::set_xcm_dest_weight_and_fee(
		origin.clone(),
		KSM,
		XcmOperation::Delegate,
		Some((20_000_000_000, BalanceOf::<T>::unique_saturated_from(10_000_000_000u128))),
	));

	assert_ok!(Slp::<T>::set_xcm_dest_weight_and_fee(
		origin.clone(),
		KSM,
		XcmOperation::Payout,
		Some((20_000_000_000, BalanceOf::<T>::unique_saturated_from(10_000_000_000u128))),
	));

	assert_ok!(Slp::<T>::set_xcm_dest_weight_and_fee(
		origin.clone(),
		KSM,
		XcmOperation::CancelLeave,
		Some((20_000_000_000, BalanceOf::<T>::unique_saturated_from(10_000_000_000u128))),
	));

	assert_ok!(Slp::<T>::set_xcm_dest_weight_and_fee(
		origin.clone(),
		KSM,
		XcmOperation::Liquidize,
		Some((20_000_000_000, BalanceOf::<T>::unique_saturated_from(10_000_000_000u128))),
	));

	assert_ok!(Slp::<T>::set_xcm_dest_weight_and_fee(
		origin.clone(),
		KSM,
		XcmOperation::ExecuteLeave,
		Some((20_000_000_000, BalanceOf::<T>::unique_saturated_from(10_000_000_000u128))),
	));

	assert_ok!(Slp::<T>::set_xcm_dest_weight_and_fee(
		origin.clone(),
		KSM,
		XcmOperation::TransferBack,
		Some((20_000_000_000, BalanceOf::<T>::unique_saturated_from(10_000_000_000u128))),
	));

	assert_ok!(Slp::<T>::set_xcm_dest_weight_and_fee(
		origin.clone(),
		KSM,
		XcmOperation::XtokensTransferBack,
		Some((20_000_000_000, BalanceOf::<T>::unique_saturated_from(10_000_000_000u128))),
	));

	assert_ok!(Slp::<T>::set_xcm_dest_weight_and_fee(
		origin.clone(),
		KSM,
		XcmOperation::TransferTo,
		Some((20_000_000_000, BalanceOf::<T>::unique_saturated_from(10_000_000_000u128))),
	));

	let mins_and_maxs = MinimumsMaximums {
		delegator_bonded_minimum: BalanceOf::<T>::unique_saturated_from(0u128),

		bond_extra_minimum: BalanceOf::<T>::unique_saturated_from(0u128),
		unbond_minimum: BalanceOf::<T>::unique_saturated_from(0u128),
		rebond_minimum: BalanceOf::<T>::unique_saturated_from(0u128),
		unbond_record_maximum: 1u32,
		validators_back_maximum: 100u32,
		delegator_active_staking_maximum: BalanceOf::<T>::unique_saturated_from(
			200_000_000_000_000_000_000u128,
		),
		validators_reward_maximum: 300u32,
		delegation_amount_minimum: BalanceOf::<T>::unique_saturated_from(0u128),
	};

	// Set minimums and maximums
	assert_ok!(Slp::<T>::set_minimums_and_maximums(origin.clone(), KSM, Some(mins_and_maxs)));

	// Set delegator ledger
	assert_ok!(Slp::<T>::add_validator(
		origin.clone(),
		KSM,
		Box::new(validator_0_location.clone()),
	));

	// initialize delegator
	Ok(())
}

benchmarks! {
	initialize_delegator {
		let origin = T::ControlOrigin::successful_origin();
		let call = Call::<T>::initialize_delegator {
			currency_id:KSM,
		};
	}: {call.dispatch_bypass_filter(origin)?}
	verify {
		assert_eq!(Slp::<T>::get_delegator_next_index(KSM),1);
	}

	bond {
		let origin = T::ControlOrigin::successful_origin();
		let who:T::AccountId = whitelisted_caller();
		let who: T::AccountId = whitelisted_caller();
		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who.clone()).unwrap();
		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();
		let validator_0_account_id_20: [u8; 20] =
			hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();
		let validator_0_location = MultiLocation {
			parents: 1,
			interior: X2(
				Parachain(2023),
				Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
			),
		};
		kusama_setup::<T>()?;
		assert_ok!(Slp::<T>::add_delegator(origin.clone(), KSM, 1u16,Box::new(subaccount_0_location.clone())));
		let call = Call::<T>::bond {
			currency_id:KSM,
			who:Box::new(subaccount_0_location.clone()),
			amount:BalanceOf::<T>::unique_saturated_from(5_000_000_000_000_000_000u128),
			validator:Some(validator_0_location.clone())
		};
  }: {call.dispatch_bypass_filter(RawOrigin::Signed(who.clone()).into())?}

	bond_extra {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who.clone()).unwrap();
		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();
		let validator_0_account_id_20: [u8; 20] =
			hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();
		let validator_0_location = MultiLocation {
			parents: 1,
			interior: X2(
				Parachain(2023),
				Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
			),
		};
		kusama_setup::<T>()?;
		assert_ok!(Slp::<T>::add_delegator(origin.clone(), KSM, 1u16,Box::new(subaccount_0_location.clone())));
		assert_ok!(Slp::<T>::bond(
			RawOrigin::Signed(who.clone()).into(),
			KSM,
			Box::new(subaccount_0_location.clone()),
			BalanceOf::<T>::unique_saturated_from(5_000_000_000_000_000_000u128),
			Some(validator_0_location.clone()),
		));
		let call = Call::<T>::bond_extra {
				currency_id:KSM,
				who:Box::new(subaccount_0_location.clone()),
				validator:Some(validator_0_location.clone()),
				amount:BalanceOf::<T>::unique_saturated_from(5_000_000_000_000_000_000u128),
			};
	  }: {call.dispatch_bypass_filter(RawOrigin::Signed(who.clone()).into())?}

rebond {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who.clone()).unwrap();
		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();
		let validator_0_account_id_20: [u8; 20] =
			hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();
		let validator_0_location = MultiLocation {
			parents: 1,
			interior: X2(
				Parachain(2023),
				Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
			),
		};
		kusama_setup::<T>()?;
		assert_ok!(Slp::<T>::add_delegator(origin.clone(), KSM, 1u16,Box::new(subaccount_0_location.clone())));
		assert_ok!(Slp::<T>::bond(
			RawOrigin::Signed(who.clone()).into(),
			KSM,
			Box::new(subaccount_0_location.clone()),
			BalanceOf::<T>::unique_saturated_from(5_000_000_000_000_000_000u128),
			Some(validator_0_location.clone()),
		));
	let call = Call::<T>::rebond {
			currency_id:KSM,
			who:Box::new(subaccount_0_location.clone()),
			validator:Some(validator_0_location.clone()),
			amount:Some(BalanceOf::<T>::unique_saturated_from(0u128)),
		};
  }: {call.dispatch_bypass_filter(RawOrigin::Signed(who.clone()).into())?}

delegate {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who.clone()).unwrap();
		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();
		let who2: T::AccountId = whitelisted_caller();
		let validator_0_account_id_20: [u8; 32] = Pallet::<T>::account_id_to_account_32(who2.clone()).unwrap();
		let validator_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(validator_0_account_id_20).unwrap();
		kusama_setup::<T>()?;
		assert_ok!(Slp::<T>::add_delegator(origin.clone(), KSM, 1u16,Box::new(subaccount_0_location.clone())));
		assert_ok!(Slp::<T>::add_validator(origin.clone(), KSM,Box::new(validator_0_location.clone())));
		assert_ok!(Slp::<T>::bond(
			RawOrigin::Signed(who.clone()).into(),
			KSM,
			Box::new(subaccount_0_location.clone()),
			BalanceOf::<T>::unique_saturated_from(5_000_000_000_000_000_000u128),
			Some(validator_0_location.clone()),
		));
	let call = Call::<T>::delegate {
			currency_id:KSM,
			who:Box::new(subaccount_0_location.clone()),
			targets:vec![validator_0_location.clone()],
		};
  }: {call.dispatch_bypass_filter(RawOrigin::Signed(who.clone()).into())?}

	redelegate {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who.clone()).unwrap();
		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();
		let who2: T::AccountId = whitelisted_caller();
		let validator_0_account_id_20: [u8; 32] = Pallet::<T>::account_id_to_account_32(who2.clone()).unwrap();
		let validator_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(validator_0_account_id_20).unwrap();
		kusama_setup::<T>()?;
		assert_ok!(Slp::<T>::add_delegator(origin.clone(), KSM, 1u16,Box::new(subaccount_0_location.clone())));
		assert_ok!(Slp::<T>::add_validator(origin.clone(), KSM,Box::new(validator_0_location.clone())));
		assert_ok!(Slp::<T>::bond(
			RawOrigin::Signed(who.clone()).into(),
			KSM,
			Box::new(subaccount_0_location.clone()),
			BalanceOf::<T>::unique_saturated_from(5_000_000_000_000_000_000u128),
			Some(validator_0_location.clone()),
		));
	let call = Call::<T>::redelegate {
			currency_id:KSM,
			who:Box::new(subaccount_0_location.clone()),
			targets:Some(vec![validator_0_location.clone()]),
		};
  }: {call.dispatch_bypass_filter(RawOrigin::Signed(who.clone()).into())?}

	payout {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who.clone()).unwrap();
		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();
		let who2: T::AccountId = whitelisted_caller();
		let validator_0_account_id_20: [u8; 32] = Pallet::<T>::account_id_to_account_32(who2.clone()).unwrap();
		let validator_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(validator_0_account_id_20).unwrap();
		kusama_setup::<T>()?;
		assert_ok!(Slp::<T>::add_delegator(origin.clone(), KSM, 1u16,Box::new(subaccount_0_location.clone())));
		assert_ok!(Slp::<T>::add_validator(origin.clone(), KSM,Box::new(validator_0_location.clone())));
	let call = Call::<T>::payout {
			currency_id:KSM,
			who:Box::new(subaccount_0_location.clone()),
			validator:Box::new(validator_0_location.clone()),
			when:Some(TimeUnit::Era(27))
		};
  }: {call.dispatch_bypass_filter(RawOrigin::Signed(who.clone()).into())?}

	liquidize {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who.clone()).unwrap();
		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();
		let who2: T::AccountId = whitelisted_caller();
		let validator_0_account_id_20: [u8; 32] = Pallet::<T>::account_id_to_account_32(who2.clone()).unwrap();
		let validator_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(validator_0_account_id_20).unwrap();
		kusama_setup::<T>()?;
		assert_ok!(Slp::<T>::add_delegator(origin.clone(), KSM, 1u16,Box::new(subaccount_0_location.clone())));
		assert_ok!(Slp::<T>::add_validator(origin.clone(), KSM,Box::new(validator_0_location.clone())));
		assert_ok!(Slp::<T>::bond(
			RawOrigin::Signed(who.clone()).into(),
			KSM,
			Box::new(subaccount_0_location.clone()),
			BalanceOf::<T>::unique_saturated_from(5_000_000_000_000_000_000u128),
			Some(validator_0_location.clone()),
		));
		<frame_system::Pallet<T>>::set_block_number(BlockNumberFor::<T>::from(101_000u32));
	let call = Call::<T>::liquidize {
			currency_id:KSM,
			who:Box::new(subaccount_0_location.clone()),
			when:Some(TimeUnit::SlashingSpan(5)),
			validator:None,
		};
  }: {call.dispatch_bypass_filter(RawOrigin::Signed(who.clone()).into())?}

		transfer_back {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();

		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who.clone()).unwrap();

		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();

				let exit_account_id_32: [u8; 32] =
			hex_literal::hex!["6d6f646c62662f76746f75740000000000000000000000000000000000000000"]
				.into();

		let exit_account_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 { network: Any, id: exit_account_id_32 }),
		};
			// set operate_origins
		assert_ok!(Slp::<T>::set_operate_origin(origin.clone(), KSM, Some(who.clone())));
		assert_ok!(Slp::<T>::add_delegator(origin.clone(), KSM, 1u16,Box::new(subaccount_0_location.clone())));
		// assert_ok!(Slp::<T>::add_validator(origin.clone(), KSM,Box::new(validator_0_location.clone())));
		assert_ok!(Slp::<T>::set_xcm_dest_weight_and_fee(
		origin.clone(),
		KSM,
		XcmOperation::TransferBack,
		Some((20_000_000_000, BalanceOf::<T>::unique_saturated_from(10_000_000_000u128))),
	));
	let call = Call::<T>::transfer_back {
			currency_id:KSM,
			from:Box::new(subaccount_0_location.clone()),
			to:Box::new(exit_account_location.clone()),
			amount: BalanceOf::<T>::unique_saturated_from(10_000_000_000u128)
		};
  }: {call.dispatch_bypass_filter(RawOrigin::Signed(who.clone()).into())?}

	increase_token_pool {
		let origin = T::ControlOrigin::successful_origin();
		let call = Call::<T>::increase_token_pool {
			currency_id:KSM,
			amount: BalanceOf::<T>::unique_saturated_from(10_000_000_000u128)
		};
  }: {call.dispatch_bypass_filter(origin)?}

	decrease_token_pool {
		let origin = T::ControlOrigin::successful_origin();
		assert_ok!(Slp::<T>::increase_token_pool(origin.clone(), KSM, BalanceOf::<T>::unique_saturated_from(10_000_000_000u128)));
		let call = Call::<T>::decrease_token_pool {
			currency_id:KSM,
			amount: BalanceOf::<T>::unique_saturated_from(10_000_000u128)
		};
  }: {call.dispatch_bypass_filter(origin)?}

	update_ongoing_time_unit {
		let origin = T::ControlOrigin::successful_origin();
		assert_ok!(Slp::<T>::set_ongoing_time_unit_update_interval(origin.clone(), KSM, Some(BlockNumberFor::<T>::from(0u32))));

		let call = Call::<T>::update_ongoing_time_unit {
			currency_id:KSM,
			time_unit: TimeUnit::Era(27)
		};
  }: {call.dispatch_bypass_filter(origin)?}

	refund_currency_due_unbond {
		let origin = T::ControlOrigin::successful_origin();
		assert_ok!(Slp::<T>::set_ongoing_time_unit_update_interval(origin.clone(), KSM, Some(BlockNumberFor::<T>::from(0u32))));

		let call = Call::<T>::refund_currency_due_unbond {
			currency_id:KSM,
		};
  }: {call.dispatch_bypass_filter(origin)?}

	charge_host_fee_and_tune_vtoken_exchange_rate {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who.clone()).unwrap();
		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();
		let who2: T::AccountId = whitelisted_caller();
		let validator_0_account_id_20: [u8; 32] = Pallet::<T>::account_id_to_account_32(who2.clone()).unwrap();
		let validator_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(validator_0_account_id_20).unwrap();
		kusama_setup::<T>()?;
		assert_ok!(Slp::<T>::add_delegator(origin.clone(), KSM, 1u16,Box::new(subaccount_0_location.clone())));
		assert_ok!(Slp::<T>::add_validator(origin.clone(), KSM,Box::new(validator_0_location.clone())));

		let pct_100 = Permill::from_percent(100);
		let treasury_32: [u8; 32] =
			hex_literal::hex!["6d6f646c62662f74727372790000000000000000000000000000000000000000"];
				let pct = Permill::from_percent(20);
		let treasury_location = MultiLocation {
			parents: 0,
			interior: X1(AccountId32 { network: Any, id: treasury_32 }),
		};
		assert_ok!(Slp::<T>::set_hosting_fees(origin.clone(), KSM, Some((pct, treasury_location))));
			assert_ok!(Slp::<T>::increase_token_pool(origin.clone(), KSM,BalanceOf::<T>::unique_saturated_from(10_000_000_000u128)));
		assert_ok!(Slp::<T>::set_currency_tune_exchange_rate_limit(origin.clone(), KSM,Some((1, pct_100))));
		assert_ok!(Slp::<T>::bond(
			RawOrigin::Signed(who.clone()).into(),
			KSM,
			Box::new(subaccount_0_location.clone()),
			BalanceOf::<T>::unique_saturated_from(5_000_000_000_000_000_000u128),
			Some(validator_0_location.clone()),
		));

		let call = Call::<T>::charge_host_fee_and_tune_vtoken_exchange_rate {
			currency_id:KSM,
			value:BalanceOf::<T>::unique_saturated_from(100u128),
			who:Some(subaccount_0_location.clone())
		};
  }: {call.dispatch_bypass_filter(RawOrigin::Signed(who.clone()).into())?}

	confirm_delegator_ledger_query_response {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who.clone()).unwrap();
		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();
		let who2: T::AccountId = whitelisted_caller();
		let validator_0_account_id_20: [u8; 32] = Pallet::<T>::account_id_to_account_32(who2.clone()).unwrap();
		let validator_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(validator_0_account_id_20).unwrap();
		kusama_setup::<T>()?;
		assert_ok!(Slp::<T>::add_delegator(origin.clone(), KSM, 1u16,Box::new(subaccount_0_location.clone())));
		assert_ok!(Slp::<T>::add_validator(origin.clone(), KSM,Box::new(validator_0_location.clone())));

		assert_ok!(Slp::<T>::bond(
		RawOrigin::Signed(who.clone()).into(),
		KSM,
		Box::new(subaccount_0_location.clone()),
		BalanceOf::<T>::unique_saturated_from(5_000_000_000_000_000_000u128),
		Some(validator_0_location.clone()),
	));
	let call = Call::<T>::confirm_delegator_ledger_query_response {
			currency_id:KSM,
			query_id:0,
		};
  }: {call.dispatch_bypass_filter(RawOrigin::Signed(who.clone()).into())?}


	fail_delegator_ledger_query_response {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who.clone()).unwrap();
		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();
		let who2: T::AccountId = whitelisted_caller();
		let validator_0_account_id_20: [u8; 32] = Pallet::<T>::account_id_to_account_32(who2.clone()).unwrap();
		let validator_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(validator_0_account_id_20).unwrap();
		kusama_setup::<T>()?;
		assert_ok!(Slp::<T>::add_delegator(origin.clone(), KSM, 1u16,Box::new(subaccount_0_location.clone())));
		assert_ok!(Slp::<T>::add_validator(origin.clone(), KSM,Box::new(validator_0_location.clone())));

		assert_ok!(Slp::<T>::bond(
		RawOrigin::Signed(who.clone()).into(),
		KSM,
		Box::new(subaccount_0_location.clone()),
		BalanceOf::<T>::unique_saturated_from(5_000_000_000_000_000_000u128),
		Some(validator_0_location.clone()),
	));
	let call = Call::<T>::fail_delegator_ledger_query_response {
			currency_id:KSM,
			query_id:0,
		};
  }: {call.dispatch_bypass_filter(RawOrigin::Signed(who.clone()).into())?}

	set_xcm_dest_weight_and_fee {
		let origin = T::ControlOrigin::successful_origin();
		let call = Call::<T>::set_xcm_dest_weight_and_fee {
			currency_id:KSM,
			operation:XcmOperation::Bond,
			weight_and_fee:Some((5_000_000_000, BalanceOf::<T>::unique_saturated_from(5_000_000_000u128)))
		};
  }: {call.dispatch_bypass_filter(origin)?}
	verify {
		assert_eq!(Slp::<T>::xcm_dest_weight_and_fee(KSM,XcmOperation::Bond),Some((5_000_000_000,BalanceOf::<T>::unique_saturated_from(5_000_000_000u128) )));
	}

	set_operate_origin {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let call = Call::<T>::set_operate_origin {
			currency_id:KSM,
			who:Some(who.clone())
		};
	}: {call.dispatch_bypass_filter(origin)?}
	verify {
		assert_eq!(Slp::<T>::get_operate_origin(KSM),Some(who.clone()));
	}

	set_fee_source {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let who_32 = Pallet::<T>::account_id_to_account_32(who).unwrap();
		let who_location = Pallet::<T>::account_32_to_local_location(who_32).unwrap();
		let call = Call::<T>::set_fee_source {
			currency_id:KSM,
			who_and_fee:Some((who_location.clone(),BalanceOf::<T>::unique_saturated_from(5_000_000_000u128)))
		};
	}: {call.dispatch_bypass_filter(origin)?}
	verify {
		assert_eq!(Slp::<T>::get_fee_source(KSM),Some((who_location.clone(),BalanceOf::<T>::unique_saturated_from(5_000_000_000u128))));
	}

	add_delegator {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who).unwrap();

		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();

		let call = Call::<T>::add_delegator {
			currency_id:KSM,
			index:0u16,
			who:Box::new(subaccount_0_location.clone()),
		};
	}: {call.dispatch_bypass_filter(origin)?}

	remove_delegator {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who.clone()).unwrap();
		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();
		let who2: T::AccountId = whitelisted_caller();
		let validator_0_account_id_20: [u8; 32] = Pallet::<T>::account_id_to_account_32(who2.clone()).unwrap();
		let validator_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(validator_0_account_id_20).unwrap();
		kusama_setup::<T>()?;
		assert_ok!(Slp::<T>::add_delegator(origin.clone(), KSM, 1u16,Box::new(subaccount_0_location.clone())));
		assert_ok!(Slp::<T>::add_validator(origin.clone(), KSM,Box::new(validator_0_location.clone())));

		assert_ok!(Slp::<T>::bond(
		RawOrigin::Signed(who.clone()).into(),
		KSM,
		Box::new(subaccount_0_location.clone()),
		BalanceOf::<T>::unique_saturated_from(5_000_000_000_000_000_000u128),
		Some(validator_0_location.clone()),
	));

		let call = Call::<T>::remove_delegator {
			currency_id:KSM,
			who:Box::new(subaccount_0_location.clone()),
		};
	}: {call.dispatch_bypass_filter(origin)?}

	set_validators_by_delegator {
				let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who.clone()).unwrap();
		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();
		let who2: T::AccountId = whitelisted_caller();
		let validator_0_account_id_20: [u8; 32] = Pallet::<T>::account_id_to_account_32(who2.clone()).unwrap();
		let validator_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(validator_0_account_id_20).unwrap();
		kusama_setup::<T>()?;
		assert_ok!(Slp::<T>::add_delegator(origin.clone(), KSM, 1u16,Box::new(subaccount_0_location.clone())));
		assert_ok!(Slp::<T>::add_validator(origin.clone(), KSM,Box::new(validator_0_location.clone())));

		assert_ok!(Slp::<T>::bond(
		RawOrigin::Signed(who.clone()).into(),
		KSM,
		Box::new(subaccount_0_location.clone()),
		BalanceOf::<T>::unique_saturated_from(5_000_000_000_000_000_000u128),
		Some(validator_0_location.clone()),
	));
		let call = Call::<T>::set_validators_by_delegator {
			currency_id:KSM,
			who:Box::new(subaccount_0_location.clone()),
			validators:vec![validator_0_location.clone()]
		};
	}: {call.dispatch_bypass_filter(origin)?}

	set_delegator_ledger {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who).unwrap();

		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();
		kusama_setup::<T>()?;
		let sb_ledger = SubstrateLedger {
			account: subaccount_0_location.clone(),
			total: BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128),
			active: BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128),
			unlocking: vec![],
		};
		let ledger = Ledger::Substrate(sb_ledger);

		let call = Call::<T>::set_delegator_ledger {
			currency_id:KSM,
			who:Box::new(subaccount_0_location.clone()),
			ledger:Box::new(Some(ledger))
		};
	}: {call.dispatch_bypass_filter(origin)?}

	add_validator {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who).unwrap();

		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();

		let call = Call::<T>::add_validator {
			currency_id:KSM,
			who:Box::new(subaccount_0_location.clone()),
		};
	}: {call.dispatch_bypass_filter(origin)?}

	remove_validator {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who).unwrap();

		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();

		assert_ok!(Slp::<T>::add_validator(origin.clone(),KSM,Box::new(subaccount_0_location.clone())));

		let call = Call::<T>::remove_validator {
			currency_id:KSM,
			who:Box::new(subaccount_0_location.clone()),
		};
	}: {call.dispatch_bypass_filter(origin)?}

	set_minimums_and_maximums {
		let origin = T::ControlOrigin::successful_origin();

		let mins_and_maxs = MinimumsMaximums {
		delegator_bonded_minimum: BalanceOf::<T>::unique_saturated_from(100_000_000_000u128),

		bond_extra_minimum: BalanceOf::<T>::unique_saturated_from(100_000_000_000u128),
		unbond_minimum: BalanceOf::<T>::unique_saturated_from(100_000_000_000u128),
		rebond_minimum: BalanceOf::<T>::unique_saturated_from(100_000_000_000u128),
		unbond_record_maximum: 1u32,
		validators_back_maximum: 100u32,
		delegator_active_staking_maximum: BalanceOf::<T>::unique_saturated_from(200_000_000_000_000_000_000u128),
		validators_reward_maximum: 300u32,
		delegation_amount_minimum: BalanceOf::<T>::unique_saturated_from(500_000_000u128),
	};
		let call = Call::<T>::set_minimums_and_maximums {
			currency_id:KSM,
			constraints:Some(mins_and_maxs)
		};
	}: {call.dispatch_bypass_filter(origin)?}

	set_currency_delays {
		let origin = T::ControlOrigin::successful_origin();
		let delay =
			Delays { unlock_delay: TimeUnit::Round(24), leave_delegators_delay: TimeUnit::Round(24) };

		let call = Call::<T>::set_currency_delays {
			currency_id:KSM,
			maybe_delays:Some(delay)
		};
	}: {call.dispatch_bypass_filter(origin)?}

	set_hosting_fees {
		let origin = T::ControlOrigin::successful_origin();
		let treasury_32: [u8; 32] =
			hex_literal::hex!["6d6f646c62662f74727372790000000000000000000000000000000000000000"];
				let pct = Permill::from_percent(20);
		let treasury_location = MultiLocation {
			parents: 0,
			interior: X1(AccountId32 { network: Any, id: treasury_32 }),
		};
		let call = Call::<T>::set_hosting_fees {
			currency_id:KSM,
			maybe_fee_set:Some((pct, treasury_location))
		};
	}: {call.dispatch_bypass_filter(origin)?}

	set_currency_tune_exchange_rate_limit {
		let origin = T::ControlOrigin::successful_origin();
		let pct_100 = Permill::from_percent(100);
		let call = Call::<T>::set_currency_tune_exchange_rate_limit {
			currency_id:KSM,
			maybe_tune_exchange_rate_limit:Some((1, pct_100))
		};
	}: {call.dispatch_bypass_filter(origin)?}

	set_ongoing_time_unit_update_interval {
		let origin = T::ControlOrigin::successful_origin();

		let call = Call::<T>::set_ongoing_time_unit_update_interval {
			currency_id:KSM,
			maybe_interval:Some(BlockNumberFor::<T>::from(100u32))
		};
	}: {call.dispatch_bypass_filter(origin)?}

	verify {
		assert_eq!(Slp::<T>::get_ongoing_time_unit_update_interval(KSM),Some(BlockNumberFor::<T>::from(100u32)));
	}

	add_supplement_fee_account_to_whitelist {
		let origin = T::ControlOrigin::successful_origin();
		let exit_account_id_32: [u8; 32] =
			hex_literal::hex!["6d6f646c62662f76746f75740000000000000000000000000000000000000000"]
				.into();

		let exit_account_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 { network: Any, id: exit_account_id_32 }),
		};
		let call = Call::<T>::add_supplement_fee_account_to_whitelist {
			currency_id:KSM,
			who:Box::new(exit_account_location.clone()),
		};
	}: {call.dispatch_bypass_filter(origin)?}
	remove_supplement_fee_account_from_whitelist {
		let origin = T::ControlOrigin::successful_origin();
		let exit_account_id_32: [u8; 32] =
			hex_literal::hex!["6d6f646c62662f76746f75740000000000000000000000000000000000000000"]
				.into();

		let exit_account_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 { network: Any, id: exit_account_id_32 }),
		};
		 assert_ok!(Slp::<T>::add_supplement_fee_account_to_whitelist(origin.clone(), KSM, Box::new(exit_account_location.clone())));
		let call = Call::<T>::remove_supplement_fee_account_from_whitelist {
			currency_id:KSM,
			who:Box::new(exit_account_location.clone()),
		};
	}: {call.dispatch_bypass_filter(origin)?}

	unbond {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who.clone()).unwrap();
		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();
		let validator_0_account_id_20: [u8; 20] =
			hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();
		let validator_0_location = MultiLocation {
			parents: 1,
			interior: X2(
				Parachain(2023),
				Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
			),
		};
		kusama_setup::<T>()?;
		assert_ok!(Slp::<T>::add_delegator(origin.clone(), KSM, 1u16,Box::new(subaccount_0_location.clone())));
		assert_ok!(Slp::<T>::bond(
			RawOrigin::Signed(who.clone()).into(),
			KSM,
			Box::new(subaccount_0_location.clone()),
			BalanceOf::<T>::unique_saturated_from(5_000_000_000_000_000_000u128),
			Some(validator_0_location.clone()),
		));

		let call = Call::<T>::unbond {
				currency_id:KSM,
				who:Box::new(subaccount_0_location.clone()),
				validator:None,
				amount:BalanceOf::<T>::unique_saturated_from(0u128),
			};
	  }: {call.dispatch_bypass_filter(RawOrigin::Signed(who.clone()).into())?}

	unbond_all {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who.clone()).unwrap();
		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();
		let validator_0_account_id_20: [u8; 20] =
			hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();
		let validator_0_location = MultiLocation {
			parents: 1,
			interior: X2(
				Parachain(2023),
				Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
			),
		};
		kusama_setup::<T>()?;
		assert_ok!(Slp::<T>::add_delegator(origin.clone(), KSM, 1u16,Box::new(subaccount_0_location.clone())));
		assert_ok!(Slp::<T>::bond(
			RawOrigin::Signed(who.clone()).into(),
			KSM,
			Box::new(subaccount_0_location.clone()),
			BalanceOf::<T>::unique_saturated_from(5_000_000_000_000_000_000u128),
			Some(validator_0_location.clone()),
		));

		let call = Call::<T>::unbond_all {
				currency_id:KSM,
				who:Box::new(subaccount_0_location.clone()),
			};
	  }: {call.dispatch_bypass_filter(RawOrigin::Signed(who.clone()).into())?}

	undelegate {
		let origin = T::ControlOrigin::successful_origin();
		let who: T::AccountId = whitelisted_caller();
		let subaccount_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(who.clone()).unwrap();
		let subaccount_0_location: MultiLocation =
			Pallet::<T>::account_32_to_parent_location(subaccount_0_32).unwrap();

let validator_0_account: T::AccountId = account("validator0",0,1);
	let validator_0_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(validator_0_account.clone()).unwrap();
	let validator_0_location: MultiLocation =
		Pallet::<T>::account_32_to_parent_location(validator_0_32).unwrap();

	let validator_1_account: T::AccountId = account("validator1",0,1);
	let validator_1_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(validator_1_account.clone()).unwrap();
	let validator_1_location: MultiLocation =
		Pallet::<T>::account_32_to_parent_location(validator_1_32).unwrap();

		kusama_setup::<T>()?;

		assert_ok!(Slp::<T>::add_delegator(origin.clone(), KSM, 1u16,Box::new(subaccount_0_location.clone())));
		assert_ok!(Slp::<T>::add_validator(origin.clone(), KSM,Box::new(validator_0_location.clone())));
		assert_ok!(Slp::<T>::add_validator(origin.clone(), KSM,Box::new(validator_1_location.clone())));

		let sb_ledger = SubstrateLedger {
			account: subaccount_0_location.clone(),
			total: BalanceOf::<T>::unique_saturated_from(1000_000_000_000u128),
			active: BalanceOf::<T>::unique_saturated_from(500_000_000_000u128),
			unlocking: vec![],
		};
		let ledger = Ledger::Substrate(sb_ledger);
		DelegatorLedgers::<T>::insert(KSM, subaccount_0_location.clone(), ledger);

		assert_ok!(Slp::<T>::set_validators_by_delegator (
			origin.clone(),
			KSM,
			Box::new(subaccount_0_location.clone()),
			vec![validator_0_location.clone(),validator_1_location.clone()]
		));

		let call = Call::<T>::undelegate {
			currency_id:KSM,
			who:Box::new(subaccount_0_location.clone()),
			targets:vec![validator_0_location.clone()],
		};
	  }: {call.dispatch_bypass_filter(RawOrigin::Signed(who.clone()).into())?}
}
// Todo:
// 	fn chill() -> Weight;
// 	fn transfer_to() -> Weight;
// 	fn supplement_fee_reserve() -> Weight;
// 	fn confirm_validators_by_delegator_query_response() -> Weight;
// 	fn fail_validators_by_delegator_query_response() -> Weight;
