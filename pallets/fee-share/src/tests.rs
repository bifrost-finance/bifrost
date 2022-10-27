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

use crate::{mock::*, *};
use frame_support::assert_ok;
use sp_arithmetic::per_things::Perbill;

#[test]
fn on_idle() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let tokens_proportion = vec![(ALICE, Perbill::from_percent(100))];

		assert_ok!(FeeShare::create_distribution(
			Origin::signed(ALICE),
			vec![KSM],
			tokens_proportion,
			true,
		));
		let keeper: AccountId =
			<Runtime as Config>::FeeSharePalletId::get().into_sub_account_truncating(0);

		assert_ok!(FeeShare::set_era_length(Origin::signed(ALICE), 1));
		FeeShare::on_idle(<frame_system::Pallet<Runtime>>::block_number() + 1, 0);
		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(KSM, &ALICE, &keeper, 100,));
		FeeShare::on_idle(<frame_system::Pallet<Runtime>>::block_number() + 2, 0);
		assert_eq!(Tokens::free_balance(KSM, &keeper), 0);
	});
}

#[test]
fn edit_delete_distribution() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let tokens_proportion = vec![(ALICE, Perbill::from_percent(100))];

		assert_ok!(FeeShare::create_distribution(
			Origin::signed(ALICE),
			vec![KSM],
			tokens_proportion.clone(),
			true,
		));
		assert_ok!(FeeShare::edit_distribution(
			Origin::signed(ALICE),
			0,
			None, // Some(vec![KSM]),
			Some(tokens_proportion),
			Some(false),
		));
		let keeper: AccountId =
			<Runtime as Config>::FeeSharePalletId::get().into_sub_account_truncating(0);

		assert_ok!(FeeShare::set_era_length(Origin::signed(ALICE), 1));
		FeeShare::on_idle(<frame_system::Pallet<Runtime>>::block_number() + 1, 0);
		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(KSM, &ALICE, &keeper, 100,));
		FeeShare::on_idle(<frame_system::Pallet<Runtime>>::block_number() + 2, 0);
		assert_eq!(Tokens::free_balance(KSM, &keeper), 10100);
		assert_ok!(FeeShare::execute_distribute(Origin::signed(ALICE), 0));
		assert_eq!(Tokens::free_balance(KSM, &keeper), 0);

		if let Some(infos) = FeeShare::distribution_infos(0) {
			assert_eq!(infos.token_type, vec![KSM])
		}
		assert_ok!(FeeShare::delete_distribution(Origin::signed(ALICE), 0));
		assert_eq!(FeeShare::distribution_infos(0), None);
	});
}
