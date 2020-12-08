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
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::vec::Vec;

mod mock;
mod tests;

use codec::{Decode, Encode};
use frame_support::traits::Get;
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure,
	weights::Weight, Parameter, StorageValue,
};
use frame_system::{ensure_root, ensure_signed};
use node_primitives::AssetTrait;
use sp_runtime::traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member, Saturating, Zero};

pub trait Trait: frame_system::Trait {
	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	/// The units in which we record costs.
	type Cost: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	/// The units in which we record incomes.
	type Income: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	type AssetTrait: AssetTrait<Self::AssetId, Self::AccountId, Self::Balance>;

	/// Bidding order id.
	type BiddingOrderId: Member
		+ Parameter
		+ AtLeast32Bit
		+ Default
		+ Copy
		+ MaybeSerializeDeserialize;

	/// Era id
	type EraId: Member
		+ Parameter
		+ AtLeast32Bit
		+ Default
		+ Copy
		+ MaybeSerializeDeserialize
		+ From<Self::BlockNumber>
		+ Into<Self::BlockNumber>;

	/// event
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	/// The units in which we record balances.
	type Balance: Member
		+ Parameter
		+ AtLeast32Bit
		+ Default
		+ Copy
		+ MaybeSerializeDeserialize
		+ From<Self::BlockNumber>
		+ Into<Self::BlockNumber>;

	/// the number of records that the order roi list should keep
	type TokenOrderROIListLength: Get<u8>;

	/// Rate precision
	type BidRatePrecision: Get<Self::Balance>;

	/// the minimum number of votes for a bidding proposal
	type MinimumVotes: Get<Self::Balance>;

	/// the maximum number of votes for a bidding proposal to prevent from attack
	type MaximumVotes: Get<Self::Balance>;

	/// how many blocks per year
	type BlocksPerYear: Get<Self::BlockNumber>;

	/// the maximum proposals in queue for a bidder
	type MaxProposalNumberForBidder: Get<u32>;
}

decl_event! {
	pub enum Event<T> where
		AssetId = <T as Trait>::AssetId,
		BlockNumber = <T as frame_system::Trait>::BlockNumber,
		BiddingOrderId = <T as Trait>::BiddingOrderId,
		Balance = <T as Trait>::Balance,
		{
			SetOrderEndTimeSuccess(BiddingOrderId, BlockNumber),
			CreateProposalSuccess,
			VtokenRegisterSuccess(AssetId),
			SetMinMaxOrderLastingBlockNumSuccess(AssetId, BlockNumber, BlockNumber),
			SetBlockNumberPerEraSuccess(AssetId, BlockNumber),
			SetServiceStopBlockNumLagSuccess(AssetId, BlockNumber),
			SetSlashMarginRatesSuccess(AssetId, Balance),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		TokenNotExist,
		NotValidVtoken,
		NotEnoughBalance,
		OrderNotExist,
		AmountNotAboveZero,
		VotesExceedLowerBound,
		VotesExceedUpperBound,
		BlockNumberNotValid,
		RateExceedUpperBound,
		MinimumOrMaximumNotRight,
		VtokenBlockNumberPerEraNotSet,
		MinMaxOrderLastingBlockNumNotSet,
		SlashMarginRatesNotSet,
		ServiceStopBlockNumLagNotSet,
		VtokenAlreadyRegistered,
		VtokenNotRegistered,
		ProposalNotExist,
		NotProposalOwner,
		ProposalsExceedLimit,
	}
}

/// struct for matched order in service
#[derive(Default, Clone, Eq, PartialEq, Debug, Encode, Decode)]
pub struct BiddingOrderUnit<AccountId, AssetId, BlockNumber, Balance> {
	/// bidder id
	bidder_id: AccountId,
	/// token id
	token_id: AssetId,
	/// if it's a bidding proposal unit, then block_num means bidding block number.
	/// If it's an order in service unit, then block_num means order end block number.
	block_num: BlockNumber,
	/// if it's a bidding proposal unit, then votes field means number of votes that the bidder wants to bid for
	/// If it's an order in service unit, then votes field means the votes in service.
	votes: Balance,
	/// the annual rate of return that the bidder provides to the vtoken holder
	annual_roi: Balance,
	/// the validator address that these votes will goes to
	validator: AccountId,
}

