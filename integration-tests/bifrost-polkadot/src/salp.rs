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
	polkadot_integration_tests::*,
	polkadot_test_net::{register_token2_asset, Bifrost},
};
use bifrost_asset_registry::AssetIdMaps;
use bifrost_polkadot_runtime::{Runtime, Salp, SlotLength};
use bifrost_salp::{FundInfo, FundStatus};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use xcm_emulator::TestExt;

const DOT: u128 = 1_000_000_000_000;

#[test]
fn create_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		register_token2_asset();
		Bifrost::execute_with(|| {
			assert_eq!(AssetIdMaps::<Runtime>::check_token2_registered(0), true);
			assert_eq!(AssetIdMaps::<Runtime>::check_vsbond2_registered(0, 3000, 1, 8), false);
			// first_slot + 7 >= last_slot
			assert_ok!(Salp::create(
				RawOrigin::Root.into(),
				//paraid
				3_000,
				//cap
				100 * DOT,
				//first_slot
				1,
				//last_slot
				SlotLength::get()
			));
			assert_eq!(
				Salp::funds(3_000).unwrap(),
				FundInfo {
					raised: 0,
					cap: 100 * DOT,
					first_slot: 1,
					last_slot: SlotLength::get(),
					trie_index: 0,
					status: FundStatus::Ongoing,
				}
			);
			assert_eq!(AssetIdMaps::<Runtime>::check_vsbond2_registered(0, 3000, 1, 8), true);
		});
	})
}
