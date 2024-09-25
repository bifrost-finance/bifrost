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
	mock::{new_test_ext, RuntimeOrigin, Test},
	Pallet as XcmInterface, XcmWeightAndFee,
};
use bifrost_primitives::{XcmOperationType, BNC};
use frame_support::assert_ok;
use xcm::v4::Weight;

#[test]
fn update_xcm_dest_weight_and_fee() {
	new_test_ext().execute_with(|| {
		let updates = vec![
			(BNC, XcmOperationType::Bond, Weight::zero(), 0u128),
			(BNC, XcmOperationType::Bond, Weight::zero(), 0u128),
			(BNC, XcmOperationType::Bond, Weight::zero(), 0u128),
			(BNC, XcmOperationType::Bond, Weight::zero(), 0u128),
			(BNC, XcmOperationType::Bond, Weight::zero(), 0u128),
		];

		assert_ok!(XcmInterface::<Test>::update_xcm_dest_weight_and_fee(
			RuntimeOrigin::root(),
			updates
		));

		assert_eq!(
			XcmWeightAndFee::<Test>::get(BNC, XcmOperationType::Bond),
			Some((Weight::zero(), 0u128))
		);
	})
}