decl_storage! {
	trait Store for Module<T: Trait> as Bid {
		/// queue for unmatched bidding proposals
		BiddingQueues get(fn bidding_queues): map hasher(blake2_128_concat) T::AssetId
						=> Vec<(T::Balance, T::BiddingOrderId)>;
		/// proposal Id
		ProposalNextId get(fn proposal_next_id): T::BiddingOrderId;
		/// Proposals map, recording all the proposals. key is id, value is proposal detail.
		ProposalsInQueue get(fn proposals_in_queue): map hasher(blake2_128_concat) T::BiddingOrderId
							=> BiddingOrderUnit<T::AccountId, T::AssetId, T::BlockNumber, T::Balance>;
		/// Bidder proposals in queue which haven't been matched.
		BidderProposalInQueue get(fn bidder_proposal_in_queue): double_map
			hasher(blake2_128_concat) T::AccountId,
			hasher(blake2_128_concat) T::AssetId
			=> Vec<T::BiddingOrderId>;
		/// the bidding balance of each registered vtoken.
		TotalProposalsInQueue get(fn total_proposals_in_queue): map hasher(blake2_128_concat) T::AssetId => T::Balance;
		/// map for recording orders in service. key is id, value is BiddingOrderUnit struct.
		OrdersInService get(fn orders_in_service): map hasher(blake2_128_concat) T::BiddingOrderId
													=> BiddingOrderUnit<T::AccountId, T::AssetId, T::BlockNumber, T::Balance>;
		/// Recording the orders in service ids for every end block number.
		OrderEndBlockNumMap get(fn order_end_block_num_map): map hasher(blake2_128_concat) T::BlockNumber
																=> Vec<T::BiddingOrderId>;
		/// Record bidder token orders in service in the form of id in a map.
		BidderTokenOrdersInService get(fn bidder_token_orders_in_service): double_map
			hasher(blake2_128_concat) T::AccountId,
			hasher(blake2_128_concat) T::AssetId
			=> Vec<T::BiddingOrderId>;
		/// maintain a list of order id for each token in the order of ROI increasing. Every Vec constrain to a constant length
		/// token => (annual roi, order id), order by annual roi ascending.
		TokenOrderROIList get(fn token_order_roi_list): map hasher(blake2_128_concat) T::AssetId
															 => Vec<(T::Balance, T::BiddingOrderId)>;
		/// total votes which are already in service
		TotalVotesInService get(fn total_votes_in_service): map hasher(blake2_128_concat) T::AssetId => T::Balance;
		/// Record the releasing votes from now to the end of current era.
		ToReleaseVotesTilEndOfEra get(fn to_release_votes_til_end_of_era): map hasher(blake2_128_concat)
																				(T::AssetId, T::EraId) => T::Balance;
		/// Order next id
		OrderNextId get(fn order_next_id): T::BiddingOrderId;
		/// the min and max number of blocks that an matched order can last. 【token => (min, max)】
		MinMaxOrderLastingBlockNum get(fn max_order_lasting_block_num): map hasher(blake2_128_concat) T::AssetId
		=> (T::BlockNumber, T::BlockNumber);
		/// slash margin rates for each type of token
		SlashMarginRates get(fn slash_margin_rates): map hasher(blake2_128_concat) T::AssetId => T::Balance;
		/// Block number per era for each vtoken
		BlockNumberPerEra get(fn block_number_per_era): map hasher(blake2_128_concat) T::AssetId => T::BlockNumber;
		/// the block number lag before we can vote for another validator when we stop a staking
		ServiceStopBlockNumLag get(fn service_stop_block_num_lag): map hasher(blake2_128_concat) T::AssetId => T::BlockNumber;
		/// vtokens that have been registered for bidding marketplace
		VtokensRegisteredForBidding get(fn vtoken_registered_for_bidding): Vec<T::AssetId>;


		// **********************************************************************************************************
		// Below storage should be called by other pallets to update data, and then used by this bid pallet.       //
		// **********************************************************************************************************
		/// Slash amounts for orders in service. This storage should be updated by the Staking pallet whenever there is
		/// slash occurred for a certain order. When the order ends, remaining slash deposit should be return to the
		/// bidder and the record in this storage should be deleted.
		SlashForOrdersInService get(fn slash_for_orders_in_service): map hasher(blake2_128_concat) T::BiddingOrderId
																		=> T::Balance;
		/// Record the reserved votes for users to withdraw at the end of this era. Whenever a user initiate a withdrawing,
		/// a record should be added here to preserve token amount to the end of the era to be withdrew.
		WithdrawReservedVotes get(fn withdraw_reserved_votes): map hasher(blake2_128_concat) T::AssetId => T::Balance;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;
		const TokenOrderROIListLength: u8 = T::TokenOrderROIListLength::get();
		const BidRatePrecision: T::Balance = T::BidRatePrecision::get();
		const MinimumVotes: T::Balance = T::MinimumVotes::get();
		const MaximumVotes: T::Balance = T::MaximumVotes::get();
		const BlocksPerYear: T::BlockNumber = T::BlocksPerYear::get();
		const MaxProposalNumberForBidder: u32 = T::MaxProposalNumberForBidder::get();

		fn deposit_event() = default;

		// ****************************************************************************************
		//  注意，用户如果要提币，把vtoken转换成token的话，需要等到本era结束之后，金额才能准备好，才能开始流程。//
		// ****************************************************************************************
		/// What on_initialize function does?
		/// 1. query for current available votes.
		/// 2. compare available votes and total votes in service for each vtoken. If available votes are less than total
		/// 	votes in service, release the difference from the bidder who provides the lowest roi rate.
		/// 3. check if there is unsatisfied bidding proposal. If yes, match it with available votes.
		fn on_initialize(n: T::BlockNumber) -> Weight {

			let vtoken_list = VtokensRegisteredForBidding::<T>::get();
			for vtoken in vtoken_list.iter() {
				// We compare only one storage to see it the token has been initialized. If not, do it.
				if !TotalVotesInService::<T>::contains_key(&vtoken) {
					if let Err(_rs) = Self::vtoken_empty_storage_initialization(vtoken.clone()){
						return 0;
					};
				}

				if let Ok((available_flag, available_votes)) = Self::calculate_available_votes(vtoken.clone(), n, false) {

					// release the votes difference from bidders who provide least roi rate.
					if !available_flag {
						if let Err(_rs) = Self::release_votes_from_bidder(vtoken.clone(), available_votes) {
							return 0;
						};
					} else {
						// if there are unmatched bidding proposals as well as available votes, match proposals to orders in service.
						if let Err(_rs) = Self::check_and_match_unsatisfied_bidding_proposal(vtoken.clone(), n){
							return 0;
						};
					}
				}
			}

			1_000
		}

		/// on_finalize function releases all the votes that has the end block number of current block number.
		fn on_finalize(n: T::BlockNumber) {
			// find out and delete orders with order_end_block_num the same as current block number.
			// Meanwhile, settle the slash deposit of the bidder.
			if OrderEndBlockNumMap::<T>::contains_key(n) {
				if let Err(_rs) = Self::delete_and_settle_orders_end_in_current_block(n) {
					return;
				};
			}


			let vtoken_list = VtokensRegisteredForBidding::<T>::get();
			for vtoken in vtoken_list.iter(){
				// delete record in ToReleaseVotesTilEndOfEra of current era.
				let block_num_per_era = BlockNumberPerEra::<T>::get(vtoken);
				let era_id: T::EraId = (n / block_num_per_era).into();

				if ToReleaseVotesTilEndOfEra::<T>::contains_key((vtoken, era_id)) {
					ToReleaseVotesTilEndOfEra::<T>::remove((vtoken, era_id));
				}
			}
		}

		/// Register a vtoken for bidding marketplace
		#[weight = 1_000]
		fn register_vtoken_for_bidding(origin, vtoken: T::AssetId) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(T::AssetTrait::is_v_token(vtoken), Error::<T>::NotValidVtoken); // ensure the passed in vtoken valid
			ensure!(!VtokensRegisteredForBidding::<T>::get().contains(&vtoken), Error::<T>::VtokenAlreadyRegistered);

			VtokensRegisteredForBidding::<T>::mutate(|vtoken_vec| {
				vtoken_vec.push(vtoken);
			});

			Self::vtoken_empty_storage_initialization(vtoken)?;
			Self::deposit_event(RawEvent::VtokenRegisterSuccess(vtoken));
			Ok(())
		}

		/// cancel a bidding proposal
		#[weight = 1_000]
		fn cancel_a_bidding_proposal(origin, proposal_id: T::BiddingOrderId) -> DispatchResult {
			let canceler = ensure_signed(origin)?;

			ensure!(ProposalsInQueue::<T>::contains_key(proposal_id), Error::<T>::ProposalNotExist);

			let BiddingOrderUnit {bidder_id, token_id: vtoken, block_num: _block_num, votes, annual_roi: _annual_roi, validator: _validator} = ProposalsInQueue::<T>::get(proposal_id);
			ensure!(bidder_id == canceler, Error::<T>::NotProposalOwner);

			BiddingQueues::<T>::mutate(vtoken, |bidding_proposal_vec| {
				if let Ok(index) =
				bidding_proposal_vec.binary_search_by(|(_roi, pro_id)| pro_id.cmp(&proposal_id))
				{
					&bidding_proposal_vec.remove(index);
				}
			});

			// remove the proposal in bidding queue
			ProposalsInQueue::<T>::remove(proposal_id);

			// remove the proposal id from the bidder's list of proposals
			BidderProposalInQueue::<T>::mutate(
				bidder_id,
				vtoken,
				|proposal_vec| {
					let index = proposal_vec.binary_search(&proposal_id).unwrap();
					&proposal_vec.remove(index);
				},
			);

			// deduct the total bidding votes in queue
			TotalProposalsInQueue::<T>::mutate(vtoken, |proposal_balance| {
				*proposal_balance = proposal_balance.saturating_sub(votes);
			});


			Ok(())
		}


		/// this function is call by outer pallets.
		#[weight = 1_000]
		fn set_bidding_order_end_time(origin, order_id: T::BiddingOrderId, end_block_num: T::BlockNumber) -> DispatchResult {
			let setter = ensure_signed(origin.clone())?;
			 // get the order bidder id
			let order_owner = OrdersInService::<T>::get(order_id).bidder_id;

			// only root or the bidder himself can reset the bidding order end time
			if &setter != &order_owner {
				ensure_root(origin)?
			}
			ensure!(OrdersInService::<T>::contains_key(order_id), Error::<T>::OrderNotExist);  //ensure the order exists
			let current_block_number = <frame_system::Module<T>>::block_number(); // get current block number
			ensure!(end_block_num >= current_block_number, Error::<T>::BlockNumberNotValid);  // ensure end_block_num valid

			Self::deposit_event(RawEvent::SetOrderEndTimeSuccess(order_id, end_block_num));

			Self::set_order_end_block(order_id, end_block_num)?;

			Ok(())
		}

		/// create a bidding proposal and update it to the corresponding storage
		#[weight = 1_000]
		fn create_bidding_proposal(origin, vtoken: T::AssetId, votes_needed: T::Balance, annual_roi: T::Balance, validator: T::AccountId
		) -> DispatchResult {
			let bidder = ensure_signed(origin)?;
			ensure!(VtokensRegisteredForBidding::<T>::get().contains(&vtoken), Error::<T>::VtokenNotRegistered);
			// Actually, the token should be ensured as a "vtoken" instead of ensured existence.
			// Should be refactored when the new asset pallet is ready.

			ensure!(votes_needed >= T::MinimumVotes::get(), Error::<T>::VotesExceedLowerBound); // ensure votes_needed valid
			ensure!(votes_needed <= T::MaximumVotes::get(), Error::<T>::VotesExceedUpperBound); // ensure votes_needed valid
			ensure!(annual_roi > Zero::zero(), Error::<T>::AmountNotAboveZero); // ensure annual_roi is valid

			// ensure the bidder's unmatched proposal for a certain vtoken is no more than the limit.
			if BidderProposalInQueue::<T>::contains_key(&bidder, vtoken) {
				ensure!((BidderProposalInQueue::<T>::get(&bidder, vtoken).len() as u32) < T::MaxProposalNumberForBidder::get(), Error::<T>::ProposalsExceedLimit);
			}

			// check if tokens are enough to be reserved.
			let slash_deposit = Self::calculate_order_slash_deposit(vtoken,votes_needed)?;
			let onetime_payment = Self::calculate_order_onetime_payment(vtoken, votes_needed, annual_roi)?;
			let should_deposit = slash_deposit.saturating_add(onetime_payment);
			// get the corresponding token id by vtoken id.
			let token_id = T::AssetTrait::get_pair(vtoken).unwrap();
			let user_token_balance = T::AssetTrait::get_account_asset(token_id, &bidder).available;

			ensure!(user_token_balance >= should_deposit, Error::<T>::NotEnoughBalance);  // ensure user has enough balance

			let current_block_number = <frame_system::Module<T>>::block_number(); // get current block number
			let new_proposal = BiddingOrderUnit {
				bidder_id: bidder.clone(),
				token_id: vtoken,
				block_num: current_block_number,
				votes: votes_needed,
				annual_roi,
				validator
			};

			let new_proposal_id = ProposalNextId::<T>::get();
			ProposalNextId::<T>::mutate(|proposal_id| {
				*proposal_id = proposal_id.saturating_add(1.into());
			});

			ProposalsInQueue::<T>::insert(new_proposal_id, new_proposal);

			// insert a new proposal record into BiddingQueues storage
			BiddingQueues::<T>::mutate(vtoken, |bidding_proposal_vec| {
				let index = bidding_proposal_vec
					.binary_search_by_key(&annual_roi, |(roi, _pro_id)| *roi)
					.unwrap();

				&bidding_proposal_vec.insert(index, (annual_roi, new_proposal_id));
			});

			if !BidderProposalInQueue::<T>::contains_key(&bidder, vtoken) {
				let new_vec: Vec<T::BiddingOrderId> = vec![new_proposal_id];
				BidderProposalInQueue::<T>::insert(&bidder, vtoken, new_vec);
			} else {
				BidderProposalInQueue::<T>::mutate(&bidder, vtoken, |bidder_order_vec| {
					&bidder_order_vec.push(new_proposal_id);
				});
			}

			TotalProposalsInQueue::<T>::mutate(vtoken, |total_props_in_queue| {
				*total_props_in_queue = total_props_in_queue.saturating_add(votes_needed);
			});

			Self::deposit_event(RawEvent::CreateProposalSuccess);

			// match orders
			Self::check_and_match_unsatisfied_bidding_proposal(vtoken, current_block_number)?;

			Ok(())
		}

		/// Below functions can be only called by root.
		/// set the default minimum and maximum order lasting time in the form of block number.
		#[weight = 1_000]
		fn set_min_max_order_lasting_block_num(origin, vtoken: T::AssetId, minimum: T::BlockNumber, maximum: T::BlockNumber
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(VtokensRegisteredForBidding::<T>::get().contains(&vtoken), Error::<T>::VtokenNotRegistered);
			ensure!(minimum <= maximum, Error::<T>::MinimumOrMaximumNotRight);

			if !MinMaxOrderLastingBlockNum::<T>::contains_key(vtoken) {
				MinMaxOrderLastingBlockNum::<T>::insert(vtoken, (minimum, maximum));
			} else {
				MinMaxOrderLastingBlockNum::<T>::mutate(vtoken, |(min, max)| {
					*min = minimum;
					*max = maximum;
				});
			}

			Self::deposit_event(RawEvent::SetMinMaxOrderLastingBlockNumSuccess(vtoken, minimum, maximum));
			Ok(())
		}

		/// set the default block number per era for each vtoken according to its original token chain
		#[weight = 1_000]
		fn set_block_number_per_era(origin, vtoken: T::AssetId, block_num_per_era: T::BlockNumber) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(VtokensRegisteredForBidding::<T>::get().contains(&vtoken), Error::<T>::VtokenNotRegistered);

			if !BlockNumberPerEra::<T>::contains_key(vtoken) {
				BlockNumberPerEra::<T>::insert(vtoken, block_num_per_era);
			} else {
				BlockNumberPerEra::<T>::mutate(vtoken, |old_block_num| {
					*old_block_num = block_num_per_era;
				});
			}

			Self::deposit_event(RawEvent::SetBlockNumberPerEraSuccess(vtoken, block_num_per_era));

			Ok(())
		}

		/// set the lag block number before we can change voting for another validator when we stop a taking
		#[weight = 1_000]
		fn set_service_stop_block_num_lag(origin, vtoken: T::AssetId, service_stop_lag_block_num: T::BlockNumber
		) -> DispatchResult {
			ensure_root(origin)?;

			if !ServiceStopBlockNumLag::<T>::contains_key(vtoken) {
				ServiceStopBlockNumLag::<T>::insert(vtoken, service_stop_lag_block_num);
			} else {
				ServiceStopBlockNumLag::<T>::mutate(vtoken, |old_block_num| {
					*old_block_num = service_stop_lag_block_num;
				});
			}

			Self::deposit_event(RawEvent::SetServiceStopBlockNumLagSuccess(vtoken, service_stop_lag_block_num));

			Ok(())
		}

		/// set slash margin rate for each vtoken.
		#[weight = 1_000]
		fn set_slash_margin_rates(origin, vtoken: T::AssetId, slash_margin_rate: T::Balance) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(VtokensRegisteredForBidding::<T>::get().contains(&vtoken), Error::<T>::VtokenNotRegistered);
			ensure!(slash_margin_rate< T::BidRatePrecision::get(), Error::<T>::RateExceedUpperBound);

			if !SlashMarginRates::<T>::contains_key(vtoken) {
				SlashMarginRates::<T>::insert(vtoken, slash_margin_rate);
			} else {
				SlashMarginRates::<T>::mutate(vtoken, |old_rate| {
					*old_rate = slash_margin_rate;
				});
			}
			Self::deposit_event(RawEvent::SetSlashMarginRatesSuccess(vtoken, slash_margin_rate));

			Ok(())
		}
	}
}

