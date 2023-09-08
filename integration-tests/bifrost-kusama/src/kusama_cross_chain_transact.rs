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
	kusama_integration_tests::*, kusama_test_net::*, vtoken_voting::set_balance_proposal_bounded,
};
use frame_support::{
	assert_ok,
	dispatch::{GetDispatchInfo, RawOrigin},
};
use pallet_conviction_voting::{AccountVote, Vote};
use xcm::v3::{prelude::*, Weight};
use xcm_emulator::TestExt;

#[test]
fn relaychain_transact_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		let vote_call =
			kusama_runtime::RuntimeCall::ConvictionVoting(pallet_conviction_voting::Call::<
				kusama_runtime::Runtime,
			>::vote {
				poll_index: 0,
				vote: aye(2, 1),
			});

		let notify_vote_call =
			RuntimeCall::VtokenVoting(bifrost_vtoken_voting::Call::<Runtime>::notify_vote {
				query_id: 0,
				response: Default::default(),
			});

		KusamaNet::execute_with(|| {
			use frame_support::traits::schedule::DispatchTime;
			use kusama_runtime::{Referenda, RuntimeEvent, RuntimeOrigin, System};

			println!("KusamaNet vote_call weight: {:?}", vote_call.get_dispatch_info().weight);

			assert_ok!(Referenda::submit(
				RuntimeOrigin::signed(ALICE.into()),
				Box::new(RawOrigin::Root.into()),
				set_balance_proposal_bounded(1),
				DispatchTime::At(1),
			));
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::Referenda(pallet_referenda::Event::Submitted {
					index: 0,
					track: _,
					proposal: _,
				})
			)));
		});

		Bifrost::execute_with(|| {
			// QueryStatus::Pending { responder: V3(MultiLocation { parents: 1, interior: Here }),
			// maybe_match_querier: Some(V3(MultiLocation { parents: 0, interior: Here })),
			// maybe_notify: Some((0, 7)), timeout: 100 } let query_id =
			let query_id = pallet_xcm::Pallet::<Runtime>::new_notify_query(
				MultiLocation::parent(),
				notify_vote_call,
				100u32.into(),
				Here,
			);

			// QueryResponse { query_id: 0, response: DispatchResult(Success), max_weight: Weight {
			// ref_time: 4000000000, proof_size: 0 }, querier: Some(MultiLocation { parents: 0,
			// interior: Here }) }
			let asset: MultiAsset =
				MultiAsset { id: Concrete(MultiLocation::here()), fun: Fungible(517318631) };
			let msg = Xcm(vec![
				WithdrawAsset(asset.clone().into()),
				BuyExecution { fees: asset, weight_limit: Unlimited },
				Transact {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most: Weight::from_parts(961496000, 83866),
					call: vote_call.encode().into(),
				},
				ReportTransactStatus(QueryResponseInfo {
					destination: MultiLocation::from(X1(Parachain(2001))),
					query_id,
					max_weight: Weight::from_parts(66000000, 3582),
				}),
				RefundSurplus,
				DepositAsset {
					assets: All.into(),
					beneficiary: MultiLocation { parents: 0, interior: X1(Parachain(2001)) },
				},
			]);
			assert_ok!(pallet_xcm::Pallet::<Runtime>::send_xcm(Here, MultiLocation::parent(), msg));
		});

		KusamaNet::execute_with(|| {
			use kusama_runtime::{RuntimeEvent, System};

			System::events().iter().for_each(|r| println!("KusamaNet >>> {:?}", r.event));
			assert!(System::events().iter().any(|r| matches!(
				&r.event,
				RuntimeEvent::Ump(polkadot_runtime_parachains::ump::Event::ExecutedUpward(
					_,
					crate::kusama_cross_chain_transact::Outcome::Complete(_)
				))
			)));
		});

		Bifrost::execute_with(|| {
			System::events().iter().for_each(|r| println!("Bifrost >>> {:?}", r.event));
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::VtokenVoting(bifrost_vtoken_voting::Event::ResponseReceived {
					responder: MultiLocation { parents: 1, interior: Here },
					query_id: 0,
					response: crate::kusama_cross_chain_transact::Response::DispatchResult(
						MaybeErrorCode::Success
					)
				})
			)));
		});
	})
}

pub fn aye(amount: Balance, conviction: u8) -> AccountVote<Balance> {
	let vote = Vote { aye: true, conviction: conviction.try_into().unwrap() };
	AccountVote::Standard { vote, balance: amount }
}
