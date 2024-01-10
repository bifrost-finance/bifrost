// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
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

use bifrost_kusama_runtime::{
	Runtime, RuntimeOrigin, Slp, XcmDestWeightAndFeeHandler, XcmInterface,
};
use bifrost_primitives::{
	currency::VKSM, Balance, CurrencyId, VTokenSupplyProvider, XcmOperationType as XcmOperation,
	KSM,
};
use bifrost_slp::{Ledger, MinimumsMaximums, SubstrateLedger};
use bifrost_vtoken_voting::{AccountVote, TallyOf};
use frame_support::{
	assert_ok,
	dispatch::RawOrigin,
	traits::{schedule::DispatchTime, StorePreimage},
	weights::Weight,
};
use integration_tests_common::{BifrostKusama, BifrostKusamaAlice, Kusama, KusamaAlice};
use pallet_conviction_voting::Vote;
use sp_runtime::Perbill;
use xcm::v3::Parent;
use xcm_emulator::{Parachain, RelayChain, TestExt};

#[test]
fn vote_works() {
	let vtoken = VKSM;
	let poll_index = 0;

	Kusama::execute_with(|| {
		use kusama_runtime::{Balances, Referenda, RuntimeEvent, RuntimeOrigin, System, Utility};

		assert_ok!(Balances::force_set_balance(
			RuntimeOrigin::root(),
			Kusama::sovereign_account_id_of_child_para(BifrostKusama::para_id()).into(),
			1_000_000_000_000_000u128
		));
		assert_ok!(Balances::force_set_balance(
			RuntimeOrigin::root(),
			Utility::derivative_account_id(
				Kusama::sovereign_account_id_of_child_para(BifrostKusama::para_id()).into(),
				5
			)
			.into(),
			1_000_000_000_000_000u128
		));
		assert_ok!(Referenda::submit(
			RuntimeOrigin::signed(BifrostKusamaAlice::get()),
			Box::new(RawOrigin::Root.into()),
			set_balance_proposal_bounded(1),
			DispatchTime::At(10),
		));
		System::events().iter().for_each(|r| log::debug!("Kusama >>> {:?}", r.event));
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

	BifrostKusama::execute_with(|| {
		use bifrost_kusama_runtime::{
			RuntimeEvent, RuntimeOrigin, System, VtokenMinting, VtokenVoting,
		};

		assert_ok!(VtokenMinting::mint(
			RuntimeOrigin::signed(BifrostKusamaAlice::get()),
			KSM,
			1_000_000_000_000,
			Default::default(),
			None
		));
		assert_eq!(
			<Runtime as bifrost_vtoken_voting::Config>::VTokenSupplyProvider::get_token_supply(KSM),
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
		assert_ok!(Slp::add_delegator(RuntimeOrigin::root(), token, 5, Box::new(Parent.into())));
		assert_ok!(Slp::set_delegator_ledger(
			RuntimeOrigin::root(),
			token,
			Box::new(Parent.into()),
			Box::new(Some(Ledger::Substrate(SubstrateLedger {
				account: Parent.into(),
				total: 1_000_000_000_000u128,
				active: 1_000_000_000_000u128,
				unlocking: vec![],
			})))
		));

		assert_ok!(VtokenVoting::set_vote_cap_ratio(
			RuntimeOrigin::root(),
			vtoken,
			Perbill::from_percent(90)
		));
		assert_ok!(VtokenVoting::add_delegator(RuntimeOrigin::root(), vtoken, 5));
		assert_ok!(VtokenVoting::set_vote_locking_period(RuntimeOrigin::root(), vtoken, 0));
		assert_ok!(VtokenVoting::set_undeciding_timeout(RuntimeOrigin::root(), vtoken, 100));

		assert_ok!(VtokenVoting::vote(
			RuntimeOrigin::signed(BifrostKusamaAlice::get()),
			vtoken,
			poll_index,
			aye(1_000_000_000_000u128, 5)
		));

		assert_eq!(
			tally(vtoken, poll_index),
			TallyOf::<Runtime>::from_parts(5_000_000_000_000, 0, 1_000_000_000_000)
		);

		assert!(System::events().iter().any(|r| matches!(
			r.event,
			RuntimeEvent::VtokenVoting(bifrost_vtoken_voting::Event::Voted {
				who: _,
				vtoken: VKSM,
				poll_index: 0,
				token_vote: _,
				delegator_vote: _,
			})
		)));
		System::reset_events();
	});

	Kusama::execute_with(|| {
		use kusama_runtime::System;

		System::events().iter().for_each(|r| log::debug!("Kusama >>> {:?}", r.event));
		assert!(System::events().iter().any(|r| matches!(
			r.event,
			kusama_runtime::RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed {
				id: _,
				origin: _,
				weight_used: _,
				success: true
			})
		)));
		System::reset_events();
	});

	BifrostKusama::execute_with(|| {
		use bifrost_kusama_runtime::{RuntimeEvent, System, VtokenVoting};

		System::events()
			.iter()
			.for_each(|r| log::debug!("BifrostKusama >>> {:?}", r.event));
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
			RuntimeOrigin::signed(BifrostKusamaAlice::get()),
			VKSM,
			0,
			5,
		));
		System::reset_events();
	});

	Kusama::execute_with(|| {
		use kusama_runtime::{RuntimeEvent, System};

		System::events().iter().for_each(|r| log::debug!("Kusama >>> {:?}", r.event));
		assert!(System::events().iter().any(|r| matches!(
			&r.event,
			RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed {
				id: _,
				origin: _,
				weight_used: _,
				success: true
			})
		)));
	});

	BifrostKusama::execute_with(|| {
		use bifrost_kusama_runtime::{RuntimeEvent, System};

		System::events()
			.iter()
			.for_each(|r| log::debug!("BifrostKusama >>> {:?}", r.event));
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
	});
}

pub fn set_balance_proposal_bounded(
	value: Balance,
) -> pallet_referenda::BoundedCallOf<kusama_runtime::Runtime, ()> {
	let c = kusama_runtime::RuntimeCall::Balances(pallet_balances::Call::force_set_balance {
		who: KusamaAlice::get().into(),
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