#[allow(dead_code)]
impl<T: Trait> Module<T> {
	/// read BiddingQueues storage to see if there are unsatisfied proposals, and match them with available votes.
	/// If the available votes are less than needed, an order in service will be created with the available votes.
	/// Meanwhile a new bidding proposal will be issued with the remained unmet votes.
	fn check_and_match_unsatisfied_bidding_proposal(
		vtoken: T::AssetId,
		current_block_num: T::BlockNumber,
	) -> DispatchResult {
		// current mode for checking votes availability, not end of era future mode.
		let (available_flag, available_votes) =
			Self::calculate_available_votes(vtoken, current_block_num, true)?;

		ensure!(
			MinMaxOrderLastingBlockNum::<T>::contains_key(vtoken),
			Error::<T>::MinMaxOrderLastingBlockNumNotSet
		);
		let (_, max_order_lasting_block_num) = MinMaxOrderLastingBlockNum::<T>::get(vtoken);

		// if there are unmatched bidding proposals as well as available votes, match proposals to orders in service.
		if available_flag {
			BiddingQueues::<T>::mutate(vtoken, |bidding_proposal_vec| -> DispatchResult {
				if !bidding_proposal_vec.is_empty() {
					// There are un-matching proposals.
					let mut votes_avail = available_votes;
					let mut vec_pointer = bidding_proposal_vec.len();

					while (vec_pointer >= Zero::zero()) & (votes_avail > Zero::zero()) {
						vec_pointer = vec_pointer.saturating_sub(1);
						let order_end_block_num =
							current_block_num.saturating_add(max_order_lasting_block_num);
						// let mut votes_matched = bidding_proposal_vec[vec_pointer].1.votes;

						let (_, proposal_id) = bidding_proposal_vec[vec_pointer];
						let proposal = ProposalsInQueue::<T>::get(proposal_id);

						let bidder_id = &proposal.bidder_id;
						let mut votes_matched = &proposal.votes;

						if votes_matched <= &votes_avail {
							bidding_proposal_vec.pop(); // delete this proposal

							// remove the proposal in bidding queue
							ProposalsInQueue::<T>::remove(proposal_id);

							// remove the proposal id from the bidder's list of proposals
							BidderProposalInQueue::<T>::mutate(bidder_id, vtoken, |proposal_vec| {
								let index = proposal_vec.binary_search(&proposal_id).unwrap();
								&proposal_vec.remove(index);
							});
						} else {
							// deduct the needed votes of original proposal
							ProposalsInQueue::<T>::mutate(proposal_id, |proposal_detail| {
								proposal_detail.votes = votes_matched.saturating_sub(votes_avail);
							});
							votes_matched = &votes_avail;
						}

						// deduct the total bidding votes in queue
						TotalProposalsInQueue::<T>::mutate(vtoken, |proposal_balance| {
							*proposal_balance = proposal_balance.saturating_sub(*votes_matched);
						});

						// create a matched order
						Self::create_order_in_service(
							&proposal,
							order_end_block_num,
							*votes_matched,
						)?;
						votes_avail = votes_avail.saturating_sub(*votes_matched);
					}
				}
				Ok(())
			})?;
		}

		Ok(())
	}

