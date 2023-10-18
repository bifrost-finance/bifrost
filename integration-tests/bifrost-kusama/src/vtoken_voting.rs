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
use bifrost_slp::{Ledger, MinimumsMaximums, SubstrateLedger};
use bifrost_vtoken_voting::{AccountVote, TallyOf, VoteRole};
use frame_support::{
	assert_ok,
	dispatch::RawOrigin,
	traits::{schedule::DispatchTime, StorePreimage},
	weights::Weight,
};
use node_primitives::{currency::VKSM, XcmOperationType as XcmOperation};
use pallet_conviction_voting::{Conviction, Vote};
use xcm::v3::Parent;
use xcm_emulator::TestExt;

#[test]
fn vote_works() {
	env_logger::init();

	sp_io::TestExternalities::default().execute_with(|| {
		let vtoken = VKSM;
		let poll_index = 0;

		KusamaNet::execute_with(|| {
			use kusama_runtime::{Referenda, RuntimeEvent, RuntimeOrigin, System};

			assert_ok!(Referenda::submit(
				RuntimeOrigin::signed(ALICE.into()),
				Box::new(RawOrigin::Root.into()),
				set_balance_proposal_bounded(1),
				DispatchTime::At(10),
			));
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::Referenda(pallet_referenda::Event::Submitted {
					index: 0,
					track: 0,
					proposal: _,
				})
			)));
			System::reset_events();
		});

		Bifrost::execute_with(|| {
			use bifrost_kusama_runtime::{
				RuntimeEvent, RuntimeOrigin, System, VtokenMinting, VtokenVoting,
			};

			assert_ok!(VtokenMinting::mint(
				RuntimeOrigin::signed(ALICE.into()),
				KSM,
				1_000_000_000_000,
				Default::default()
			));
			assert_eq!(
				<Runtime as bifrost_vtoken_voting::Config>::VTokenSupplyProvider::get_token_supply(
					KSM
				),
				Some(1_000_000_000_000)
			);
			assert_eq!(
				<Runtime as bifrost_vtoken_voting::Config>::VTokenSupplyProvider::get_vtoken_supply(
					VKSM
				),
				Some(1_000_000_000_000)
			);
			let token = CurrencyId::to_token(&vtoken).unwrap();
			assert_ok!(XcmInterface::set_xcm_dest_weight_and_fee(
				token,
				XcmOperation::Vote,
				Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
			));
			assert_ok!(XcmInterface::set_xcm_dest_weight_and_fee(
				token,
				XcmOperation::RemoveVote,
				Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
			));
			assert_ok!(Slp::set_minimums_and_maximums(
				RuntimeOrigin::root(),
				token,
				Some(MinimumsMaximums {
					delegator_bonded_minimum: 0u32.into(),
					bond_extra_minimum: 0u32.into(),
					unbond_minimum: 0u32.into(),
					rebond_minimum: 0u32.into(),
					unbond_record_maximum: 0u32,
					validators_back_maximum: 0u32,
					delegator_active_staking_maximum: 0u32.into(),
					validators_reward_maximum: 0u32,
					delegation_amount_minimum: 0u32.into(),
					delegators_maximum: u16::MAX,
					validators_maximum: 0u16,
				})
			));
			assert_ok!(Slp::add_delegator(
				RuntimeOrigin::root(),
				token,
				5,
				Box::new(Parent.into())
			));
			assert_ok!(Slp::set_delegator_ledger(
				RuntimeOrigin::root(),
				token,
				Box::new(Parent.into()),
				Box::new(Some(Ledger::Substrate(SubstrateLedger {
					account: Parent.into(),
					total: 100u32.into(),
					active: 100u32.into(),
					unlocking: vec![],
				})))
			));

			assert_ok!(VtokenVoting::set_delegator_role(
				RuntimeOrigin::root(),
				vtoken,
				5,
				VoteRole::Standard { aye: true, conviction: Conviction::Locked5x },
			));
			assert_ok!(VtokenVoting::set_vote_locking_period(RuntimeOrigin::root(), vtoken, 0));
			assert_ok!(VtokenVoting::set_undeciding_timeout(RuntimeOrigin::root(), vtoken, 100));

			assert_ok!(VtokenVoting::vote(
				RuntimeOrigin::signed(ALICE.into()),
				vtoken,
				poll_index,
				aye(2, 5)
			));
			assert_eq!(tally(vtoken, poll_index), TallyOf::<Runtime>::from_parts(10, 0, 2));

			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::VtokenVoting(bifrost_vtoken_voting::Event::Voted {
					who: _,
					vtoken: VKSM,
					poll_index: 0,
					new_vote: _,
					delegator_vote: _,
				})
			)));
			System::reset_events();
		});

		KusamaNet::execute_with(|| {
			use kusama_runtime::System;

			System::events().iter().for_each(|r| log::debug!("KusamaNet >>> {:?}", r.event));
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				kusama_runtime::RuntimeEvent::Ump(
					polkadot_runtime_parachains::ump::Event::ExecutedUpward(..)
				)
			)));
			System::reset_events();
		});

		Bifrost::execute_with(|| {
			use bifrost_kusama_runtime::{RuntimeEvent, System, VtokenVoting};

			System::events().iter().for_each(|r| log::debug!("Bifrost >>> {:?}", r.event));
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::VtokenVoting(bifrost_vtoken_voting::Event::VoteNotified {
					vtoken: VKSM,
					poll_index: 0,
					success: true,
				})
			)));
			assert_ok!(VtokenVoting::set_referendum_status(
				RuntimeOrigin::root(),
				VKSM,
				0,
				bifrost_vtoken_voting::ReferendumInfoOf::<Runtime>::Completed(1),
			));
			assert_ok!(VtokenVoting::remove_delegator_vote(
				RuntimeOrigin::signed(ALICE.into()),
				VKSM,
				0,
				5,
			));
			System::reset_events();
		});

		KusamaNet::execute_with(|| {
			use kusama_runtime::System;

			System::events().iter().for_each(|r| log::debug!("KusamaNet >>> {:?}", r.event));
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				kusama_runtime::RuntimeEvent::Ump(
					polkadot_runtime_parachains::ump::Event::ExecutedUpward(..)
				)
			)));
			System::reset_events();
		});

		Bifrost::execute_with(|| {
			use bifrost_kusama_runtime::{RuntimeEvent, System};

			System::events().iter().for_each(|r| log::debug!("Bifrost >>> {:?}", r.event));
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::VtokenVoting(
					bifrost_vtoken_voting::Event::DelegatorVoteRemovedNotified {
						vtoken: VKSM,
						poll_index: 0,
						success: true,
					}
				)
			)));
			System::reset_events();
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
	bifrost_kusama_runtime::VtokenVoting::ensure_referendum_ongoing(vtoken, poll_index)
		.expect("No poll")
		.tally
}
