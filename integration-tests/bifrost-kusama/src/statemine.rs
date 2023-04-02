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

use crate::{kusama_integration_tests::*, kusama_test_net::*};
use bifrost_asset_registry::AssetMetadata;
use frame_support::assert_ok;
use polkadot_parachain::primitives::Sibling;
use sp_runtime::traits::AccountIdConversion;
use xcm::{
	v3::{prelude::*, Weight},
	VersionedMultiAssets, VersionedMultiLocation,
};
use xcm_emulator::TestExt;

const USDT: u128 = 1_000_000;

#[test]
fn cross_usdt() {
	sp_io::TestExternalities::default().execute_with(|| {
		Bifrost::execute_with(|| {
			let metadata = AssetMetadata {
				name: b"USDT".to_vec(),
				symbol: b"USDT".to_vec(),
				decimals: 6,
				minimal_balance: 10,
			};

			assert_ok!(AssetRegistry::register_token_metadata(
				RuntimeOrigin::root(),
				Box::new(metadata.clone())
			));

			let location = VersionedMultiLocation::V3(MultiLocation {
				parents: 1,
				interior: X3(Parachain(1000), PalletInstance(50), GeneralIndex(1984)),
			});

			assert_ok!(AssetRegistry::register_multilocation(
				RuntimeOrigin::root(),
				CurrencyId::Token2(0),
				Box::new(location.clone()),
				Weight::zero()
			));
		});

		Statemine::execute_with(|| {
			use frame_support::traits::Currency;
			use statemine_runtime::*;

			let origin = RuntimeOrigin::signed(ALICE.into());

			statemine_runtime::Balances::make_free_balance_be(&ALICE.into(), 10 * KSM_DECIMALS);
			statemine_runtime::Balances::make_free_balance_be(&BOB.into(), 10 * KSM_DECIMALS);

			// need to have some KSM to be able to receive user assets
			statemine_runtime::Balances::make_free_balance_be(
				&Sibling::from(2001).into_account_truncating(),
				10 * KSM_DECIMALS,
			);

			assert_ok!(Assets::create(
				statemine_runtime::RuntimeOrigin::signed(ALICE.into()),
				codec::Compact(1984),
				MultiAddress::Id(ALICE.into()),
				10
			));

			assert_ok!(Assets::set_metadata(
				statemine_runtime::RuntimeOrigin::signed(ALICE.into()),
				codec::Compact(1984),
				b"USDT".to_vec(),
				b"USDT".to_vec(),
				6
			));

			assert_ok!(Assets::mint(
				statemine_runtime::RuntimeOrigin::signed(ALICE.into()),
				codec::Compact(1984),
				MultiAddress::Id(ALICE.into()),
				100 * USDT
			));
			assert_eq!(Assets::balance(1984, sp_runtime::AccountId32::from(ALICE)), 100 * USDT);

			let para_acc: AccountId = Sibling::from(2001).into_account_truncating();
			println!("{:?}", para_acc);

			let assets = MultiAssets::from(vec![MultiAsset::from((
				Concrete(MultiLocation::new(0, X2(PalletInstance(50), GeneralIndex(1984)))),
				Fungibility::from(10 * USDT),
			))]);

			assert_ok!(
				pallet_xcm::Pallet::<statemine_runtime::Runtime>::limited_reserve_transfer_assets(
					origin.clone(),
					Box::new(VersionedMultiLocation::V3(MultiLocation::new(
						1,
						X1(Parachain(2001))
					))),
					Box::new(VersionedMultiLocation::V3(MultiLocation::new(
						0,
						X1(Junction::AccountId32 { id: BOB, network: None })
					))),
					Box::new(VersionedMultiAssets::V3(assets)),
					0,
					WeightLimit::Unlimited,
				)
			);
			assert_eq!(Assets::balance(1984, para_acc), 10 * USDT);
			System::reset_events();
		});

		Bifrost::execute_with(|| {
			assert_ok!(Tokens::deposit(
				CurrencyId::Token2(0),
				&sp_runtime::AccountId32::from(ALICE),
				10 * USDT
			));
			assert_ok!(XcmInterface::transfer_statemine_assets(
				RuntimeOrigin::signed(ALICE.into()),
				5 * USDT,
				1984,
				Some(sp_runtime::AccountId32::from(BOB))
			));

			assert_eq!(
				Tokens::free_balance(CurrencyId::Token2(0), &AccountId::from(ALICE),),
				5 * USDT
			);
		});
		Statemine::execute_with(|| {
			use statemine_runtime::*;
			println!("{:?}", System::events());
			assert_eq!(Assets::balance(1984, AccountId::from(BOB)), 5 * USDT);
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success {
					message_hash: _,
					weight: _
				})
			)));
		});
	})
}