	/// create an order in service. The votes_matched might be less than the needed votes in the proposal.
	fn create_order_in_service(
		proposal: &BiddingOrderUnit<T::AccountId, T::AssetId, T::BlockNumber, T::Balance>,
		order_end_block_num: T::BlockNumber,
		votes_matched: T::Balance,
	) -> DispatchResult {
		// current block number
		let current_block_num = <frame_system::Module<T>>::block_number();
		ensure!(
			order_end_block_num >= current_block_num,
			Error::<T>::BlockNumberNotValid
		);
		ensure!(votes_matched > Zero::zero(), Error::<T>::AmountNotAboveZero);

		let BiddingOrderUnit {
			bidder_id: bidder,
			token_id: vtoken,
			block_num: _block_num,
			votes: _votes,
			annual_roi,
			validator,
		} = proposal;

		// ensure the bidder has enough balance
		let slash_deposit = Self::calculate_order_slash_deposit(*vtoken, votes_matched)?;
		let onetime_payment =
			Self::calculate_order_onetime_payment(*vtoken, votes_matched, *annual_roi)?;
		let should_deposit = slash_deposit.saturating_add(onetime_payment);
		// get the corresponding token id by vtoken id.
		let token_id = T::AssetTrait::get_pair(*vtoken).unwrap();
		let user_token_balance = T::AssetTrait::get_account_asset(token_id, &bidder).available;

		ensure!(
			user_token_balance >= should_deposit,
			Error::<T>::NotEnoughBalance
		); // ensure user has enough balance

		// lock the slash deposit
		T::AssetTrait::lock_asset(&bidder, token_id, slash_deposit);
		// deduct the onetime payment asset_redeem(assetId, &target, amount)
		T::AssetTrait::asset_redeem(token_id, &bidder, onetime_payment);

		let new_order = BiddingOrderUnit {
			bidder_id: bidder.clone(),
			token_id: *vtoken,
			block_num: order_end_block_num,
			votes: votes_matched,
			annual_roi: *annual_roi,
			validator: validator.clone(),
		};

		let new_order_id = OrderNextId::<T>::get();
		OrderNextId::<T>::mutate(|odr_id| {
			*odr_id = new_order_id.saturating_add(1.into());
		});
		// Below are code adding this order to corresponding storage.
		OrdersInService::<T>::insert(new_order_id, new_order);

		if !OrderEndBlockNumMap::<T>::contains_key(order_end_block_num) {
			let new_vec: Vec<T::BiddingOrderId> = vec![new_order_id];
			OrderEndBlockNumMap::<T>::insert(order_end_block_num, new_vec);
		} else {
			OrderEndBlockNumMap::<T>::mutate(order_end_block_num, |order_end_block_num_vec| {
				&order_end_block_num_vec.push(new_order_id);
			});
		}

		if !BidderTokenOrdersInService::<T>::contains_key(&bidder, vtoken) {
			let new_vec: Vec<T::BiddingOrderId> = vec![new_order_id];
			BidderTokenOrdersInService::<T>::insert(&bidder, vtoken, new_vec);
		} else {
			BidderTokenOrdersInService::<T>::mutate(&bidder, vtoken, |bidder_order_vec| {
				&bidder_order_vec.push(new_order_id);
			});
		}

		TokenOrderROIList::<T>::mutate(vtoken, |balance_order_vec| {
			let index = balance_order_vec
				.binary_search_by_key(&annual_roi, |(roi, _odr_id)| roi)
				.unwrap();
			if index < (T::TokenOrderROIListLength::get() as usize) {
				&balance_order_vec.insert(index, (*annual_roi, new_order_id));
			}

			if &balance_order_vec.len() > &(T::TokenOrderROIListLength::get() as usize) {
				// shrink the vec to maximum size
				&balance_order_vec.resize(
					T::TokenOrderROIListLength::get() as usize,
					(Zero::zero(), Zero::zero()),
				);
			}
		});

		TotalVotesInService::<T>::mutate(vtoken, |total_votes_in_service| {
			*total_votes_in_service = total_votes_in_service.saturating_add(votes_matched);
		});

		let block_num_per_era = BlockNumberPerEra::<T>::get(vtoken);
		let era_id: T::EraId = (order_end_block_num / block_num_per_era).into();
		if !ToReleaseVotesTilEndOfEra::<T>::contains_key((vtoken, era_id)) {
			ToReleaseVotesTilEndOfEra::<T>::insert((vtoken, era_id), votes_matched);
		} else {
			ToReleaseVotesTilEndOfEra::<T>::mutate((vtoken, era_id), |votes_released| {
				*votes_released = votes_released.saturating_add(votes_matched);
			});
		}
		Ok(())
	}

