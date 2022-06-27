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

use crate::{mock::*,*};
use frame_support::{assert_ok, sp_runtime::Permill};

#[test]
fn token_config_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(SystemStaking::token_config(
			Origin::root(),
			KSM,
			Some(1),
			Some(Permill::from_percent(80)),
			Some(false),
			Some(100),
			None,
		));
    let token_info = <TokenStatus<Runtime>>::get(KSM).unwrap();
    assert_eq!(token_info.new_config.add_or_sub, false);
    assert_eq!(token_info.new_config.exec_delay, 1);
    assert_eq!(token_info.new_config.system_stakable_farming_rate, Permill::from_percent(80));
    assert_eq!(token_info.new_config.system_stakable_base, 100);
    assert_eq!(token_info.new_config.farming_poolids, Vec::<PoolId>::new());
	});
}

// #[test]
// fn refresh_token_info_should_work() {
//   ExtBuilder::default().build().execute_with(|| {
//     assert_ok!(SystemStaking::token_config(
//       Origin::root(),
//       KSM,
//       Some(1),
//       Some(Permill::from_percent(80)),
//       Some(false),
//       Some(100),
//       None,
//     ));
//     let token_info = <TokenStatus<Runtime>>::get(KSM).unwrap();
// }
