// Copyright 2019-2021 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

#![cfg(test)]

use crate::*;
use super::mock::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;

#[test]
fn issue_and_burn_should_work_as_expected() {
    ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
            let to_issue = 300;

            System::set_block_number(1);

            // Issue vKSM to alice
            assert_ok!(BrmlAssets::issue(RawOrigin::Root.into(), ALICE, vKSM, to_issue));
            // Check Alice vKSMs
			assert_eq!(Assets::free_balance(vKSM, &ALICE), to_issue);
            // Check totoal issuance
			assert_eq!(Assets::total_issuance(vKSM), to_issue);

            // Check event
			let issue_event = mock::Event::brml_assets(crate::Event::Issued(ALICE, vKSM, to_issue));
			assert!(System::events().iter().any(|record| record.event == issue_event));

            // Issue vKSM to bob
            assert_ok!(BrmlAssets::issue(RawOrigin::Root.into(), BOB, vKSM, to_issue));
            // Check Alice vKSMs
			assert_eq!(Assets::free_balance(vKSM, &BOB), to_issue);
            // Check totoal issuance
			assert_eq!(Assets::total_issuance(vKSM), to_issue * 2);

            // Destroy some vKSM from alice and bob
            let destroy_alice = 20;
            let destroy_bob = 50;
            assert_ok!(BrmlAssets::burn(RawOrigin::Root.into(), ALICE, vKSM, destroy_alice));

            // Check event
			let burn_event = mock::Event::brml_assets(crate::Event::Burned(ALICE, vKSM, destroy_alice));
			assert!(System::events().iter().any(|record| record.event == burn_event));

            assert_ok!(BrmlAssets::burn(RawOrigin::Root.into(), BOB, vKSM, destroy_bob));

            // // Check Alice and Bob vKSMs
            assert_eq!(Assets::free_balance(vKSM, &ALICE), to_issue - destroy_alice);
            assert_eq!(Assets::free_balance(vKSM, &BOB), to_issue - destroy_bob);
            // Check totoal issuance
			assert_eq!(Assets::total_issuance(vKSM), to_issue * 2 - destroy_alice - destroy_bob);

            // Alice and Bob should have no right to issue and butn tokens
            assert!(BrmlAssets::issue(mock::Origin::signed(ALICE), ALICE, vKSM, to_issue).is_err());
            assert!(BrmlAssets::burn(mock::Origin::signed(BOB), ALICE, vKSM, destroy_alice).is_err());
        });
}
