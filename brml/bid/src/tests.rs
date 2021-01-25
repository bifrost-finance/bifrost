// Copyright 2019-2020 Liebi Technologies.
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

//! Tests for the module.

#![cfg(test)]

use crate::mock::*;
use crate::*;
use frame_support::{assert_ok, dispatch::DispatchError};
use node_primitives::TokenType;

fn storages_initialization() {
	let symbol0 = b"BNC".to_vec();
	let precision0 = 12;
	let token_type0 = TokenType::Native;
	<Test as Trait>::AssetTrait::asset_create(symbol0, precision0, token_type0).unwrap_or_default();

	let symbol1 = b"aUSD".to_vec();
	let precision1 = 12;
	let token_type1 = TokenType::Stable;
	<Test as Trait>::AssetTrait::asset_create(symbol1, precision1, token_type1).unwrap_or_default();

	let symbol2 = b"DOT".to_vec();
	let precision2 = 12;
	<Test as Trait>::AssetTrait::asset_create_pair(symbol2, precision2).unwrap_or_default();

	let symbol3 = b"KSM".to_vec();
	let precision3 = 12;
	<Test as Trait>::AssetTrait::asset_create_pair(symbol3, precision3).unwrap_or_default();

	let symbol4 = b"EOS".to_vec();
	let precision4 = 12;
	<Test as Trait>::AssetTrait::asset_create_pair(symbol4, precision4).unwrap_or_default();

	let symbol5 = b"IOST".to_vec();
	let precision5 = 12;
	<Test as Trait>::AssetTrait::asset_create_pair(symbol5, precision5).unwrap_or_default();

	let alice = 1;
	let bob = 2;
	let charlie = 3;
	// let dole = 4;
	// let eddie = 5;
	// let frank = 6;
	// let gorge = 7;
	// let henry = 8;
	// let ian = 9;
	// let jerry = 10;

	let dot_id = 2;
	let ksm_id = 4;
	let eos_id = 6;
	let iost_id = 8;

	let amount = 100_000_000;
	// create some assets for bidder Alice
	<Test as Trait>::AssetTrait::asset_issue(dot_id, &alice, amount);
	<Test as Trait>::AssetTrait::asset_issue(ksm_id, &alice, amount);
	<Test as Trait>::AssetTrait::asset_issue(eos_id, &alice, amount);
	<Test as Trait>::AssetTrait::asset_issue(iost_id, &alice, amount);

	let amount = 10_000;
	// create some assets for bidder Bob
	<Test as Trait>::AssetTrait::asset_issue(dot_id, &bob, amount);
	<Test as Trait>::AssetTrait::asset_issue(ksm_id, &bob, amount);
	<Test as Trait>::AssetTrait::asset_issue(eos_id, &bob, amount);
	<Test as Trait>::AssetTrait::asset_issue(iost_id, &bob, amount);

	let amount = 100;
	// create some assets for bidder Charlie
	<Test as Trait>::AssetTrait::asset_issue(dot_id, &charlie, amount);
	<Test as Trait>::AssetTrait::asset_issue(ksm_id, &charlie, amount);
	<Test as Trait>::AssetTrait::asset_issue(eos_id, &charlie, amount);
	<Test as Trait>::AssetTrait::asset_issue(iost_id, &charlie, amount);

	// register vtokens
	let origin_root = Origin::root();
	let vdot_id = 3;
	let vksm_id = 5;
	let veos_id = 7;
	let viost_id = 9;
	Bid::register_vtoken_for_bidding(origin_root.clone(), vdot_id).unwrap_or_default();
	Bid::register_vtoken_for_bidding(origin_root.clone(), vksm_id).unwrap_or_default();
	Bid::register_vtoken_for_bidding(origin_root.clone(), veos_id).unwrap_or_default();
	Bid::register_vtoken_for_bidding(origin_root.clone(), viost_id).unwrap_or_default();

	let minimum_lasting_block_num_vdot = 43_200;
	let maximum_lasting_block_num_vdot = 432_000;
	Bid::set_min_max_order_lasting_block_num(
		origin_root.clone(),
		vdot_id,
		minimum_lasting_block_num_vdot,
		maximum_lasting_block_num_vdot,
	)
	.unwrap_or_default();
	let minimum_lasting_block_num_vksm = 7;
	let maximum_lasting_block_num_vksm = 21;
	Bid::set_min_max_order_lasting_block_num(
		origin_root.clone(),
		vksm_id,
		minimum_lasting_block_num_vksm,
		maximum_lasting_block_num_vksm,
	)
	.unwrap_or_default();

	let minimum_lasting_block_num_veos = 3_600;
	let maximum_lasting_block_num_veos = 36_000;
	Bid::set_min_max_order_lasting_block_num(
		origin_root.clone(),
		veos_id,
		minimum_lasting_block_num_veos,
		maximum_lasting_block_num_veos,
	)
	.unwrap_or_default();

	let minimum_lasting_block_num_viost = 3_600;
	let maximum_lasting_block_num_viost = 36_000;
	Bid::set_min_max_order_lasting_block_num(
		origin_root.clone(),
		viost_id,
		minimum_lasting_block_num_viost,
		maximum_lasting_block_num_viost,
	)
	.unwrap_or_default();

	// set blocks number per era
	let block_num_per_era_vdot = 14_400;
	Bid::set_block_number_per_era(origin_root.clone(), vdot_id, block_num_per_era_vdot)
		.unwrap_or_default();

	let block_num_per_era_vksm = 7;
	Bid::set_block_number_per_era(origin_root.clone(), vksm_id, block_num_per_era_vksm)
		.unwrap_or_default();

	let block_num_per_era_veos = 172_800;
	Bid::set_block_number_per_era(origin_root.clone(), veos_id, block_num_per_era_veos)
		.unwrap_or_default();

	let block_num_per_era_viost = 172_800;
	Bid::set_block_number_per_era(origin_root.clone(), viost_id, block_num_per_era_viost)
		.unwrap_or_default();

	// set_service_stop_block_num_lag
	let service_stop_lag_block_num_vdot = 0;
	Bid::set_service_stop_block_num_lag(
		origin_root.clone(),
		vdot_id,
		service_stop_lag_block_num_vdot,
	)
	.unwrap_or_default();

	let service_stop_lag_block_num_vksm = 0;
	Bid::set_service_stop_block_num_lag(
		origin_root.clone(),
		vksm_id,
		service_stop_lag_block_num_vksm,
	)
	.unwrap_or_default();

	let service_stop_lag_block_num_veos = 0;
	Bid::set_service_stop_block_num_lag(
		origin_root.clone(),
		veos_id,
		service_stop_lag_block_num_veos,
	)
	.unwrap_or_default();

	let service_stop_lag_block_num_viost = 518_400;
	Bid::set_service_stop_block_num_lag(
		origin_root.clone(),
		viost_id,
		service_stop_lag_block_num_viost,
	)
	.unwrap_or_default();

	// set_slash_margin_rates
	let set_slash_margin_rates_vdot = 30;
	Bid::set_slash_margin_rates(origin_root.clone(), vdot_id, set_slash_margin_rates_vdot)
		.unwrap_or_default();

	let set_slash_margin_rates_vksm = 30;
	Bid::set_slash_margin_rates(origin_root.clone(), vksm_id, set_slash_margin_rates_vksm)
		.unwrap_or_default();

	let set_slash_margin_rates_veos = 30;
	Bid::set_slash_margin_rates(origin_root.clone(), veos_id, set_slash_margin_rates_veos)
		.unwrap_or_default();

	let set_slash_margin_rates_viost = 30;
	Bid::set_slash_margin_rates(origin_root.clone(), viost_id, set_slash_margin_rates_viost)
		.unwrap_or_default();

	// create a proposal for Bob
	let bob = 2;
	let origin_bob = Origin::signed(bob);
	let votes_needed_bob = 200;
	let annual_roi_bob = 6000;
	let validator_bob = bob;
	Bid::create_bidding_proposal(
		origin_bob,
		vksm_id,
		votes_needed_bob,
		annual_roi_bob,
		validator_bob,
	)
	.unwrap_or_default();
}

