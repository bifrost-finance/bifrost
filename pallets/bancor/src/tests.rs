// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

use crate::mock::*;
use crate::*;
use frame_support::{assert_noop, assert_ok};


#[test]
fn exchange_should_work() {
	ExtBuilder::default()
	.one_thousand_for_alice_n_bob()
	.build()
	.execute_with(|| {
		
		// Check if the bancor pools have already been initialized.
		let ksm_pool = Bancor::get_bancor_pool(KSM).unwrap();
		let dot_pool = Bancor::get_bancor_pool(DOT).unwrap();

		assert_eq!(ksm_pool, BancorPool { currency_id: CurrencyId::Token(TokenSymbol::KSM), token_pool: 0, vstoken_pool: 0, token_base_supply: 2000000000000000000, vstoken_base_supply: 1000000000000000000 });
		assert_eq!(dot_pool, BancorPool { currency_id: CurrencyId::Token(TokenSymbol::DOT), token_pool: 0, vstoken_pool: 0, token_base_supply: 20000000000000000, vstoken_base_supply: 10000000000000000 });

		// the pool has no real DOT
		assert_noop!(Bancor::exchange(Origin::signed(ALICE), DOT, 50), Error::<Test>::TokenSupplyNotEnought);

		let updated_pool = BancorPool  {
			currency_id: dot_pool.currency_id,
			token_pool: 100,
			vstoken_pool: dot_pool.vstoken_pool,
			token_base_supply: dot_pool.token_base_supply,
			vstoken_base_supply: dot_pool.vstoken_base_supply
		};

		// add some DOTs to the pool
		BancorPools::<Test>::insert(DOT, updated_pool);

		// exchange succeeds.
		assert_ok!(Bancor::exchange(Origin::signed(ALICE), DOT, 50));
		let dot_pool = Bancor::get_bancor_pool(DOT).unwrap();
		assert_eq!(dot_pool, BancorPool { currency_id: CurrencyId::Token(TokenSymbol::DOT), token_pool: 50, vstoken_pool: 50, token_base_supply: 20000000000000000, vstoken_base_supply: 10000000000000000 });

		// check ALICE's account balances. ALICE should have 1000-50 VSDOT, and 1000+50 DOT.
		assert_eq!(Assets::free_balance(DOT, &ALICE), 1050);
		assert_eq!(Assets::free_balance(VSDOT, &ALICE), 950);
});
}