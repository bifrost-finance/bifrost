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

#![cfg(test)]

use crate::{mock::*, BNC, FIL, KSM, *};
use frame_support::{assert_noop, assert_ok};
use orml_traits::MultiCurrency;
use sp_runtime::WeakBoundedVec;
use xcm::opaque::latest::NetworkId::Any;

fn mins_maxs_setup() {
	let mins_and_maxs = MinimumsMaximums {
		delegator_bonded_minimum: 100_000_000_000,
		bond_extra_minimum: 0,
		unbond_minimum: 0,
		rebond_minimum: 0,
		unbond_record_maximum: 32,
		validators_back_maximum: 36,
		delegator_active_staking_maximum: 200_000_000_000_000,
		validators_reward_maximum: 0,
		delegation_amount_minimum: 0,
		delegators_maximum: 100,
		validators_maximum: 300,
	};

	// Set minimums and maximums
	MinimumsAndMaximums::<Runtime>::insert(FIL, mins_and_maxs);
}

#[test]
fn initialize_delegator_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let location = MultiLocation {
			parents: 100,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};

		System::set_block_number(1);

		mins_maxs_setup();
		assert_ok!(Slp::initialize_delegator(
			Origin::signed(ALICE),
			FIL,
			Some(Box::new(location.clone()))
		));

		assert_eq!(DelegatorNextIndex::<Runtime>::get(FIL), 1);
		assert_eq!(DelegatorsIndex2Multilocation::<Runtime>::get(FIL, 0), Some(location.clone()));
		assert_eq!(DelegatorsMultilocation2Index::<Runtime>::get(FIL, location), Some(0));
	});
}