	/// split an order in service into two orders with only votes_matched field different.
	/// order1 gets the original order id. order2 gets a new order id.
	fn split_order_in_service(
		order_id: T::BiddingOrderId,
		order1_votes_amount: T::Balance,
	) -> DispatchResult {
		ensure!(
			OrdersInService::<T>::contains_key(order_id),
			Error::<T>::OrderNotExist
		);
		let BiddingOrderUnit {
			bidder_id,
			token_id,
			block_num,
			votes,
			annual_roi,
			validator,
		} = OrdersInService::<T>::get(order_id);
		let order2_votes = votes.saturating_sub(order1_votes_amount);

		let new_order = BiddingOrderUnit {
			bidder_id: bidder_id.clone(),
			token_id,
			block_num,
			votes: order2_votes,
			annual_roi,
			validator,
		};

		let new_order_id = OrderNextId::<T>::get();
		OrderNextId::<T>::mutate(|odr_id| {
			*odr_id = new_order_id.saturating_add(1.into());
		});

		OrdersInService::<T>::insert(new_order_id, new_order);
		OrdersInService::<T>::mutate(order_id, |order_detail| {
			order_detail.votes = order1_votes_amount;
		});

		OrderEndBlockNumMap::<T>::mutate(block_num, |ord_id_vec| {
			&ord_id_vec.push(new_order_id);
		});

		BidderTokenOrdersInService::<T>::mutate(bidder_id, token_id, |ord_id_vec| {
			&ord_id_vec.push(new_order_id);
		});
		TokenOrderROIList::<T>::mutate(token_id, |balance_order_vec| {
			let index = balance_order_vec
				.binary_search_by_key(&annual_roi, |(roi, _odr_id)| *roi)
				.unwrap();
			if index < (T::TokenOrderROIListLength::get() as usize) {
				&balance_order_vec.insert(index, (annual_roi, new_order_id));
			}

			if &balance_order_vec.len() > &(T::TokenOrderROIListLength::get() as usize) {
				// shrink the vec to maximum size
				&balance_order_vec.resize(
					T::TokenOrderROIListLength::get() as usize,
					(Zero::zero(), Zero::zero()),
				);
			}
		});

		if SlashForOrdersInService::<T>::contains_key(order_id) {
			let slash_amount = SlashForOrdersInService::<T>::get(order_id);

			// calculate order1 and order2 slash amount according to their proportion
			let order1_slash_amount = order1_votes_amount.saturating_mul(slash_amount)
				/ (order1_votes_amount.saturating_add(order2_votes));
			let order2_slash_amount = slash_amount.saturating_sub(order1_slash_amount);

			// change order1 slash amount and insert order2 slash amount
			SlashForOrdersInService::<T>::mutate(order_id, |old_slash_amount| {
				*old_slash_amount = order1_slash_amount;
			});
			SlashForOrdersInService::<T>::insert(new_order_id, order2_slash_amount);
		}

		Ok(())
	}

