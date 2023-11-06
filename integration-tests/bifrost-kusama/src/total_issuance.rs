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

use crate::{
	kusama_integration_tests::{ALICE, BOB},
	kusama_test_net::Bifrost,
};
use bifrost_kusama_runtime::{
	constants::currency::MILLIBNC, Balances, Runtime, RuntimeOrigin, System,
};
use bifrost_slp::BalanceOf;
use frame_support::{assert_ok, traits::Currency};
use sp_runtime::{traits::AccountIdConversion, AccountId32, SaturatedConversion};
use xcm_emulator::TestExt;

#[test]
fn dollar_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		Bifrost::execute_with(|| {
			let treasury_account: AccountId32 =
				bifrost_kusama_runtime::TreasuryPalletId::get().into_account_truncating();
			assert_eq!(
				BalanceOf::<Runtime>::saturated_from(10 * MILLIBNC),
				Balances::minimum_balance()
			);

			assert_ok!(Balances::force_set_balance(
				RuntimeOrigin::root(),
				sp_runtime::MultiAddress::Id(ALICE.into()),
				BalanceOf::<Runtime>::saturated_from(20 * MILLIBNC)
			));
			assert_ok!(Balances::force_set_balance(
				RuntimeOrigin::root(),
				sp_runtime::MultiAddress::Id(BOB.into()),
				BalanceOf::<Runtime>::saturated_from(20 * MILLIBNC)
			));
			assert_ok!(Balances::force_set_balance(
				RuntimeOrigin::root(),
				treasury_account.clone().into(),
				BalanceOf::<Runtime>::saturated_from(20 * MILLIBNC)
			));

			assert_eq!(
				Balances::total_issuance(),
				BalanceOf::<Runtime>::saturated_from(70000000000u128)
			);

			System::reset_events();

			assert_ok!(Balances::transfer_allow_death(
				RuntimeOrigin::signed(ALICE.into()),
				sp_runtime::MultiAddress::Id(BOB.into()),
				BalanceOf::<Runtime>::saturated_from(20 * MILLIBNC - 1)
			));

			println!("{:?}", System::events());

			// As expected dust balance is removed.
			assert_eq!(Balances::free_balance(&ALICE.into()), 0);
			assert_eq!(Balances::free_balance(&treasury_account.into()), 20 * MILLIBNC + 1);

			assert_eq!(
				Balances::total_issuance(),
				BalanceOf::<Runtime>::saturated_from(70000000000u128)
			);
		});
	});
}
