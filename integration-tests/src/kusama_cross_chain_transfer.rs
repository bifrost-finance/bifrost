// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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
use orml_traits::MultiCurrency;
use xcm::{latest::prelude::*, VersionedMultiAssets, VersionedMultiLocation};
use xcm_emulator::TestExt;

use crate::{integration_tests::*, kusama_test_net::*};

#[test]
fn transfer_from_relay_chain() {
	KusamaNet::execute_with(|| {
		assert_ok!(kusama_runtime::XcmPallet::reserve_transfer_assets(
			kusama_runtime::Origin::signed(ALICE.into()),
			Box::new(VersionedMultiLocation::V1(X1(Parachain(2001)).into())),
			Box::new(VersionedMultiLocation::V1(
				X1(Junction::AccountId32 { id: BOB, network: NetworkId::Any }).into()
			)),
			Box::new(VersionedMultiAssets::V1((Here, DOLLARS).into())),
			0,
		));
	});

	Bifrost::execute_with(|| {
		assert_eq!(
			Tokens::free_balance(RelayCurrencyId::get(), &AccountId::from(BOB)),
			999_936_000_000
		);
	});
}

#[test]
fn transfer_to_relay_chain() {
	Bifrost::execute_with(|| {
		assert_ok!(XTokens::transfer(
			Origin::signed(ALICE.into()),
			RelayCurrencyId::get(),
			DOLLARS,
			Box::new(MultiLocation::new(
				1,
				X1(Junction::AccountId32 { id: BOB, network: NetworkId::Any })
			)),
			4_000_000_000
		));
	});

	KusamaNet::execute_with(|| {
		assert_eq!(kusama_runtime::Balances::free_balance(&AccountId::from(BOB)), 999_893_333_340);
	});
}