	/// change the order in service's end block time.
	fn set_order_end_block(
		order_id: T::BiddingOrderId,
		end_block_num: T::BlockNumber,
	) -> DispatchResult {
		ensure!(
			OrdersInService::<T>::contains_key(order_id),
			Error::<T>::OrderNotExist
		); //ensure the order exists

		let BiddingOrderUnit {
			bidder_id: _bidder_id,
			token_id: vtoken,
			block_num: original_end_block_num,
			votes,
			annual_roi: _annual_roi,
			validator: _validator,
		} = OrdersInService::<T>::get(order_id);

		let current_block_number = <frame_system::Module<T>>::block_number(); // get current block number
		ensure!(
			end_block_num <= current_block_number,
			Error::<T>::BlockNumberNotValid
		);

		let block_num_per_era = BlockNumberPerEra::<T>::get(vtoken);
		let era_id: T::EraId = (end_block_num / block_num_per_era).into();

		OrdersInService::<T>::mutate(order_id, |order_to_revise| {
			order_to_revise.block_num = end_block_num;
		});

		let original_end_era: T::EraId = (original_end_block_num / block_num_per_era).into();
		OrderEndBlockNumMap::<T>::mutate(original_end_block_num, |order_id_vec| {
			if let Ok(index) = order_id_vec.binary_search(&order_id) {
				&order_id_vec.remove(index);
			}
		});

		if !OrderEndBlockNumMap::<T>::contains_key(end_block_num) {
			let new_vec = vec![order_id];
			OrderEndBlockNumMap::<T>::insert(end_block_num, new_vec);
		} else {
			OrderEndBlockNumMap::<T>::mutate(end_block_num, |order_id_vec| {
				&order_id_vec.push(order_id);
			});
		}

		ToReleaseVotesTilEndOfEra::<T>::mutate((vtoken, original_end_era), |votes_to_release| {
			*votes_to_release = votes_to_release.saturating_sub(votes);
		});

		if !ToReleaseVotesTilEndOfEra::<T>::contains_key((vtoken, era_id)) {
			ToReleaseVotesTilEndOfEra::<T>::insert((vtoken, era_id), votes)
		} else {
			ToReleaseVotesTilEndOfEra::<T>::mutate((vtoken, era_id), |votes_to_release| {
				*votes_to_release = votes_to_release.saturating_add(votes);
			});
		}
		Ok(())
	}

