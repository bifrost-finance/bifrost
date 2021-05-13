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
	<Test as Config>::AssetTrait::asset_create(symbol0, precision0, token_type0).unwrap_or_default();

	let symbol1 = b"aUSD".to_vec();
	let precision1 = 12;
	let token_type1 = TokenType::Stable;
	<Test as Config>::AssetTrait::asset_create(symbol1, precision1, token_type1).unwrap_or_default();

	let symbol2 = b"DOT".to_vec();
	let precision2 = 12;
	<Test as Config>::AssetTrait::asset_create_pair(symbol2, precision2).unwrap_or_default();

	let symbol3 = b"KSM".to_vec();
	let precision3 = 12;
	<Test as Config>::AssetTrait::asset_create_pair(symbol3, precision3).unwrap_or_default();

	let symbol4 = b"EOS".to_vec();
	let precision4 = 12;
	<Test as Config>::AssetTrait::asset_create_pair(symbol4, precision4).unwrap_or_default();

	let symbol5 = b"IOST".to_vec();
	let precision5 = 12;
	<Test as Config>::AssetTrait::asset_create_pair(symbol5, precision5).unwrap_or_default();

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
	<Test as Config>::AssetTrait::asset_issue(dot_id, &alice, amount);
	<Test as Config>::AssetTrait::asset_issue(ksm_id, &alice, amount);
	<Test as Config>::AssetTrait::asset_issue(eos_id, &alice, amount);
	<Test as Config>::AssetTrait::asset_issue(iost_id, &alice, amount);

	let amount = 10_000;
	// create some assets for bidder Bob
	<Test as Config>::AssetTrait::asset_issue(dot_id, &bob, amount);
	<Test as Config>::AssetTrait::asset_issue(ksm_id, &bob, amount);
	<Test as Config>::AssetTrait::asset_issue(eos_id, &bob, amount);
	<Test as Config>::AssetTrait::asset_issue(iost_id, &bob, amount);

	let amount = 100;
	// create some assets for bidder Charlie
	<Test as Config>::AssetTrait::asset_issue(dot_id, &charlie, amount);
	<Test as Config>::AssetTrait::asset_issue(ksm_id, &charlie, amount);
	<Test as Config>::AssetTrait::asset_issue(eos_id, &charlie, amount);
	<Test as Config>::AssetTrait::asset_issue(iost_id, &charlie, amount);

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
			<Test as Config>::AssetTrait::asset_create_pair(symbol, precision).unwrap_or_default();

		assert_eq!(<Test as Config>::AssetTrait::is_token(token_id), true);
		assert_eq!(<Test as Config>::AssetTrait::is_v_token(vtoken_id), true);

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
				index: 2,
				error: 1,
				message: Some("NotValidVtoken")
			})
		);

		// repeatedly registering the same vtoken is not allowed
		assert_eq!(
			Bid::register_vtoken_for_bidding(origin_root.clone(), vtoken_id),
			Err(DispatchError::Module {
				index: 2,
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
			<Test as Config>::AssetTrait::asset_create_pair(symbol, precision).unwrap_or_default();

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
				index: 2,
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
				index: 2,
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
			<Test as Config>::AssetTrait::asset_create_pair(symbol, precision).unwrap_or_default();

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
				index: 2,
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
			<Test as Config>::AssetTrait::asset_create_pair(symbol, precision).unwrap_or_default();

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
				index: 2,
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
			<Test as Config>::AssetTrait::asset_create_pair(symbol, precision).unwrap_or_default();

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
				index: 2,
				error: 15,
				message: Some("VtokenNotRegistered")
			})
		);

		assert_eq!(
			Bid::set_slash_margin_rates(origin_root.clone(), vtoken_id, 200),
			Err(DispatchError::Module {
				index: 2,
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
				index: 2,
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
				index: 2,
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
				index: 2,
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
				index: 2,
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
				index: 2,
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
				index: 2,
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
				index: 2,
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
				index: 2,
				error: 16,
				message: Some("ProposalNotExist")
			})
		);

		// cancel the proposal
		assert_eq!(
			Bid::cancel_a_bidding_proposal(origin_alice.clone(), 0),
			Err(DispatchError::Module {
				index: 2,
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

		assert_eq!(TotalVotesInService::<Test>::get(vksm_id), 200);
		assert_eq!(BiddingQueues::<Test>::contains_key(0), false);
		assert_eq!(BiddingQueues::<Test>::get(vksm_id).len(), 0);
		assert_eq!(BiddingQueues::<Test>::get(vksm_id).is_empty(), true);
		assert_eq!(ProposalsInQueue::<Test>::contains_key(0), false);
		assert_eq!(
			BidderProposalInQueue::<Test>::get(bob, vksm_id).contains(&0),
			false
		);
		assert_eq!(TotalProposalsInQueue::<Test>::get(vksm_id), 0);
		assert_eq!(OrdersInService::<Test>::get(0).votes, 200);

		run_to_block(5);

		// order 0 will be released by the end of block 5, while order 1 will be released by the end of block 21
		assert_eq!(OrdersInService::<Test>::contains_key(1), false);
		assert_eq!(OrdersInService::<Test>::get(2).votes, 5);

		assert_eq!(OrdersInService::<Test>::contains_key(0), false);
		assert_eq!(OrdersInService::<Test>::get(2).block_num, 21);

		// Bidder Bob has two orders
		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(bob, vksm_id).len(),
			1
		);
		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(bob, vksm_id).contains(&1),
			false
		); // order 1 is finished.
		assert_eq!(
			BidderTokenOrdersInService::<Test>::get(bob, vksm_id).contains(&2),
			true
		);

		// TokenOrderROIList has only order 2 in it.
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

		// order has only 5 votes left, while 195 votes are forcibly released.
		assert_eq!(TotalVotesInService::<Test>::get(vksm_id), 5);

		// there will be 195 votes forcibly released by the end of era 0.
		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 0)), 0);
		// There will be 5 votes release by the end of era 3.
		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 3)), 5);
		assert_eq!(OrderNextId::<Test>::get(), 3);
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

		run_to_block(6);

		assert_eq!(OrdersInService::<Test>::contains_key(1), false);
		assert_eq!(OrdersInService::<Test>::contains_key(3), true);
		assert_eq!(OrdersInService::<Test>::get(3).votes, 195);=
		assert_eq!(OrdersInService::<Test>::get(3).block_num, 21);

		assert_eq!(
			ForciblyUnbondOrdersInCurrentEra::<Test>::get(vksm_id).len(),
			0
		);

		assert_eq!(OrderEndBlockNumMap::<Test>::get(21).len(), 2);
		assert_eq!(OrderEndBlockNumMap::<Test>::get(21).contains(&2), true);
		assert_eq!(OrderEndBlockNumMap::<Test>::get(21).contains(&3), true);

		// Bidder Bob orders of 2 and 3.
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

		// TokenOrderROIList should have to orders，order 2 and revival order 3
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

		assert_eq!(TotalVotesInService::<Test>::get(vksm_id), 200);
		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 0)), 0);
		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 3)), 200);
		assert_eq!(OrderNextId::<Test>::get(), 4);
		assert_eq!(
			ForciblyUnbondOrdersInCurrentEra::<Test>::get(vksm_id).len(),
			0
		);

		run_to_block(10);
		assert_eq!(TotalVotesInService::<Test>::get(vksm_id), 10);

		assert_eq!(OrderNextId::<Test>::get(), 6);
		// block 6 has extra votes which enables the forced closed order to revive and gets a new order number of 5.
		assert_eq!(OrdersInService::<Test>::contains_key(5), true);
		assert_eq!(OrdersInService::<Test>::get(5).votes, 5);
		assert_eq!(OrdersInService::<Test>::get(5).block_num, 21);

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

		// Bidder Bob orders of 2 and 5
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

		// no votes will be released by the end of current era
		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 0)), 0); 
		// 10 votes will be released by the end of era 3
		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 3)), 10);

		run_to_block(11); // 211 votes are available
		assert_eq!(TotalVotesInService::<Test>::get(vksm_id), 200);
		assert_eq!(
			ForciblyUnbondOrdersInCurrentEra::<Test>::get(vksm_id).len(),
			0
		);

		assert_eq!(OrderNextId::<Test>::get(), 7);
		assert_eq!(OrdersInService::<Test>::contains_key(2), true);
		assert_eq!(OrdersInService::<Test>::contains_key(5), true);
		assert_eq!(OrdersInService::<Test>::contains_key(6), true);
		// order 6 has 190 votes
		assert_eq!(OrdersInService::<Test>::get(6).votes, 190);
		 // order 6 will release all the votes by the end of block 21
		assert_eq!(OrdersInService::<Test>::get(6).block_num, 21);

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

		// Bidder Bob has orders of 2\5\6
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

		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 0)), 0);
		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 3)), 200);

		// Alice make one more order of 100 votes. Since there will be only 211 votes
		// available in this era. Bob has taken up 200 votes with only 11 votes
		// left for Alice
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

		// Alice's order is split, with only 89 votes left for sale.
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
		// the new order has 190 votes.
		assert_eq!(OrdersInService::<Test>::get(7).votes, 11);
		// the new order will be released by the end of block 31.
		assert_eq!(OrdersInService::<Test>::get(7).block_num, 31);

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

		assert_eq!(TokenOrderROIList::<Test>::get(vksm_id)[3].1, 7);

		// Bidder Bob has 3 orders，alice has only 1 order.
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

		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 0)), 0);
		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 3)), 200);
		assert_eq!(ToReleaseVotesTilEndOfEra::<Test>::get((vksm_id, 4)), 11);

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

		// Alice makes one more order of 200 votes, roi 90%, proposal_id = 3
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

		assert_eq!(BiddingQueues::<Test>::get(vksm_id).len(), 3);
		// the queue is ordered by ascending. So the last order should be alice's proposal of roi 90%
		assert_eq!(BiddingQueues::<Test>::get(vksm_id)[2].1, 3); 
		// second from the last should be alice's proposal of roi 80%.
		assert_eq!(BiddingQueues::<Test>::get(vksm_id)[1].1, 1);
		// second from the last should be bob's proposal of roi 70%.
		assert_eq!(BiddingQueues::<Test>::get(vksm_id)[0].1, 2);

		run_to_block(12);
		 // 2 orders left。Bob's order 2 of 150 votes with roi 70% 150 and  alice's order 1 of 88 votes with roi 80%.
		assert_eq!(BiddingQueues::<Test>::get(vksm_id).len(), 2);
		assert_eq!(BiddingQueues::<Test>::get(vksm_id)[1].1, 1);
		assert_eq!(BiddingQueues::<Test>::get(vksm_id)[0].1, 2);
		// order 8 is from alice's votes order, which has an roi of 90%
		assert_eq!(TotalVotesInService::<Test>::get(vksm_id), 412);

		assert_eq!(OrdersInService::<Test>::get(8).votes, 200); // 
		assert_eq!(
			OrdersInService::<Test>::get(8).annual_roi,
			Permill::from_parts(90 * 10_000)
		);
		assert_eq!(OrdersInService::<Test>::get(9).votes, 1);
		assert_eq!(
			OrdersInService::<Test>::get(9).annual_roi,
			Permill::from_parts(80 * 10_000)
		); // order 9 is from alice's 1 vote order, which has an roi of 80%.
	});
}
