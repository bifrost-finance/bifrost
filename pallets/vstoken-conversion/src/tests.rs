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
fn vsksm_convert_to_vsbond() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		const EXCHANGE_FEE: VstokenConversionExchangeFee<BalanceOf<Runtime>> =
			VstokenConversionExchangeFee {
				vstoken_exchange_fee: 10,
				vsbond_exchange_fee_of_vstoken: 10,
			};
		assert_ok!(VstokenConversion::set_exchange_fee(RuntimeOrigin::root(), EXCHANGE_FEE));
		pub const EXCHANGE_RATE_PERCENTAGE: Percent = Percent::from_percent(5);
		const EXCHANGE_RATE: VstokenConversionExchangeRate = VstokenConversionExchangeRate {
			vsbond_convert_to_vstoken: EXCHANGE_RATE_PERCENTAGE,
			// vsbond_convert_to_vsksm: EXCHANGE_RATE_PERCENTAGE,
			vstoken_convert_to_vsbond: EXCHANGE_RATE_PERCENTAGE,
			// vsdot_convert_to_vsbond: EXCHANGE_RATE_PERCENTAGE,
		};
		assert_ok!(VstokenConversion::set_relaychain_lease(RuntimeOrigin::signed(ALICE), 1));
		assert_noop!(
			VstokenConversion::vstoken_convert_to_vsbond(Some(BOB).into(), vsBond, 1000, 1),
			Error::<Runtime>::NotEnoughBalance
		);
		assert_noop!(
			VstokenConversion::vstoken_convert_to_vsbond(Some(BOB).into(), vsBond, 100, 1),
			Error::<Runtime>::CalculationOverflow
		);
		assert_ok!(VstokenConversion::set_exchange_rate(
			RuntimeOrigin::signed(ALICE),
			8,
			EXCHANGE_RATE
		));
		assert_eq!(VstokenConversion::exchange_rate(8), EXCHANGE_RATE);
		assert_noop!(
			VstokenConversion::vstoken_convert_to_vsbond(Some(BOB).into(), vsBond, 100, 1),
			orml_tokens::Error::<Runtime>::BalanceTooLow
		);
		assert_noop!(
			VstokenConversion::vstoken_convert_to_vsbond(Some(BOB).into(), KSM, 100, 1),
			Error::<Runtime>::NotSupportTokenType
		);
		let vsbond_account: AccountId =
			<Runtime as Config>::VsbondAccount::get().into_account_truncating();
		assert_ok!(<Tokens as MultiCurrency<AccountId>>::deposit(vsBond, &vsbond_account, 10000));
		assert_ok!(VstokenConversion::vstoken_convert_to_vsbond(Some(BOB).into(), vsBond, 100, 1));
		assert_eq!(Tokens::free_balance(vsKSM, &BOB), 0);
		assert_eq!(Tokens::free_balance(vsBond, &vsbond_account), 8200);
		assert_eq!(Tokens::free_balance(vsBond, &BOB), 1900);

		assert_ok!(<Tokens as MultiCurrency<AccountId>>::deposit(vsKSM, &BOB, 1000));
		pub const EXCHANGE_RATE_PERCENTAGE_0: Percent = Percent::from_percent(100);
		const EXCHANGE_RATE_0: VstokenConversionExchangeRate = VstokenConversionExchangeRate {
			vsbond_convert_to_vstoken: EXCHANGE_RATE_PERCENTAGE_0,
			vstoken_convert_to_vsbond: EXCHANGE_RATE_PERCENTAGE_0,
		};
		assert_ok!(VstokenConversion::set_relaychain_lease(RuntimeOrigin::signed(ALICE), 11));
		assert_ok!(VstokenConversion::set_exchange_rate(
			RuntimeOrigin::signed(ALICE),
			-2,
			EXCHANGE_RATE_0
		));
		assert_ok!(VstokenConversion::vstoken_convert_to_vsbond(Some(BOB).into(), vsBond, 100, 1));
		assert_eq!(Tokens::free_balance(vsKSM, &BOB), 900);
		assert_eq!(Tokens::free_balance(vsBond, &vsbond_account), 8110);
		assert_eq!(Tokens::free_balance(vsBond, &BOB), 1990);
	});
}

#[test]
fn vsbond_convert_to_vsksm() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		const EXCHANGE_FEE: VstokenConversionExchangeFee<BalanceOf<Runtime>> =
			VstokenConversionExchangeFee {
				vstoken_exchange_fee: 10,
				vsbond_exchange_fee_of_vstoken: 10,
			};
		assert_ok!(VstokenConversion::set_exchange_fee(RuntimeOrigin::root(), EXCHANGE_FEE));
		const EXCHANGE_RATE_PERCENTAGE: Percent = Percent::from_percent(5);
		const EXCHANGE_RATE: VstokenConversionExchangeRate = VstokenConversionExchangeRate {
			vsbond_convert_to_vstoken: EXCHANGE_RATE_PERCENTAGE,
			vstoken_convert_to_vsbond: EXCHANGE_RATE_PERCENTAGE,
		};
		assert_ok!(VstokenConversion::set_relaychain_lease(RuntimeOrigin::signed(ALICE), 1));
		assert_ok!(VstokenConversion::set_exchange_rate(
			RuntimeOrigin::signed(ALICE),
			8,
			EXCHANGE_RATE
		));
		assert_eq!(VstokenConversion::exchange_rate(8), EXCHANGE_RATE);
		let vsbond_account: AccountId =
			<Runtime as Config>::VsbondAccount::get().into_account_truncating();
		assert_ok!(VstokenConversion::vsbond_convert_to_vstoken(Some(BOB).into(), vsBond, 100, 1));
		assert_eq!(Tokens::free_balance(vsKSM, &BOB), 104);
		assert_eq!(Tokens::free_balance(vsBond, &vsbond_account), 100);
		assert_eq!(Tokens::free_balance(vsBond, &BOB), 0);
	});
}
