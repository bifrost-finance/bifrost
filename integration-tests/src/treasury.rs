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
use crate::{integration_tests::*, kusama_test_net::*};
use frame_support::assert_ok;
use xcm::latest::prelude::*;
use xcm_emulator::TestExt;

#[test]
fn treasury_send_ksm_to_parachain() {
	TestNet::reset();

	Bifrost::execute_with(|| {
		let remark = kusama_runtime::Call::System(
			frame_system::Call::<kusama_runtime::Runtime>::remark_with_event {
				remark: "Hello from Bifrost!".as_bytes().to_vec(),
			},
		);

		let asset = MultiAsset {
			id: Concrete(MultiLocation::here()),
			fun: Fungibility::Fungible(8000000000),
		};
		let weight = 10_000_000_000;
		let xcm_msg = Xcm(vec![
			WithdrawAsset(asset.clone().into()),
			BuyExecution { fees: asset.clone(), weight_limit: Unlimited },
			Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: weight,
				call: remark.encode().into(),
			},
			RefundSurplus,
			DepositAsset {
				assets: All.into(),
				max_assets: u32::max_value(),
				beneficiary: MultiLocation { parents: 0, interior: X1(Parachain(2001)) },
			},
		]);
		Bifrost::execute_with(|| {
			assert_ok!(pallet_xcm::Pallet::<Runtime>::send_xcm(Here, Parent, xcm_msg));
		});
	});
}

#[test]
fn treasury_transfer_ksm_from_parachain_to_treasury() {
	Bifrost::execute_with(|| {
		assert_ok!(XTokens::transfer(
			Origin::signed(ALICE.into()),
			CurrencyId::Token(TokenSymbol::KSM),
			10_000_000_000_000,
			Box::new(
				MultiLocation::new(
					1,
					X2(
						Parachain(2001),
						Junction::AccountId32 { network: NetworkId::Any, id: BIFROST_TREASURY_ACCOUNT.into() }
					)
				)
				.into()
			),
			1_000_000_000,
		));

		// assert_eq!(
		// 	Tokens::free_balance(CurrencyId::Token(TokenSymbol::KSM), &AccountId::from(ALICE)),
		// 	90_000_000_000_000
		// );
	});
}
