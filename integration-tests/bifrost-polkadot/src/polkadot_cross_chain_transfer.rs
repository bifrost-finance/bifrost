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
use bifrost_asset_registry::AssetMetadata;
use bifrost_polkadot_runtime::AssetRegistry;
use bifrost_runtime_common::millicent;
use frame_support::assert_ok;
use node_primitives::{CurrencyId, TokenSymbol};
use orml_traits::MultiCurrency;
use xcm::{latest::prelude::*, VersionedMultiAssets, VersionedMultiLocation};
use xcm_emulator::TestExt;

use crate::{polkadot_integration_tests::*, polkadot_test_net::*};

#[test]
fn transfer_from_relay_chain() {
	bifrost_register_asset(CurrencyId::Token(TokenSymbol::DOT));
	PolkadotNet::execute_with(|| {
		assert_ok!(polkadot_runtime::XcmPallet::reserve_transfer_assets(
			polkadot_runtime::Origin::signed(ALICE.into()),
			Box::new(VersionedMultiLocation::V1(X1(Parachain(2010)).into())),
			Box::new(VersionedMultiLocation::V1(
				X1(Junction::AccountId32 { id: BOB, network: NetworkId::Any }).into()
			)),
			Box::new(VersionedMultiAssets::V1((Here, dollar(RelayCurrencyId::get())).into())),
			0,
		));
	});

	Bifrost::execute_with(|| {
		assert_eq!(
			Tokens::free_balance(RelayCurrencyId::get(), &AccountId::from(BOB)),
			999990730400
		);
	});
}

#[test]
fn transfer_to_relay_chain() {
	Bifrost::execute_with(|| {
		assert_ok!(XTokens::transfer(
			Origin::signed(ALICE.into()),
			RelayCurrencyId::get(),
			dollar(RelayCurrencyId::get()),
			Box::new(xcm::VersionedMultiLocation::V1(MultiLocation::new(
				1,
				X1(Junction::AccountId32 { id: BOB, network: NetworkId::Any })
			))),
			4_000_000_000
		));
	});

	PolkadotNet::execute_with(|| {
		assert_eq!(polkadot_runtime::Balances::free_balance(&AccountId::from(BOB)), 999530582548);
	});
}

#[test]
fn transfer_to_sibling() {
	bifrost_register_asset(CurrencyId::Token2(DOT_TOKEN_ID));
	sibling_register_asset(CurrencyId::Token2(DOT_TOKEN_ID));

	Bifrost::execute_with(|| {
		assert_ok!(XTokens::transfer(
			Origin::signed(ALICE.into()),
			CurrencyId::Token2(DOT_TOKEN_ID),
			2 * dollar(CurrencyId::Token2(DOT_TOKEN_ID)),
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
			8 * dollar(CurrencyId::Token2(DOT_TOKEN_ID))
		);
	});
}

fn bifrost_register_asset(currency_id: CurrencyId) {
	Bifrost::execute_with(|| {
		assert_ok!(AssetRegistry::do_register_native_asset(
			currency_id,
			&MultiLocation::parent(),
			&AssetMetadata {
				name: currency_id.name().map(|s| s.as_bytes().to_vec()).unwrap_or_default(),
				symbol: currency_id.symbol().map(|s| s.as_bytes().to_vec()).unwrap_or_default(),
				decimals: currency_id.decimals().unwrap_or_default(),
				minimal_balance: 10 * millicent(currency_id),
			}
		));
	});
}

fn sibling_register_asset(currency_id: CurrencyId) {
	Sibling::execute_with(|| {
		assert_ok!(AssetRegistry::do_register_native_asset(
			currency_id,
			&MultiLocation::parent(),
			&AssetMetadata {
				name: currency_id.name().map(|s| s.as_bytes().to_vec()).unwrap_or_default(),
				symbol: currency_id.symbol().map(|s| s.as_bytes().to_vec()).unwrap_or_default(),
				decimals: currency_id.decimals().unwrap_or_default(),
				minimal_balance: 10 * millicent(currency_id),
			}
		));
	});
}
