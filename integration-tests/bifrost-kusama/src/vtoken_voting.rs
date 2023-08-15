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

use crate::{kusama_integration_tests::*, kusama_test_net::*};
use bifrost_vtoken_voting::{TallyOf, VoteRole};
use frame_support::{assert_ok, dispatch::RawOrigin, traits::StorePreimage};
use node_primitives::currency::VKSM;
use pallet_conviction_voting::{AccountVote, Conviction, Tally, Vote};
use xcm_emulator::TestExt;

#[test]
fn vote_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		KusamaNet::execute_with(|| {
			use frame_support::traits::schedule::DispatchTime;
			use kusama_runtime::{Referenda, RuntimeEvent, RuntimeOrigin, System};

			assert_ok!(Referenda::submit(
				RuntimeOrigin::signed(ALICE.into()),
				Box::new(RawOrigin::Root.into()),
				set_balance_proposal_bounded(1),
				DispatchTime::At(10),
			));
			System::events().iter().for_each(|r| println!("KusamaNet >>> {:?}", r.event));
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::Referenda(pallet_referenda::Event::Submitted {
					index: _,
					track: _,
					proposal: _,
				})
			)));
		});

		Bifrost::execute_with(|| {
			use bifrost_kusama_runtime::{RuntimeEvent, RuntimeOrigin, System, VtokenVoting};

			let vtoken = VKSM;
			let poll_index = 0;

			assert_ok!(VtokenVoting::set_delegator_role(
				RuntimeOrigin::root(),
				VKSM,
				5,
				VoteRole::Standard { aye: true, conviction: Conviction::Locked5x },
			));
			assert_ok!(VtokenVoting::set_delegator_role(
				RuntimeOrigin::root(),
				VKSM,
				21,
				VoteRole::SplitAbstain,
			));

			assert_ok!(VtokenVoting::vote(
				RuntimeOrigin::signed(ALICE.into()),
				vtoken,
				poll_index,
				aye(2, 5)
			));
			assert_eq!(tally(vtoken, poll_index), Tally::from_parts(10, 0, 2));

			assert_ok!(VtokenVoting::update_referendum_status(
				RuntimeOrigin::signed(ALICE.into()),
				vtoken,
				poll_index,
			));

			System::events().iter().for_each(|r| println!("Bifrost >>> {:?}", r.event));
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::VtokenVoting(bifrost_vtoken_voting::Event::Voted {
					who: _,
					vtoken: VKSM,
					poll_index: 0,
					vote: _,
				})
			)));
		});

		KusamaNet::execute_with(|| {
			kusama_runtime::System::events()
				.iter()
				.for_each(|r| println!("KusamaNet >>> {:?}", r.event));

			assert!(kusama_runtime::System::events().iter().any(|r| matches!(
				r.event,
				kusama_runtime::RuntimeEvent::Ump(
					polkadot_runtime_parachains::ump::Event::ExecutedUpward(..)
				)
			)));
		});

		Bifrost::execute_with(|| {
			System::events().iter().for_each(|r| println!("Bifrost >>> {:?}", r.event));
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::VtokenVoting(bifrost_vtoken_voting::Event::VoteNotified {
					vtoken: _,
					poll_index: _,
					success: true,
				})
			)));
		});
	});
}

pub fn set_balance_proposal_bounded(
	value: Balance,
) -> pallet_referenda::BoundedCallOf<kusama_runtime::Runtime, ()> {
	let c = kusama_runtime::RuntimeCall::Balances(pallet_balances::Call::force_set_balance {
		who: MultiAddress::Id(AccountId::new(ALICE)),
		new_free: value,
	});
	<kusama_runtime::Preimage as StorePreimage>::bound(c).unwrap()
}

pub fn aye(amount: Balance, conviction: u8) -> AccountVote<Balance> {
	let vote = Vote { aye: true, conviction: conviction.try_into().unwrap() };
	AccountVote::Standard { vote, balance: amount }
}

fn tally(vtoken: CurrencyId, poll_index: u32) -> TallyOf<Runtime> {
	bifrost_kusama_runtime::VtokenVoting::as_ongoing(vtoken, poll_index)
		.expect("No poll")
		.0
}
