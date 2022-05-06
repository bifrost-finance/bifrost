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

#![cfg(test)]

use frame_support::{assert_noop, assert_ok};
pub use primitives::{VstokenConversionExchangeFee, VstokenConversionExchangeRate};
use sp_arithmetic::per_things::Percent;

use crate::{mock::*, *};

#[test]
fn claim() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let mut tokens = BTreeMap::<CurrencyIdOf<Runtime>, BalanceOf<Runtime>>::new();
		tokens.entry(KSM).or_insert(1000);
		let mut basic_reward =
			BTreeMap::<CurrencyIdOf<Runtime>, (BalanceOf<Runtime>, BalanceOf<Runtime>)>::new();
		let _ = basic_reward.entry(KSM).or_insert((1000, 0));

		assert_ok!(Farming::create_farming_pool(
			Origin::signed(ALICE),
			tokens.clone(),
			basic_reward.clone(),
			Some(KSM)
		));

		let pid = 0;
		assert_ok!(Farming::charge(Origin::signed(BOB), pid));
		let keeper = <Runtime as Config>::PalletId::get().into_sub_account(pid);
		let pool_info = PoolInfo::reset(
			keeper,
			tokens.clone(),
			basic_reward.clone(),
			PoolState::Charged,
			Some(0),
		);

		assert_eq!(Farming::pool_infos(pid), pool_info);

		assert_ok!(Farming::deposit(Origin::signed(ALICE), pid, tokens.clone(), None));
		// assert_eq!(Farming::shares_and_withdrawn_rewards(pid, ALICE), (0, tokens));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 0);
		assert_ok!(Farming::claim(Origin::signed(ALICE), pid));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 1000);
	});
}
