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

use frame_support::assert_ok;
use xcm::v3::{prelude::*, Weight};
use xcm_emulator::{ParaId, TestExt};

use crate::{kusama_integration_tests::*, kusama_test_net::*};

#[test]
fn relaychain_transact_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		let transfer = kusama_runtime::RuntimeCall::Balances(pallet_balances::Call::<
			kusama_runtime::Runtime,
		>::transfer {
			dest: MultiAddress::Id(AccountId::from(BOB)),
			value: 1 * KSM_DECIMALS,
		});

		// let notification_received = RuntimeCall::Salp(bifrost_salp::Call::<
		// 	Runtime,
		// >::notification_received {
		// 	query_id: 0,
		// 	response: Default::default(),
		// });

		Bifrost::execute_with(|| {
			// QueryStatus::Pending { responder: V3(MultiLocation { parents: 1, interior: Here }),
			// maybe_match_querier: Some(V3(MultiLocation { parents: 0, interior: Here })),
			// maybe_notify: Some((0, 7)), timeout: 100 } let query_id =
			// pallet_xcm::Pallet::<Runtime>::new_notify_query( 	MultiLocation::parent(),
			// 	notification_received.clone(),
			// 	100u32.into(),
			// 	Here,
			// );

			// QueryResponse { query_id: 0, response: DispatchResult(Success), max_weight: Weight {
			// ref_time: 4000000000, proof_size: 0 }, querier: Some(MultiLocation { parents: 0,
			// interior: Here }) }
			let asset: MultiAsset =
				MultiAsset { id: Concrete(MultiLocation::here()), fun: Fungible(4000000000) };
			let msg = Xcm(vec![
				WithdrawAsset(asset.clone().into()),
				BuyExecution { fees: asset, weight_limit: Unlimited },
				Transact {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most: Weight::from_parts(4000000000, 0),
					call: transfer.encode().into(),
				},
				// ReportTransactStatus(QueryResponseInfo {
				// 	destination: MultiLocation::from(X1(Parachain(2001))),
				// 	query_id,
				// 	max_weight: Weight::from_parts(4000000000, 0),
				// }),
				RefundSurplus,
				DepositAsset {
					assets: All.into(),
					beneficiary: MultiLocation::from(AccountId32 { network: None, id: ALICE }),
				},
			]);
			assert_ok!(pallet_xcm::Pallet::<Runtime>::send_xcm(Here, MultiLocation::parent(), msg));
		});

		KusamaNet::execute_with(|| {
			use kusama_runtime::{RuntimeEvent, System};
			// Expect to transfer 1 KSM from parachain_account to bob_account
			let _parachain_account: AccountId = ParaId::from(2001).into_account_truncating();
			let _bob_account: AccountId = AccountId::from(BOB);
			assert!(System::events().iter().any(|r| matches!(
				&r.event,
				RuntimeEvent::Balances(pallet_balances::Event::Transfer {
					from: _parachain_account,
					to: _bob_account,
					amount: KSM_DECIMALS
				})
			)));
		});
	})
}
