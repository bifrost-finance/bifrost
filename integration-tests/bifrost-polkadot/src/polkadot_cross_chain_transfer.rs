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

//! Cross-chain transfer tests within Kusama network.
use frame_support::assert_ok;
use node_primitives::CurrencyId;
use orml_traits::MultiCurrency;
use sp_runtime::traits::AccountIdConversion;
use xcm::{v3::prelude::*, VersionedMultiAssets, VersionedMultiLocation};
use xcm_emulator::{ParaId, TestExt};

use crate::config::{Bifrost, PolkadotNet, ALICE, BOB, DOT_DECIMALS, DOT_TOKEN_ID};
use bifrost_polkadot_runtime::{
	AccountId, Balances, RelayCurrencyId, RuntimeOrigin, Tokens, XTokens,
};

#[test]
fn transfer_dot_between_bifrost_and_relay_chain() {
	sp_io::TestExternalities::default().execute_with(|| {
		PolkadotNet::execute_with(|| {
			// Polkadot alice(100 DOT) -> Bifrost bob 10 DOT
			assert_ok!(polkadot_runtime::XcmPallet::reserve_transfer_assets(
				polkadot_runtime::RuntimeOrigin::signed(ALICE.into()),
				Box::new(VersionedMultiLocation::V3(X1(Parachain(2030)).into())),
				Box::new(VersionedMultiLocation::V3(
					X1(Junction::AccountId32 { id: BOB, network: None }).into()
				)),
				Box::new(VersionedMultiAssets::V3((Here, 10 * DOT_DECIMALS).into())),
				0,
			));

			//  Polkadot alice 90 DOT
			assert_eq!(Balances::free_balance(&AccountId::from(ALICE)), 90 * DOT_DECIMALS);
			// Parachain account 10 DOT
			let parachain_account: AccountId = ParaId::from(2030).into_account_truncating();
			assert_eq!(Balances::free_balance(parachain_account), 10 * DOT_DECIMALS);
		});

		Bifrost::execute_with(|| {
			assert_eq!(
				Tokens::free_balance(RelayCurrencyId::get(), &AccountId::from(BOB)),
				99992960800
			);
			// Bifrost bob (9.9 DOT) -> Polkadot BoB 2 DOT
			assert_ok!(XTokens::transfer(
				RuntimeOrigin::signed(BOB.into()),
				CurrencyId::Token2(DOT_TOKEN_ID),
				2 * DOT_DECIMALS,
				Box::new(xcm::VersionedMultiLocation::V3(MultiLocation::new(
					1,
					X1(Junction::AccountId32 { id: BOB, network: None })
				))),
				xcm_emulator::Unlimited
			));

			// Bifrost bob 7.9 DOT
			assert_eq!(
				Tokens::free_balance(RelayCurrencyId::get(), &AccountId::from(BOB)),
				79992960800,
			);
		});

		PolkadotNet::execute_with(|| {
			// Parachain account 8 DOT
			let parachain_account: AccountId = ParaId::from(2030).into_account_truncating();
			assert_eq!(Balances::free_balance(parachain_account), 8 * DOT_DECIMALS);

			// Polkadot bob 1.9 DOT
			assert_eq!(
				polkadot_runtime::Balances::free_balance(&AccountId::from(BOB)),
				19635578476,
			);
		});
	})
}