	/// initialize empty storage for each vtoken
	fn vtoken_empty_storage_initialization(vtoken: T::AssetId) -> DispatchResult {
		let empty_bidding_order_unit_vec: Vec<(T::Balance, T::BiddingOrderId)> = Vec::new();

		// initialize proposal related storage
		BiddingQueues::<T>::insert(vtoken, empty_bidding_order_unit_vec);
		let zero_balance: T::Balance = Zero::zero();
		TotalProposalsInQueue::<T>::insert(vtoken, zero_balance);

		// initialize order related storage
		let empty_token_order_roi_vec: Vec<(T::Balance, T::BiddingOrderId)> = Vec::new();
		TokenOrderROIList::<T>::insert(vtoken, empty_token_order_roi_vec);

		let zero_votes: T::Balance = Zero::zero();
		TotalVotesInService::<T>::insert(vtoken, zero_votes.clone());
		WithdrawReservedVotes::<T>::insert(vtoken, zero_votes.clone());

		Ok(())
	}

	/// calculate currently available votes. Returned Value(Boolean, T::Balance), if the first element of the tuple shows
	/// true, the second element is the available votes. If the first element of the tuple shows false, the second element
	/// is the votes needed to be release from bidder.
	fn calculate_available_votes(
		vtoken: T::AssetId,
		current_block_num: T::BlockNumber,
		current_mode: bool,
	) -> Result<(bool, T::Balance), Error<T>> {
		ensure!(
			BlockNumberPerEra::<T>::contains_key(vtoken),
			Error::<T>::VtokenBlockNumberPerEraNotSet
		);
		let block_num_per_era = BlockNumberPerEra::<T>::get(vtoken);

		let era_id: T::EraId = (current_block_num / block_num_per_era).into(); // current era id.
		let total_votes_supply = Self::get_total_votes(vtoken); // total votes
		let total_votes_in_service = TotalVotesInService::<T>::get(vtoken); // votes in service

		let to_release_votes_til_end_of_era = {
			if !ToReleaseVotesTilEndOfEra::<T>::contains_key((vtoken, era_id)) {
				let new_votes: T::Balance = Zero::zero();
				ToReleaseVotesTilEndOfEra::<T>::insert((vtoken, era_id), new_votes.clone());
				new_votes
			} else {
				ToReleaseVotesTilEndOfEra::<T>::get((vtoken, era_id))
			}
		};
		let reserved_votes = WithdrawReservedVotes::<T>::get(vtoken);

		let lhs = {
			if current_mode {
				// if it's current mode, it means calculating current available amount.
				total_votes_supply.saturating_add(to_release_votes_til_end_of_era)
			} else {
				// if it's not current mode, it means calculating the available amount by the end of current era.
				total_votes_supply
			}
		};

		let rhs = total_votes_in_service.saturating_add(reserved_votes);
		let result = {
			if lhs >= rhs {
				(true, lhs.saturating_sub(rhs))
			} else {
				(false, rhs.saturating_sub(lhs))
			}
		};
		Ok(result)
	}

	/// If total votes are less than votes in service(available votes is a negative number), we need to release some
	///  votes from the bidder who provides the lowest roi rate.
	fn release_votes_from_bidder(vtoken: T::AssetId, release_votes: T::Balance) -> DispatchResult {
		let mut remained_to_release_vote = release_votes;

		TokenOrderROIList::<T>::mutate(vtoken, |balance_order_id_vec| -> DispatchResult {
			let mut i = 0;
			while (remained_to_release_vote > Zero::zero()) & (i < balance_order_id_vec.len()) {
				let (_roi, order_id) = &balance_order_id_vec[i];
				let BiddingOrderUnit {
					bidder_id: _bidder_id,
					token_id: _token_id,
					block_num: _block_num,
					votes,
					annual_roi: _annual_roi,
					validator: _validator,
				} = OrdersInService::<T>::get(order_id);

				let current_block_number = <frame_system::Module<T>>::block_number(); // get current block number
				let block_num_per_era = BlockNumberPerEra::<T>::get(vtoken);
				let era_id = current_block_number / block_num_per_era;
				let end_block_num = era_id
					.saturating_add(1.into())
					.saturating_mul(block_num_per_era)
					.saturating_sub(1.into());
				let mut should_deduct = votes;

				if remained_to_release_vote < votes {
					Self::split_order_in_service(*order_id, remained_to_release_vote)?;
					should_deduct = remained_to_release_vote;
				}
				Self::set_order_end_block(*order_id, end_block_num)?;
				remained_to_release_vote = remained_to_release_vote.saturating_sub(should_deduct);
				i = i.saturating_add(1);
			}
			Ok(())
		})?;
		Ok(())
	}

	/// calculate how much slash deposit bidder should be locked.
	fn calculate_order_slash_deposit(
		vtoken: T::AssetId,
		votes_matched: T::Balance,
	) -> Result<T::Balance, Error<T>> {
		ensure!(
			SlashMarginRates::<T>::contains_key(vtoken),
			Error::<T>::SlashMarginRatesNotSet
		);

		let slash_rate = SlashMarginRates::<T>::get(vtoken);
		Ok(votes_matched.saturating_mul(slash_rate) / T::BidRatePrecision::get())
	}