#[test]
fn register_vtoken_for_bidding_should_work() {
	new_test_ext().execute_with(|| {
		let alice = 1;
		let origin_signed = Origin::signed(alice);
		let origin_root = Origin::root();

		let symbol = b"DOT".to_vec();
		let precision = 12;
		// create assets
		let (token_id, vtoken_id) =
			<Test as Trait>::AssetTrait::asset_create_pair(symbol, precision).unwrap_or_default();

		assert_eq!(<Test as Trait>::AssetTrait::is_token(token_id), true);
		assert_eq!(<Test as Trait>::AssetTrait::is_v_token(vtoken_id), true);

		// a user cannot register a vtoken for bidding
		assert_eq!(
			Bid::register_vtoken_for_bidding(origin_signed, vtoken_id),
			Err(DispatchError::BadOrigin)
		);
		assert_ok!(Bid::register_vtoken_for_bidding(
			origin_root.clone(),
			vtoken_id
		));

		// token is not allowed to be registered for bidding
		assert_eq!(
			Bid::register_vtoken_for_bidding(origin_root.clone(), token_id),
			Err(DispatchError::Module {
				index: 0,
				error: 1,
				message: Some("NotValidVtoken")
			})
		);

		// repeatedly registering the same vtoken is not allowed
		assert_eq!(
			Bid::register_vtoken_for_bidding(origin_root.clone(), vtoken_id),
			Err(DispatchError::Module {
				index: 0,
				error: 14,
				message: Some("VtokenAlreadyRegistered")
			})
		);

		// vtoken is already in the VtokensRegisteredForBidding storage.
		assert_eq!(
			VtokensRegisteredForBidding::<Test>::get().contains(&vtoken_id),
			true
		);

		// vtoken is already in the BiddingQueues storage.
		assert_eq!(BiddingQueues::<Test>::contains_key(vtoken_id), true);

		// vtoken is already in the TotalProposalsInQueue storage.
		assert_eq!(TotalProposalsInQueue::<Test>::contains_key(vtoken_id), true);

		// vtoken is already in the TokenOrderROIList storage.
		assert_eq!(TokenOrderROIList::<Test>::contains_key(vtoken_id), true);

		// vtoken is already in the TotalVotesInService storage.
		assert_eq!(TotalVotesInService::<Test>::contains_key(vtoken_id), true);
	});
}

