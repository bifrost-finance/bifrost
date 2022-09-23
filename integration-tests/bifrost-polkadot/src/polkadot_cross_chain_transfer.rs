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
use xcm::{latest::prelude::*, VersionedMultiAssets, VersionedMultiLocation};
use xcm_emulator::TestExt;

use crate::{
	polkadot_integration_tests::{ALICE, BOB},
	polkadot_test_net::{register_token2_asset, Bifrost, PolkadotNet, DOT_TOKEN_ID},
};
use bifrost_polkadot_runtime::{
	AccountId, Balances, Origin, RelayCurrencyId, Runtime, Tokens, XTokens,
};
use bifrost_runtime_common::dollar;

#[test]
fn transfer_from_relay_chain() {
	sp_io::TestExternalities::default().execute_with(|| {
		register_token2_asset();
		PolkadotNet::execute_with(|| {
			assert_ok!(polkadot_runtime::XcmPallet::reserve_transfer_assets(
				polkadot_runtime::Origin::signed(ALICE.into()),
				Box::new(VersionedMultiLocation::V1(X1(Parachain(2010)).into())),
				Box::new(VersionedMultiLocation::V1(
					X1(Junction::AccountId32 { id: BOB, network: NetworkId::Any }).into()
				)),
				Box::new(VersionedMultiAssets::V1(
					(Here, 10 * dollar::<Runtime>(RelayCurrencyId::get())).into()
				)),
				0,
			));
			assert_eq!(
				Balances::free_balance(&AccountId::from(ALICE)),
				90 * dollar::<Runtime>(RelayCurrencyId::get())
			);
		});

		Bifrost::execute_with(|| {
			assert_eq!(
				Tokens::free_balance(RelayCurrencyId::get(), &AccountId::from(BOB)),
				9999990730400
			);
		});
	})
}

#[test]
fn transfer_to_relay_chain() {
	sp_io::TestExternalities::default().execute_with(|| {
		register_token2_asset();

		Bifrost::execute_with(|| {
			assert_ok!(XTokens::transfer(
				Origin::signed(ALICE.into()),
				RelayCurrencyId::get(),
				2 * dollar::<Runtime>(RelayCurrencyId::get()),
				Box::new(xcm::VersionedMultiLocation::V1(MultiLocation::new(
					1,
					X1(Junction::AccountId32 { id: BOB, network: NetworkId::Any })
				))),
				4_000_000_000
			));
			assert_eq!(
				Tokens::free_balance(RelayCurrencyId::get(), &AccountId::from(ALICE)),
				8 * dollar::<Runtime>(RelayCurrencyId::get()),
			);
		});

		PolkadotNet::execute_with(|| {
			assert_eq!(
				polkadot_runtime::Balances::free_balance(&AccountId::from(BOB)),
				19530582548
			);
		});
	})
}

#[test]
fn transfer_to_sibling() {
	sp_io::TestExternalities::default().execute_with(|| {
		register_token2_asset();

		Bifrost::execute_with(|| {
			assert_ok!(XTokens::transfer(
				Origin::signed(ALICE.into()),
				CurrencyId::Token2(DOT_TOKEN_ID),
				2 * dollar::<Runtime>(CurrencyId::Token2(DOT_TOKEN_ID)),
				Box::new(
					MultiLocation::new(
						1,
						X2(
							Parachain(2000),
							Junction::AccountId32 { network: NetworkId::Any, id: BOB.into() }
						)
					)
					.into()
				),
				1_000_000_000,
			));

			assert_eq!(
				Tokens::free_balance(CurrencyId::Token2(DOT_TOKEN_ID), &AccountId::from(ALICE)),
				8 * dollar::<Runtime>(CurrencyId::Token2(DOT_TOKEN_ID))
			);
		});
	})
}
