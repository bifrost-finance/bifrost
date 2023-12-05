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

use crate::vtoken_voting::set_balance_proposal_bounded;
use bifrost_kusama_runtime::{Runtime, RuntimeCall, RuntimeEvent, System};
use bifrost_primitives::Balance;
use frame_support::{
	assert_ok,
	dispatch::{GetDispatchInfo, RawOrigin},
};
use integration_tests_common::{BifrostKusama, Kusama, KusamaAlice};
use pallet_conviction_voting::{AccountVote, Vote};
use parity_scale_codec::Encode;
use xcm::v3::{prelude::*, Weight};
use xcm_emulator::{bx, Parachain, RelayChain, TestExt};

#[test]
fn relaychain_transact_works() {
	let vote_call =
		kusama_runtime::RuntimeCall::ConvictionVoting(pallet_conviction_voting::Call::<
			kusama_runtime::Runtime,
		>::vote {
			poll_index: 0,
			vote: aye(1_000_000_000_000u128, 1),
		});

	let notify_vote_call =
		RuntimeCall::VtokenVoting(bifrost_vtoken_voting::Call::<Runtime>::notify_vote {
			query_id: 0,
			response: Default::default(),
		});

	Kusama::execute_with(|| {
		use frame_support::traits::schedule::DispatchTime;
		use kusama_runtime::{Balances, Referenda, RuntimeEvent, RuntimeOrigin, System};

		println!("KusamaNet vote_call weight: {:?}", vote_call.get_dispatch_info().weight);

		assert_ok!(Referenda::submit(
			RuntimeOrigin::signed(KusamaAlice::get()),
			bx!(RawOrigin::Root.into()),
			set_balance_proposal_bounded(1),
			DispatchTime::At(1),
		));

		assert_ok!(Balances::force_set_balance(
			RuntimeOrigin::root(),
			Kusama::sovereign_account_id_of_child_para(BifrostKusama::para_id()).into(),
			100_000_000_000_000u128
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

	BifrostKusama::execute_with(|| {
		let notify_vote_call_weight = notify_vote_call.get_dispatch_info().weight;
		let query_id = pallet_xcm::Pallet::<Runtime>::new_notify_query(
			MultiLocation::parent(),
			notify_vote_call,
			100u32.into(),
			Here,
		);

		let asset: MultiAsset =
			MultiAsset { id: Concrete(MultiLocation::here()), fun: Fungible(1000000000000) };
		let msg = Xcm(vec![
			WithdrawAsset(asset.clone().into()),
			BuyExecution { fees: asset, weight_limit: Unlimited },
			Transact {
				origin_kind: OriginKind::SovereignAccount,
				require_weight_at_most: Weight::from_parts(1019439000, 83866),
				call: vote_call.encode().into(),
			},
			ReportTransactStatus(QueryResponseInfo {
				destination: MultiLocation::from(X1(Parachain(2001))),
				query_id,
				max_weight: notify_vote_call_weight,
			}),
			RefundSurplus,
			DepositAsset {
				assets: All.into(),
				beneficiary: MultiLocation { parents: 0, interior: X1(Parachain(2001)) },
			},
		]);
		assert_ok!(pallet_xcm::Pallet::<Runtime>::send_xcm(Here, MultiLocation::parent(), msg));
	});

	Kusama::execute_with(|| {
		use kusama_runtime::{RuntimeEvent, System};

		System::events().iter().for_each(|r| println!("KusamaNet >>> {:?}", r.event));
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
		System::events().iter().for_each(|r| println!("Bifrost >>> {:?}", r.event));
		assert!(System::events().iter().any(|r| matches!(
			r.event,
			RuntimeEvent::VtokenVoting(bifrost_vtoken_voting::Event::ResponseReceived {
				responder: MultiLocation { parents: 1, interior: Here },
				query_id: 0,
				response: Response::DispatchResult(MaybeErrorCode::Success)
			})
		)));
	});
}

pub fn aye(amount: Balance, conviction: u8) -> AccountVote<Balance> {
	let vote = Vote { aye: true, conviction: conviction.try_into().unwrap() };
	AccountVote::Standard { vote, balance: amount }
}
