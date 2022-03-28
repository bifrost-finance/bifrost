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
use sp_arithmetic::per_things::Percent;

use crate::{mock::*, *};

#[test]
fn vsksm_convert_to_vsbond() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_ok!(VstokenConversion::set_exchange_fee(Origin::root(), KSM, 10, 10));
		pub const EXCHANGE_RATE_PERCENTAGE: Percent = Percent::from_percent(5);

		assert_ok!(VstokenConversion::set_exchange_rate(
			Origin::root(),
			21,
			(EXCHANGE_RATE_PERCENTAGE, EXCHANGE_RATE_PERCENTAGE)
		));
		assert_eq!(
			VstokenConversion::exchange_rate(21),
			(EXCHANGE_RATE_PERCENTAGE, EXCHANGE_RATE_PERCENTAGE)
		);
		assert_noop!(
			VstokenConversion::vsksm_convert_to_vsbond(Some(BOB).into(), vsBond, 1000, 1),
			Error::<Runtime>::NotEnoughBalance
		);
		assert_noop!(
			VstokenConversion::vsksm_convert_to_vsbond(Some(BOB).into(), vsBond, 100, 1),
			orml_tokens::Error::<Runtime>::BalanceTooLow
		);
		assert_noop!(
			VstokenConversion::vsksm_convert_to_vsbond(Some(BOB).into(), KSM, 100, 1),
			Error::<Runtime>::NotSupportTokenType
		);
		let vsbond_account: AccountId = <Runtime as Config>::VsbondAccount::get().into_account();
		assert_ok!(<Tokens as MultiCurrency<AccountId>>::deposit(vsBond, &vsbond_account, 100));
		assert_ok!(VstokenConversion::vsksm_convert_to_vsbond(Some(BOB).into(), vsBond, 100, 1));
		assert_eq!(Tokens::free_balance(vsKSM, &BOB), 0);
		assert_eq!(Tokens::free_balance(vsBond, &vsbond_account), 96);
		assert_eq!(Tokens::free_balance(vsBond, &BOB), 104);
	});
}

#[test]
fn vsbond_convert_to_vsksm() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_ok!(VstokenConversion::set_exchange_fee(Origin::root(), KSM, 10, 10));
		const EXCHANGE_RATE_PERCENTAGE: Percent = Percent::from_percent(5);

		assert_ok!(VstokenConversion::set_exchange_rate(
			Origin::root(),
			21,
			(EXCHANGE_RATE_PERCENTAGE, EXCHANGE_RATE_PERCENTAGE)
		));
		assert_eq!(
			VstokenConversion::exchange_rate(21),
			(EXCHANGE_RATE_PERCENTAGE, EXCHANGE_RATE_PERCENTAGE)
		);
		let vsbond_account: AccountId = <Runtime as Config>::VsbondAccount::get().into_account();
		assert_ok!(VstokenConversion::vsbond_convert_to_vsksm(Some(BOB).into(), vsBond, 100, 1));
		assert_eq!(Tokens::free_balance(vsKSM, &BOB), 104);
		assert_eq!(Tokens::free_balance(vsBond, &vsbond_account), 100);
		assert_eq!(Tokens::free_balance(vsBond, &BOB), 0);
	});
}
