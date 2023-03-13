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
use bifrost_kusama_runtime::Runtime;
use frame_support::assert_ok;
use orml_traits::MultiCurrency;
use xcm::{v3::prelude::*, VersionedMultiAssets, VersionedMultiLocation};
use xcm_emulator::{ParaId, TestExt};

use crate::{kusama_integration_tests::*, kusama_test_net::*};

#[test]
fn transfer_ksm_between_bifrost_and_relay_chain() {
	sp_io::TestExternalities::default().execute_with(|| {
		KusamaNet::execute_with(|| {
			// Kusama alice(100 KSM) -> Bifrost bob 10 KSM
			assert_ok!(kusama_runtime::XcmPallet::reserve_transfer_assets(
				kusama_runtime::RuntimeOrigin::signed(ALICE.into()),
				Box::new(VersionedMultiLocation::V3(X1(Parachain(2001)).into())),
				Box::new(VersionedMultiLocation::V3(
					X1(Junction::AccountId32 { id: BOB, network: None }).into()
				)),
				Box::new(VersionedMultiAssets::V3(
					(Here, 10 * dollar::<Runtime>(RelayCurrencyId::get())).into()
				)),
				0,
			));

			//  Bifrost alice 90 KSM
			assert_eq!(
				Balances::free_balance(&AccountId::from(ALICE)),
				90 * dollar::<Runtime>(RelayCurrencyId::get())
			);
			// Parachain account 10 KSM
			let parachain_account: AccountId = ParaId::from(2001).into_account_truncating();
			assert_eq!(
				Balances::free_balance(parachain_account),
				10 * dollar::<Runtime>(RelayCurrencyId::get())
			);
		});

		Bifrost::execute_with(|| {
			// Bifrost bob 9.9 KSM
			assert_eq!(
				Tokens::free_balance(RelayCurrencyId::get(), &AccountId::from(BOB)),
				9999919872000
			);

			// Bifrost bob 9.9 KSM -> Kusama bob 2 KSM
			assert_ok!(XTokens::transfer(
				RuntimeOrigin::signed(BOB.into()),
				RelayCurrencyId::get(),
				2 * dollar::<Runtime>(RelayCurrencyId::get()),
				Box::new(xcm::VersionedMultiLocation::V3(MultiLocation::new(
					1,
					X1(Junction::AccountId32 { id: BOB, network: None })
				))),
				xcm_emulator::Unlimited
			));
			// Bifrost bob 7.9 KSM
			assert_eq!(
				Tokens::free_balance(RelayCurrencyId::get(), &AccountId::from(BOB)),
				7999919872000
			);
		});

		KusamaNet::execute_with(|| {
			// Parachain account 8 KSM
			let parachain_account: AccountId = ParaId::from(2001).into_account_truncating();
			assert_eq!(
				Balances::free_balance(parachain_account),
				8 * dollar::<Runtime>(RelayCurrencyId::get())
			);

			//  Bifrost bob 1.9 KSM
			assert_eq!(Balances::free_balance(&AccountId::from(BOB)), 1999909712564);
		});
	})
}
