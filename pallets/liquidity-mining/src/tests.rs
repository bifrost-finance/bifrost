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

use frame_support::{assert_noop, assert_ok, dispatch::DispatchError};
use orml_traits::{MultiCurrency, MultiLockableCurrency, MultiReservableCurrency};

use crate::{mock::*, Error, PoolType};

#[test]
fn create_farming_pool_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_farming_pool(
			Some(CREATOR).into(),
			2001,
			13,
			20,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			MINUTES,
			1_000 * UNIT,
			0
		));

		let pool = LM::pool(0).unwrap();

		assert_eq!(pool.r#type, PoolType::Farming);

		let per_block = REWARD_AMOUNT / MINUTES as u128;
		let reserved = per_block * MINUTES as u128;
		let free = REWARD_AMOUNT - reserved;

		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).free, free);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).reserved, reserved);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).free, free);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).reserved, reserved);
	});
}