#[test]
fn set_min_max_order_lasting_block_num_should_work() {
	new_test_ext().execute_with(|| {
		let alice = 1;
		let origin_signed = Origin::signed(alice);
		let origin_root = Origin::root();

		let symbol = b"DOT".to_vec();
		let precision = 12;

		let minimum_lasting_block_num = 43_200;
		let maximum_lasting_block_num = 432_000;
		// create assets
		let (token_id, vtoken_id) =
			<Test as Trait>::AssetTrait::asset_create_pair(symbol, precision).unwrap_or_default();

		// register vtoken
		Bid::register_vtoken_for_bidding(origin_root.clone(), vtoken_id).unwrap_or_default();

		// a user cannot set_min_max_order_lasting_block_num
		assert_eq!(
			Bid::set_min_max_order_lasting_block_num(
				origin_signed.clone(),
				vtoken_id,
				minimum_lasting_block_num,
				maximum_lasting_block_num
			),
			Err(DispatchError::BadOrigin)
		);

		// not a registered vtoken
		assert_eq!(
			Bid::set_min_max_order_lasting_block_num(
				origin_root.clone(),
				token_id,
				minimum_lasting_block_num,
				maximum_lasting_block_num
			),
			Err(DispatchError::Module {
				index: 0,
				error: 15,
				message: Some("VtokenNotRegistered")
			})
		);

		// minimum is larger than maximum
		assert_eq!(
			Bid::set_min_max_order_lasting_block_num(
				origin_root.clone(),
				vtoken_id,
				maximum_lasting_block_num,
				minimum_lasting_block_num
			),
			Err(DispatchError::Module {
				index: 0,
				error: 9,
				message: Some("MinimumOrMaximumNotRight")
			})
		);

		assert_ok!(Bid::set_min_max_order_lasting_block_num(
			origin_root.clone(),
			vtoken_id,
			minimum_lasting_block_num,
			maximum_lasting_block_num
		));

		// check the first time insert number
		assert_eq!(
			MinMaxOrderLastingBlockNum::<Test>::get(vtoken_id),
			(minimum_lasting_block_num, maximum_lasting_block_num)
		);

		// change the minimum and maximum lasting block number
		assert_ok!(Bid::set_min_max_order_lasting_block_num(
			origin_root.clone(),
			vtoken_id,
			20,
			200
		));

		// validate the newly revised number
		assert_eq!(
			MinMaxOrderLastingBlockNum::<Test>::get(vtoken_id),
			(20, 200)
		);
	});
}

#[test]
fn set_block_number_per_era_should_work() {
	new_test_ext().execute_with(|| {
		let alice = 1;
		let origin_signed = Origin::signed(alice);
		let origin_root = Origin::root();

		let symbol = b"DOT".to_vec();
		let precision = 12;

		// create assets
		let (token_id, vtoken_id) =
			<Test as Trait>::AssetTrait::asset_create_pair(symbol, precision).unwrap_or_default();

		// register vtoken
		Bid::register_vtoken_for_bidding(origin_root.clone(), vtoken_id).unwrap_or_default();

		let block_num_per_era = 14_400;
		// a user cannot set_block_number_per_era
		assert_eq!(
			Bid::set_block_number_per_era(origin_signed.clone(), vtoken_id, block_num_per_era),
			Err(DispatchError::BadOrigin)
		);

		// not a registered vtoken
		assert_eq!(
			Bid::set_block_number_per_era(origin_root.clone(), token_id, block_num_per_era),
			Err(DispatchError::Module {
				index: 0,
				error: 15,
				message: Some("VtokenNotRegistered")
			})
		);

		assert_ok!(Bid::set_block_number_per_era(
			origin_root.clone(),
			vtoken_id,
			block_num_per_era
		));

		// check the first time insert number
		assert_eq!(BlockNumberPerEra::<Test>::get(vtoken_id), block_num_per_era);

		// change the block_number_per_era
		assert_ok!(Bid::set_block_number_per_era(
			origin_root.clone(),
			vtoken_id,
			20_000
		));

		// validate the newly revised number
		assert_eq!(BlockNumberPerEra::<Test>::get(vtoken_id), 20_000);
	});
}

#[test]
fn set_service_stop_block_num_lag_should_work() {
	new_test_ext().execute_with(|| {
		let alice = 1;
		let origin_signed = Origin::signed(alice);
		let origin_root = Origin::root();

		let symbol = b"DOT".to_vec();
		let precision = 12;

		// create assets
		let (token_id, vtoken_id) =
			<Test as Trait>::AssetTrait::asset_create_pair(symbol, precision).unwrap_or_default();

		// register vtoken
		Bid::register_vtoken_for_bidding(origin_root.clone(), vtoken_id).unwrap_or_default();

		let service_stop_lag_block_num = 0;
		// a user cannot set_service_stop_block_num_lag
		assert_eq!(
			Bid::set_service_stop_block_num_lag(
				origin_signed.clone(),
				vtoken_id,
				service_stop_lag_block_num
			),
			Err(DispatchError::BadOrigin)
		);

		// not a registered vtoken
		assert_eq!(
			Bid::set_service_stop_block_num_lag(
				origin_root.clone(),
				token_id,
				service_stop_lag_block_num
			),
			Err(DispatchError::Module {
				index: 0,
				error: 15,
				message: Some("VtokenNotRegistered")
			})
		);

		assert_ok!(Bid::set_service_stop_block_num_lag(
			origin_root.clone(),
			vtoken_id,
			service_stop_lag_block_num
		));

		// check the first time insert number
		assert_eq!(
			ServiceStopBlockNumLag::<Test>::get(vtoken_id),
			service_stop_lag_block_num
		);

		// change the service_stop_block_num_lag
		assert_ok!(Bid::set_service_stop_block_num_lag(
			origin_root.clone(),
			vtoken_id,
			200
		));

		// validate the newly revised number
		assert_eq!(ServiceStopBlockNumLag::<Test>::get(vtoken_id), 200);
	});
}

