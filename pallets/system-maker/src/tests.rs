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
fn on_idle() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let para_id = 2001u32;
		let asset_0_currency_id: AssetId =
			AssetId::try_convert_from(RelayCurrencyId::get(), para_id).unwrap();
		let asset_1_currency_id: AssetId = AssetId::try_convert_from(vKSM, para_id).unwrap();
		// let path = vec![asset_0_currency_id, asset_1_currency_id];
		assert_ok!(ZenlinkProtocol::create_pair(
			Origin::root(),
			asset_0_currency_id,
			asset_1_currency_id
		));
		let deadline: BlockNumberFor<Runtime> = <frame_system::Pallet<Runtime>>::block_number() +
			<Runtime as frame_system::Config>::BlockNumber::from(100u32);
		assert_ok!(ZenlinkProtocol::add_liquidity(
			Origin::signed(ALICE),
			asset_0_currency_id,
			asset_1_currency_id,
			1000,
			2200,
			1,
			1,
			deadline
		));

		SystemMaker::set_config(
			Origin::signed(ALICE),
			RelayCurrencyId::get(),
			Info { annualization: Permill::from_percent(100), granularity: 1000 },
		);
		let system_maker =
			<Runtime as Config>::SystemMakerPalletId::get().into_account_truncating();
		assert_eq!(Tokens::free_balance(KSM, &system_maker), 10000);
		SystemMaker::on_idle(<frame_system::Pallet<Runtime>>::block_number(), 100000000);
		System::set_block_number(System::block_number() + 1);
		// SystemMaker::on_idle(<frame_system::Pallet<Runtime>>::block_number() + 1, 100);
		let a = ZenlinkProtocol::pair_account_id(asset_0_currency_id, asset_1_currency_id);
		// assert_eq!(Tokens::free_balance(KSM, &system_maker), 10000);
		assert_eq!(Tokens::free_balance(vKSM, &system_maker), 11098);
		assert_eq!(Tokens::free_balance(KSM, &a), 2000);
		assert_eq!(Tokens::free_balance(vKSM, &a), 1102);
	});
}
