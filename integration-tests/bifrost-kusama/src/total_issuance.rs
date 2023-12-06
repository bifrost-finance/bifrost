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

use bifrost_kusama_runtime::{
	constants::currency::{BNCS, MILLIBNC},
	Balances, RuntimeOrigin,
};
use frame_support::{assert_ok, traits::Currency};
use integration_tests_common::{
	BifrostKusama, BifrostKusamaAlice, BifrostKusamaBob, BifrostKusamaTreasury,
};
use xcm_emulator::TestExt;

#[test]
fn remove_dust_account_should_work() {
	BifrostKusama::execute_with(|| {
		assert_eq!(Balances::minimum_balance(), 10 * MILLIBNC);

		assert_eq!(Balances::total_issuance(), 8_000_0000 * BNCS);

		assert_ok!(Balances::transfer_allow_death(
			RuntimeOrigin::signed(BifrostKusamaAlice::get()),
			BifrostKusamaBob::get().into(),
			Balances::free_balance(&BifrostKusamaAlice::get()) - MILLIBNC
		));

		// As expected dust balance is removed.
		assert_eq!(Balances::free_balance(&BifrostKusamaAlice::get()), 0);
		assert_eq!(
			Balances::free_balance(&BifrostKusamaTreasury::get()),
			1_000_0000 * BNCS + MILLIBNC
		);

		assert_eq!(Balances::total_issuance(), 8_000_0000 * BNCS);
	});
}