#[test]
fn set_slash_margin_rates_should_work() {
	new_test_ext().execute_with(|| {
		let alice = 1;
		let origin_signed = Origin::signed(alice);
		let origin_root = Origin::root();

		let symbol = b"DOT".to_vec();
		let precision = 12;

		// create assets
		let (token_id, vtoken_id) =
			<Test as Trait>::AssetTrait::asset_create_pair(symbol, precision).unwrap_or_default();

		// register vtoken
		Bid::register_vtoken_for_bidding(origin_root.clone(), vtoken_id).unwrap_or_default();

		let slash_margin = 15;
		// a user cannot set_slash_margin_rates
		assert_eq!(
			Bid::set_slash_margin_rates(origin_signed.clone(), vtoken_id, slash_margin),
			Err(DispatchError::BadOrigin)
		);

		// not a registered vtoken
		assert_eq!(
			Bid::set_slash_margin_rates(origin_root.clone(), token_id, slash_margin),
			Err(DispatchError::Module {
				index: 0,
				error: 15,
				message: Some("VtokenNotRegistered")
			})
		);

		assert_eq!(
			Bid::set_slash_margin_rates(origin_root.clone(), vtoken_id, 200),
			Err(DispatchError::Module {
				index: 0,
				error: 8,
				message: Some("RateExceedUpperBound")
			})
		);

		assert_ok!(Bid::set_slash_margin_rates(
			origin_root.clone(),
			vtoken_id,
			slash_margin
		));

		// check the first time insert number
		assert_eq!(
			SlashMarginRates::<Test>::get(vtoken_id),
			Permill::from_parts(slash_margin * 10000)
		);

		// change the set_slash_margin_rates
		assert_ok!(Bid::set_slash_margin_rates(
			origin_root.clone(),
			vtoken_id,
			50
		));

		// validate the newly revised number
		assert_eq!(
			SlashMarginRates::<Test>::get(vtoken_id),
			Permill::from_parts(50 * 10000)
		);
	});
}

#[test]
fn create_bidding_proposal_should_work() {
	new_test_ext().execute_with(|| {
		// initialization
		storages_initialization();

		// Alice creates a proposal
		let alice = 1;
		let origin_alice = Origin::signed(alice);
		let dot_id = 2;
		let vdot_id = 3;
		let votes_needed_alice = 10_000;
		let annual_roi_alice = 1_500; // 15% annual roi
		let validator = alice;

		// un-registered token is not allowed
		assert_eq!(
			Bid::create_bidding_proposal(
				origin_alice.clone(),
				dot_id,
				votes_needed_alice,
				annual_roi_alice,
				validator
			),
			Err(DispatchError::Module {
				index: 0,
				error: 15,
				message: Some("VtokenNotRegistered")
			})
		);

		// votes needed below the minimum limit of 100
		let votes_needed = 50;
		assert_eq!(
			Bid::create_bidding_proposal(
				origin_alice.clone(),
				vdot_id,
				votes_needed,
				annual_roi_alice,
				validator
			),
			Err(DispatchError::Module {
				index: 0,
				error: 5,
				message: Some("VotesExceedLowerBound")
			})
		);

		// votes needed above the maximum limit of 50_000
		let votes_needed = 60_000;
		assert_eq!(
			Bid::create_bidding_proposal(
				origin_alice.clone(),
				vdot_id,
				votes_needed,
				annual_roi_alice,
				validator
			),
			Err(DispatchError::Module {
				index: 0,
				error: 6,
				message: Some("VotesExceedUpperBound")
			})
		);

		// annual_roi should be above zero
		let annual_roi = 0;
		assert_eq!(
			Bid::create_bidding_proposal(
				origin_alice.clone(),
				vdot_id,
				votes_needed_alice,
				annual_roi,
				validator
			),
			Err(DispatchError::Module {
				index: 0,
				error: 4,
				message: Some("AmountNotAboveZero")
			})
		);

		// annual_roi should be above zero
		let annual_roi = 12_000;
		assert_eq!(
			Bid::create_bidding_proposal(
				origin_alice.clone(),
				vdot_id,
				votes_needed_alice,
				annual_roi,
				validator
			),
			Err(DispatchError::Module {
				index: 0,
				error: 19,
				message: Some("ROIExceedOneHundredPercent")
			})
		);

		assert_ok!(Bid::create_bidding_proposal(
			origin_alice.clone(),
			vdot_id,
			votes_needed_alice,
			annual_roi_alice,
			validator
		));

		// ProposalsInQueue storage
		let proposal_id = 1; // Bob has the 1st order in initialization. Alice inserts the second order.
		let roi_permill = Permill::from_parts(annual_roi_alice * 100);
		let proposal = ProposalsInQueue::<Test>::get(proposal_id);
		assert_eq!(proposal.bidder_id, alice);
		assert_eq!(proposal.token_id, vdot_id);
		assert_eq!(proposal.block_num, 0); // the block height when this proposal is created.
		assert_eq!(proposal.votes, votes_needed_alice);
		assert_eq!(proposal.annual_roi, roi_permill);
		assert_eq!(proposal.validator, validator);

		// BiddingQueues storage
		assert_eq!(
			BiddingQueues::<Test>::get(vdot_id).contains(&(roi_permill, proposal_id)),
			true
		);

		// ProposalNextId. Bob and Alice have created an bidding proposal respectively.
		assert_eq!(ProposalNextId::<Test>::get(), 2);

		// BidderProposalInQueue
		assert_eq!(
			BidderProposalInQueue::<Test>::get(alice, vdot_id).contains(&proposal_id),
			true
		);

		// TotalProposalsInQueue
		assert_eq!(
			TotalProposalsInQueue::<Test>::get(vdot_id),
			votes_needed_alice
		);

		// charlie only has 100 dot. Thus he doesn't have enough money for creating a proposal.
		let charlie = 3;
		let origin_charlie = Origin::signed(charlie);
		let votes_needed_charlie = 50_000;
		let annual_roi_charlie = 1000;
		let validator = charlie;
		assert_eq!(
			Bid::create_bidding_proposal(
				origin_charlie,
				vdot_id,
				votes_needed_charlie,
				annual_roi_charlie,
				validator
			),
			Err(DispatchError::Module {
				index: 0,
				error: 2,
				message: Some("NotEnoughBalance")
			})
		);

		// insert 4 more proposals for vdot proposal, so that the 6th vdot proposal should not be allowed to add
		Bid::create_bidding_proposal(
			origin_alice.clone(),
			vdot_id,
			votes_needed_alice,
			1_000,
			validator,
		)
		.unwrap_or_default();
		Bid::create_bidding_proposal(
			origin_alice.clone(),
			vdot_id,
			votes_needed_alice,
			7_000,
			validator,
		)
		.unwrap_or_default();
		Bid::create_bidding_proposal(
			origin_alice.clone(),
			vdot_id,
			votes_needed_alice,
			9_000,
			validator,
		)
		.unwrap_or_default();
		Bid::create_bidding_proposal(
			origin_alice.clone(),
			vdot_id,
			votes_needed_alice,
			4_400,
			validator,
		)
		.unwrap_or_default();

		assert_eq!(
			Bid::create_bidding_proposal(
				origin_alice.clone(),
				vdot_id,
				votes_needed_alice,
				annual_roi_alice,
				validator
			),
			Err(DispatchError::Module {
				index: 0,
				error: 18,
				message: Some("ProposalsExceedLimit")
			})
		);

		// check whether the order of proposals are correct. Proposal 0 is a vksm order from Bob created in initialization.
		// proposal_id = 1, roi = 15
		// proposal_id = 2, roi = 10
		// proposal_id = 3, roi = 70
		// proposal_id = 4, roi = 90
		// proposal_id = 5, roi = 44
		// So the correct order is [1, 0, 4, 2, 3]

		assert_eq!(
			BiddingQueues::<Test>::get(vdot_id).get(0).unwrap(),
			&(Permill::from_parts(10 * 10_000), 2)
		);
		assert_eq!(
			BiddingQueues::<Test>::get(vdot_id).get(1).unwrap(),
			&(Permill::from_parts(15 * 10_000), 1)
		);
		assert_eq!(
			BiddingQueues::<Test>::get(vdot_id).get(2).unwrap(),
			&(Permill::from_parts(44 * 10_000), 5)
		);
		assert_eq!(
			BiddingQueues::<Test>::get(vdot_id).get(3).unwrap(),
			&(Permill::from_parts(70 * 10_000), 3)
		);
		assert_eq!(
			BiddingQueues::<Test>::get(vdot_id).get(4).unwrap(),
			&(Permill::from_parts(90 * 10_000), 4)
		);
	});
}