	/// calculate the minimum one time payment the bidder should pay for his votes needed.
	fn calculate_order_onetime_payment(
		vtoken: T::AssetId,
		votes_matched: T::Balance,
		roi_rate: T::Balance,
	) -> Result<T::Balance, Error<T>> {
		ensure!(
			MinMaxOrderLastingBlockNum::<T>::contains_key(vtoken),
			Error::<T>::MinMaxOrderLastingBlockNumNotSet
		);
		ensure!(
			ServiceStopBlockNumLag::<T>::contains_key(vtoken),
			Error::<T>::ServiceStopBlockNumLagNotSet
		);

		let (minimum_order_lasting_block_num, _) = MinMaxOrderLastingBlockNum::<T>::get(vtoken);
		let stop_lag_block_num = ServiceStopBlockNumLag::<T>::get(vtoken);

		let base: T::Balance =
			(minimum_order_lasting_block_num.saturating_add(stop_lag_block_num)).into();

		Ok(base.saturating_mul(roi_rate).saturating_mul(votes_matched)
			/ T::BidRatePrecision::get()
			/ T::BlocksPerYear::get().into())
	}

	/// delete and settle orders due in batch.
	fn delete_and_settle_orders_end_in_current_block(
		current_block_num: T::BlockNumber,
	) -> DispatchResult {
		let due_order_vec = OrderEndBlockNumMap::<T>::get(current_block_num);

		for order_id in due_order_vec.iter() {
			Self::delete_and_settle_an_order(*order_id, current_block_num)?;
		}

		OrderEndBlockNumMap::<T>::remove(current_block_num);

		Ok(())
	}

	/// Except the OrderEndBlockNumMap storage, delete the other storages related to an order.
	/// Settle the slash deposit with the bidder for the order.
	fn delete_and_settle_an_order(
		order_id: T::BiddingOrderId,
		current_block_num: T::BlockNumber,
	) -> DispatchResult {
		ensure!(
			OrdersInService::<T>::contains_key(order_id),
			Error::<T>::OrderNotExist
		); //ensure the order exists

		let order_detail = OrdersInService::<T>::get(&order_id);
		OrdersInService::<T>::remove(&order_id);

		BidderTokenOrdersInService::<T>::mutate(
			&order_detail.bidder_id,
			order_detail.token_id,
			|bidder_order_vec| {
				let index = bidder_order_vec.binary_search(&order_id).unwrap();
				&bidder_order_vec.remove(index);
			},
		);
		TokenOrderROIList::<T>::mutate(&order_detail.token_id, |order_roi_vec| {
			if let Ok(index) =
				order_roi_vec.binary_search_by(|(_votes, ord_id)| ord_id.cmp(&order_id))
			{
				&order_roi_vec.remove(index);
			}
		});

		TotalVotesInService::<T>::mutate(order_detail.token_id, |votes_in_service| {
			*votes_in_service = votes_in_service.saturating_sub(order_detail.votes);
		});

		let block_num_per_era = BlockNumberPerEra::<T>::get(order_detail.token_id);
		let era_id: T::EraId = (current_block_num / block_num_per_era).into(); // current era id
		ToReleaseVotesTilEndOfEra::<T>::mutate(
			(order_detail.token_id, era_id),
			|to_release_balance| {
				*to_release_balance = to_release_balance.saturating_sub(order_detail.votes);
			},
		);

		// Below is code dealing with slash deposit.

		// release the remaining slash deposit to the bidder
		let original_slash_deposit =
			Self::calculate_order_slash_deposit(order_detail.token_id, order_detail.votes)?;
		let mut slashed_amount = Zero::zero();
		if SlashForOrdersInService::<T>::contains_key(order_id) {
			slashed_amount = SlashForOrdersInService::<T>::get(order_id);
			// delete the slashed record
			SlashForOrdersInService::<T>::remove(order_id);
		}

		T::AssetTrait::unlock_asset(
			&order_detail.bidder_id,
			order_detail.token_id,
			original_slash_deposit,
		);

		// unlock the remaining slash deposit.
		if slashed_amount > original_slash_deposit {
			slashed_amount = original_slash_deposit;
		}

		T::AssetTrait::asset_redeem(
			order_detail.token_id,
			&order_detail.bidder_id,
			slashed_amount,
		);

		Ok(())
	}

	// *********************************************************
	// Below is info that needs to be used by or queried from other pallets.

	/// set the slash amount for s specific order. Whenever a slash happens, outer pallet update this storage.
	fn set_slash_amount_for_bidding_order(
		order_id: T::BiddingOrderId,
		slash_amount: T::Balance,
	) -> DispatchResult {
		if !SlashForOrdersInService::<T>::contains_key(order_id) {
			SlashForOrdersInService::<T>::insert(order_id, slash_amount);
		} else {
			SlashForOrdersInService::<T>::mutate(order_id, |old_amount| {
				*old_amount = old_amount.saturating_add(slash_amount);
			});
		}
		Ok(())
	}

	/// set the WithdrawReservedVotes storage by staking pallet. If needs to withdraw, add reserve amount. If finish
	/// withdrawing, deduct the amount
	fn set_withdraw_reserved_votes(
		token_id: T::AssetId,
		amount: T::Balance,
		deduct_mode: bool,
	) -> DispatchResult {
		if !WithdrawReservedVotes::<T>::contains_key(token_id) {
			if !deduct_mode {
				WithdrawReservedVotes::<T>::insert(token_id, amount);
			}
		} else {
			if !deduct_mode {
				WithdrawReservedVotes::<T>::mutate(token_id, |old_amount| {
					*old_amount = old_amount.saturating_add(amount);
				});
			} else {
				WithdrawReservedVotes::<T>::mutate(token_id, |old_amount| {
					*old_amount = old_amount.saturating_sub(amount);
				});
			}
		}
		Ok(())
	}

	/// get the current total votes from convert pool
	fn get_total_votes(_vtoken: T::AssetId) -> T::Balance {
		let current_block_number = <frame_system::Module<T>>::block_number(); // get current block number
		let mock_total_votes = current_block_number % T::BlockNumber::from(10_000);
		mock_total_votes.into()
	}
}
