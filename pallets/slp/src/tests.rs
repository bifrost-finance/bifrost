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

use frame_support::assert_ok;
use mock::{Event, *};
use orml_traits::MultiCurrency;

use super::*;
use crate::KSM;

#[test]
fn set_xcm_dest_weight_and_fee_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// Insert a new record.
		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			Origin::signed(ALICE),
			KSM,
			XcmOperation::Bond,
			Some((5_000_000_000, 5_000_000_000))
		));

		assert_eq!(
			XcmDestWeightAndFee::<Runtime>::get(KSM, XcmOperation::Bond),
			Some((5_000_000_000, 5_000_000_000))
		);

		// Delete a record.
		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			Origin::signed(ALICE),
			KSM,
			XcmOperation::Bond,
			None
		));

		assert_eq!(XcmDestWeightAndFee::<Runtime>::get(KSM, XcmOperation::Bond), None);
	});
}

#[test]
fn construct_xcm_and_send_as_subaccount_should_work() {
	let para_chain_account: AccountId =
		hex_literal::hex!["70617261d1070000000000000000000000000000000000000000000000000000"]
			.into();

	let sub_account_id = SubAccountIndexMultiLocationConvertor::derivative_account_id(
		para_chain_account.clone(),
		0u16,
	);

	// parachain_account 2001: 5Ec4AhPV91i9yNuiWuNunPf6AQCYDhFTTA4G5QCbtqYApH9E
	// hex: 70617261d1070000000000000000000000000000000000000000000000000000
	println!("para_string: {:?}", para_chain_account);
	// sub_account index:0(sub_account_id.to_string()))
	// 5E78xTBiaN3nAGYtcNnqTJQJqYAkSDGggKqaDfpNsKyPpbcb
	// hex: 5a53736d8e96f1c007cf0d630acf5209b20611617af23ce924c8e25328eb5d28
	println!("sub_string: {:?}", sub_account_id);
}