#[test]
fn cancel_a_bidding_proposal_should_work() {
	new_test_ext().execute_with(|| {
		// initialization
		storages_initialization();

		let alice = 1;
		let bob = 2;
		let vksm_id = 5;
		let origin_bob = Origin::signed(bob);
		let origin_alice = Origin::signed(alice);
		// make sure bob's proposal exists
		assert_eq!(
			BidderProposalInQueue::<Test>::get(bob, vksm_id).contains(&0),
			true
		);

		// proposal not exist
		assert_eq!(
			Bid::cancel_a_bidding_proposal(origin_bob.clone(), 8),
			Err(DispatchError::Module {
				index: 0,
				error: 16,
				message: Some("ProposalNotExist")
			})
		);

		// cancel the proposal
		assert_eq!(
			Bid::cancel_a_bidding_proposal(origin_alice.clone(), 0),
			Err(DispatchError::Module {
				index: 0,
				error: 17,
				message: Some("NotProposalOwner",),
			},)
		); // not the owner

		assert_ok!(Bid::cancel_a_bidding_proposal(origin_bob.clone(), 0));
		assert_eq!(
			BidderProposalInQueue::<Test>::get(bob, vksm_id).contains(&0),
			false
		);
	});
}

#[test]
fn check_overall_proposal_matching_to_orders_should_work() {
	new_test_ext().execute_with(|| {
		// initialization
		storages_initialization();

		run_to_block(3); // 4000 vtoken votes are supplied

		// Bob 要有200 票vksm的订单，池子里还剩 3800票
		// There was an vksm order of 200 votes from Bob in the storage initialization. So the pool should has only 3800
		// votes left and Bob should have a 200 vksm order.Decode
		let bob = 2;
		let vksm_id = 5;
		// BiddingQueues::<Test>::get(vksm_id)
		// assert_eq!(ProposalsInQueue::<Test>::get(0).votes, 3800);

		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(bob, vksm_id).len(),
			1
		);

		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(bob, vksm_id).contains(&0),
			true
		);

		// 总服务票数改了，对
		assert_eq!(TotalVotesInService::<Test>::get(vksm_id), 200);

		// // 维护列表有点问题
		// assert_eq!(
		//	 TokenOrderROIList::<Test>::get(vksm_id).len(),
		//	 1
		// );

		// 挂单已经不存在了，是对的
		assert_eq!(BiddingQueues::<Test>::contains_key(0), false);

		// 竞拍订单队列也已经没有了作何挂单，是对的
		assert_eq!(BiddingQueues::<Test>::get(vksm_id).len(), 0);

		assert_eq!(BiddingQueues::<Test>::get(vksm_id).is_empty(), true);

		assert_eq!(ProposalsInQueue::<Test>::contains_key(0), false);

		assert_eq!(
			BidderProposalInQueue::<Test>::get(bob, vksm_id).contains(&0),
			false
		);

		assert_eq!(TotalProposalsInQueue::<Test>::get(vksm_id), 0);

		assert_eq!(OrdersInService::<Test>::get(0).votes, 200);

		// Alice再下一个 4500票 vksm订章，只有 3800票成交，剩下 700票依然挂在上面

		// 30天后， Bob的票到期自动删除。释放出订单。Alice的剩余订单是否会自动成交200

		// 看是否成功匹配订单，各storage数字是否正确

		// 如果用户抽走vtoken，池子里不够的话，是否会自动拆单，其中一单撤走，另一单是否还留在那里。释放出来后，这些空票是否被保留着，还是又被匹配掉了？

		// 订单到期会不会被自动删除，并释放出票出来，是否还会重新匹配

		run_to_block(5);

		// 订单0是到期区块是5，订单1到期区块是21
		assert_eq!(OrdersInService::<Test>::contains_key(1), false); // order1已经被马上release了
		assert_eq!(OrdersInService::<Test>::get(2).votes, 5); // order2是保留的不变

		assert_eq!(OrdersInService::<Test>::contains_key(0), false); // 原订单已经结束,已经不在订单列表里了
		assert_eq!(OrdersInService::<Test>::get(2).block_num, 21); // 创建时间1+21-1= 21，不受影响订单保留原来的结束时间

		// Bidder Bob有两个订单
		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(bob, vksm_id).len(),
			1
		);
		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(bob, vksm_id).contains(&1),
			false
		); // 订单1已经被结束掉了
		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(bob, vksm_id).contains(&2),
			true
		);

		// TokenOrderROIList里边应该只有订单2
		assert_eq!(TokenOrderROIList::<Test>::get(vksm_id).len(), 1);
		assert_eq!(
			TokenOrderROIList::<Test>::get(vksm_id)
				.contains(&(Permill::from_parts(60 * 10_000), 1)),
			false
		);
		assert_eq!(
			TokenOrderROIList::<Test>::get(vksm_id)
				.contains(&(Permill::from_parts(60 * 10_000), 2)),
			true
		);

		// 订单剩下5 votes，已强行释放出195个votes
		assert_eq!(TotalVotesInService::<Test>::get(vksm_id), 5);

		// 本era(era 0)会有195张票强制性到期
		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 0)), 0);
		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 3)), 5); // era 3有5票到期要放

		// 在第五个区块不够用户抽走，拆单了，拆成了 195和5两个订单，加上最早的订单号，一共是3个订单，对
		assert_eq!(OrderNextId::<Test>::get(), 3);

		// 本区块ForciblyUnbondOrdersInCurrentEra应该有一个数量为195的订单1记录了强制结束，强制结束区块为5,但在等待复活列表里的区块记录为原来的区块数字21
		assert_eq!(
			ForciblyUnbondOrdersInCurrentEra::<Test>::get(vksm_id).len(),
			1
		);
		assert_eq!(
			ForciblyUnbondOrdersInCurrentEra::<Test>::get(vksm_id)[0]
				.1
				.votes,
			195
		);
		assert_eq!(
			ForciblyUnbondOrdersInCurrentEra::<Test>::get(vksm_id)[0]
				.1
				.block_num,
			21
		);

		run_to_block(6); // 区块6供应 206个订单，数量足够，bob的订单1应该恢复成结束时间为 21

		assert_eq!(OrdersInService::<Test>::contains_key(1), false); // 原来的订单1已经在区块5结束的时候强行结束了

		assert_eq!(OrdersInService::<Test>::contains_key(3), true); // 但因为区块6又有多余的votes出现，已经强行删除的订单重新复活，得到一个新的订单号3
		assert_eq!(OrdersInService::<Test>::get(3).votes, 195); // 新订单的票数为195
		assert_eq!(OrdersInService::<Test>::get(3).block_num, 21); // 新订单的到期时间为第21区块

		assert_eq!(
			ForciblyUnbondOrdersInCurrentEra::<Test>::get(vksm_id).len(),
			0
		); // 已经复活了，所以没有这条记录了

		assert_eq!(OrderEndBlockNumMap::<Test>::get(21).len(), 2);
		assert_eq!(OrderEndBlockNumMap::<Test>::get(21).contains(&2), true);
		assert_eq!(OrderEndBlockNumMap::<Test>::get(21).contains(&3), true);

		// Bidder Bob有两个订单2和3
		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(bob, vksm_id).len(),
			2
		);
		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(bob, vksm_id).contains(&2),
			true
		);
		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(bob, vksm_id).contains(&3),
			true
		);

		// TokenOrderROIList里边应该有两条订单，订单2和复活订单3
		assert_eq!(TokenOrderROIList::<Test>::get(vksm_id).len(), 2);
		assert_eq!(
			TokenOrderROIList::<Test>::get(vksm_id)
				.contains(&(Permill::from_parts(60 * 10_000), 2)),
			true
		);
		assert_eq!(
			TokenOrderROIList::<Test>::get(vksm_id)
				.contains(&(Permill::from_parts(60 * 10_000), 3)),
			true
		);

		assert_eq!(TotalVotesInService::<Test>::get(vksm_id), 200); // 在服务的票权有200个
		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 0)), 0); // 本era没有票要释放
		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 3)), 200); // era 3有200票到期要放
		assert_eq!(OrderNextId::<Test>::get(), 4); // 复活订单用了订单号3
		assert_eq!(
			ForciblyUnbondOrdersInCurrentEra::<Test>::get(vksm_id).len(),
			0
		); // 已经没有强行关闭订单了

		run_to_block(10); // 只有10票available
		assert_eq!(TotalVotesInService::<Test>::get(vksm_id), 10);

		assert_eq!(OrderNextId::<Test>::get(), 6);
		assert_eq!(OrdersInService::<Test>::contains_key(5), true); // 但因为区块6又有多余的votes出现，已经强行删除的订单重新复活，得到一个新的订单号3
		assert_eq!(OrdersInService::<Test>::get(5).votes, 5); // 新订单的票数为190
		assert_eq!(OrdersInService::<Test>::get(5).block_num, 21); // 新订单的到期时间为第21区块

		assert_eq!(OrdersInService::<Test>::contains_key(2), true);
		assert_eq!(OrdersInService::<Test>::get(2).votes, 5);
		assert_eq!(OrdersInService::<Test>::get(2).block_num, 21);

		assert_eq!(
			ForciblyUnbondOrdersInCurrentEra::<Test>::get(vksm_id).len(),
			1
		);
		assert_eq!(
			ForciblyUnbondOrdersInCurrentEra::<Test>::get(vksm_id)[0]
				.1
				.votes,
			190
		);
		assert_eq!(
			ForciblyUnbondOrdersInCurrentEra::<Test>::get(vksm_id)[0]
				.1
				.block_num,
			21
		);

		assert_eq!(TokenOrderROIList::<Test>::get(vksm_id).len(), 2);
		assert_eq!(
			TokenOrderROIList::<Test>::get(vksm_id)
				.contains(&(Permill::from_parts(60 * 10_000), 2)),
			true
		);
		assert_eq!(
			TokenOrderROIList::<Test>::get(vksm_id)
				.contains(&(Permill::from_parts(60 * 10_000), 5)),
			true
		);

		// Bidder Bob有两个订单2和5
		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(bob, vksm_id).len(),
			2
		);
		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(bob, vksm_id).contains(&2),
			true
		);
		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(bob, vksm_id).contains(&5),
			true
		);

		assert_eq!(OrderEndBlockNumMap::<Test>::get(21).len(), 2);
		assert_eq!(OrderEndBlockNumMap::<Test>::get(21).contains(&2), true);
		assert_eq!(OrderEndBlockNumMap::<Test>::get(21).contains(&5), true);

		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 0)), 0); // 本era没有票要释放
		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 3)), 10); // era 3有10票到期要放

		run_to_block(11); // 有211票available
		assert_eq!(TotalVotesInService::<Test>::get(vksm_id), 200);
		assert_eq!(
			ForciblyUnbondOrdersInCurrentEra::<Test>::get(vksm_id).len(),
			0
		);

		assert_eq!(OrderNextId::<Test>::get(), 7);
		assert_eq!(OrdersInService::<Test>::contains_key(2), true);
		assert_eq!(OrdersInService::<Test>::contains_key(5), true);
		assert_eq!(OrdersInService::<Test>::contains_key(6), true);
		assert_eq!(OrdersInService::<Test>::get(6).votes, 190); // 新订单的票数为190
		assert_eq!(OrdersInService::<Test>::get(6).block_num, 21); // 新订单的到期时间为第21区块

		assert_eq!(TokenOrderROIList::<Test>::get(vksm_id).len(), 3);
		assert_eq!(
			TokenOrderROIList::<Test>::get(vksm_id)
				.contains(&(Permill::from_parts(60 * 10_000), 2)),
			true
		);
		assert_eq!(
			TokenOrderROIList::<Test>::get(vksm_id)
				.contains(&(Permill::from_parts(60 * 10_000), 5)),
			true
		);
		assert_eq!(
			TokenOrderROIList::<Test>::get(vksm_id)
				.contains(&(Permill::from_parts(60 * 10_000), 6)),
			true
		);

		// Bidder Bob有两个订单2\5\6
		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(bob, vksm_id).len(),
			3
		);
		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(bob, vksm_id).contains(&2),
			true
		);
		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(bob, vksm_id).contains(&5),
			true
		);
		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(bob, vksm_id).contains(&6),
			true
		);

		assert_eq!(OrderEndBlockNumMap::<Test>::get(21).len(), 3);
		assert_eq!(OrderEndBlockNumMap::<Test>::get(21).contains(&2), true);
		assert_eq!(OrderEndBlockNumMap::<Test>::get(21).contains(&5), true);
		assert_eq!(OrderEndBlockNumMap::<Test>::get(21).contains(&6), true);

		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 0)), 0); // 本era没有票要释放
		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 3)), 200); // era 3有10票到期要放

		// Alice再下一单100，因为本期一共211票，bob用了200，alice只有11票，检验一下是否会拆票
		// create a proposal for alice
		let alice = 1;
		let origin_alice = Origin::signed(alice);
		let votes_needed_alice = 100;
		let annual_roi_alice = 8000;
		let validator_alice = alice;
		assert_ok!(Bid::create_bidding_proposal(
			origin_alice,
			vksm_id,
			votes_needed_alice,
			annual_roi_alice,
			validator_alice,
		));

		// Alice下的订单被拆单了，还剩89votes需求挂在上面
		assert_eq!(BiddingQueues::<Test>::get(vksm_id).len(), 1);
		assert_eq!(
			BiddingQueues::<Test>::get(vksm_id).contains(&(Permill::from_parts(80 * 10_000), 1)),
			true
		);

		let (_ord_roi, ord_id) = BiddingQueues::<Test>::get(vksm_id)[0];
		assert_eq!(ProposalsInQueue::<Test>::get(ord_id).votes, 89);

		assert_eq!(TotalVotesInService::<Test>::get(vksm_id), 211);
		assert_eq!(
			ForciblyUnbondOrdersInCurrentEra::<Test>::get(vksm_id).len(),
			0
		);
		assert_eq!(OrderNextId::<Test>::get(), 8);
		assert_eq!(OrdersInService::<Test>::contains_key(2), true);
		assert_eq!(OrdersInService::<Test>::contains_key(5), true);
		assert_eq!(OrdersInService::<Test>::contains_key(6), true);
		assert_eq!(OrdersInService::<Test>::contains_key(7), true);
		assert_eq!(OrdersInService::<Test>::get(7).votes, 11); // 新订单的票数为190
		assert_eq!(OrdersInService::<Test>::get(7).block_num, 31); // 新订单的到期时间为第31区块

		assert_eq!(TokenOrderROIList::<Test>::get(vksm_id).len(), 4);
		assert_eq!(
			TokenOrderROIList::<Test>::get(vksm_id)
				.contains(&(Permill::from_parts(60 * 10_000), 2)),
			true
		);
		assert_eq!(
			TokenOrderROIList::<Test>::get(vksm_id)
				.contains(&(Permill::from_parts(60 * 10_000), 5)),
			true
		);
		assert_eq!(
			TokenOrderROIList::<Test>::get(vksm_id)
				.contains(&(Permill::from_parts(60 * 10_000), 6)),
			true
		);
		assert_eq!(
			TokenOrderROIList::<Test>::get(vksm_id)
				.contains(&(Permill::from_parts(80 * 10_000), 7)),
			true
		);

		// 测alice的新order在TokenOrderROIList里的排序
		assert_eq!(TokenOrderROIList::<Test>::get(vksm_id)[3].1, 7);

		// Bidder Bob有3个订单，alice有1个订单
		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(bob, vksm_id).len(),
			3
		);
		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(alice, vksm_id).len(),
			1
		);
		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(alice, vksm_id).contains(&7),
			true
		);

		assert_eq!(OrderEndBlockNumMap::<Test>::get(21).len(), 3);
		assert_eq!(OrderEndBlockNumMap::<Test>::get(31).len(), 1);
		assert_eq!(OrderEndBlockNumMap::<Test>::get(31).contains(&7), true);

		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 0)), 0); // 本era没有票要释放
		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 3)), 200); // era 3有200票到期要放
		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 4)), 11); // era 4有11票到期要放

		// Bob再下一单150, roi 70%, proposal_id = 2
		// create a proposal for Bob
		let origin_bob = Origin::signed(bob);
		let votes_needed_bob = 150;
		let annual_roi_bob = 7000;
		let validator_bob = bob;
		assert_ok!(Bid::create_bidding_proposal(
			origin_bob,
			vksm_id,
			votes_needed_bob,
			annual_roi_bob,
			validator_bob,
		));

		// Alice再下一单200, roi 90%, proposal_id = 3
		// create a proposal for Alice
		let origin_alice = Origin::signed(alice);
		let votes_needed_alice = 200;
		let annual_roi_alice = 9000;
		let validator_alice = alice;
		assert_ok!(Bid::create_bidding_proposal(
			origin_alice,
			vksm_id,
			votes_needed_alice,
			annual_roi_alice,
			validator_alice,
		));

		// 先看proposal挂单队列
		assert_eq!(BiddingQueues::<Test>::get(vksm_id).len(), 3);
		assert_eq!(BiddingQueues::<Test>::get(vksm_id)[2].1, 3); // 排序是升序排列的，排序最后的订单应该是alice roi为 90%的proposal
		assert_eq!(BiddingQueues::<Test>::get(vksm_id)[1].1, 1); // 倒数第二是 alice roi 为 80%的proposal
		assert_eq!(BiddingQueues::<Test>::get(vksm_id)[0].1, 2); // 倒数第三是 bob roi 为 70%的proposal

		run_to_block(12); // 区块12释放 412个votes，看看是否优先成交最高roi的订单
		assert_eq!(BiddingQueues::<Test>::get(vksm_id).len(), 2); // 剩下2个挂单。Bob roi 70%的 150票 order 2，和 alice roi 80%的 88票 order 1
		assert_eq!(BiddingQueues::<Test>::get(vksm_id)[1].1, 1);
		assert_eq!(BiddingQueues::<Test>::get(vksm_id)[0].1, 2);

		assert_eq!(TotalVotesInService::<Test>::get(vksm_id), 412);
		assert_eq!(OrdersInService::<Test>::get(8).votes, 200); // 第8个订单是alice roi为90%的200票订单
		assert_eq!(
			OrdersInService::<Test>::get(8).annual_roi,
			Permill::from_parts(90 * 10_000)
		);
		assert_eq!(OrdersInService::<Test>::get(9).votes, 1); // 第9个订单是alice roi为80%的1票订单
		assert_eq!(
			OrdersInService::<Test>::get(9).annual_roi,
			Permill::from_parts(80 * 10_000)
		); // 第9个订单是alice roi为80%的1票订单
	});
}
