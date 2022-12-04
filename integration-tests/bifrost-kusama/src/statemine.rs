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
use xcm::{latest::prelude::*, v1::MultiAssets, VersionedMultiAssets, VersionedMultiLocation};
use xcm_emulator::TestExt;

const USDT: u128 = 1_000_000;

#[test]
fn cross_usdt() {
	sp_io::TestExternalities::default().execute_with(|| {
		TestNet::reset();

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

			// v1
			let location = VersionedMultiLocation::V1(MultiLocation {
				parents: 1,
				interior: Junctions::X3(Parachain(1000), PalletInstance(50), GeneralIndex(1984)),
			});

			assert_ok!(AssetRegistry::register_multilocation(
				RuntimeOrigin::root(),
				CurrencyId::Token2(0),
				Box::new(location.clone()),
				0
			));
		});
		// env_logger::init();
		Statemine::execute_with(|| {
			use frame_support::traits::Currency;
			use statemine_runtime::*;

			let origin = RuntimeOrigin::signed(ALICE.into());

			statemine_runtime::Balances::make_free_balance_be(&ALICE.into(), 10 * 10_000_000_000);

			// need to have some KSM to be able to receive user assets
			statemine_runtime::Balances::make_free_balance_be(
				&Sibling::from(2001).into_account_truncating(),
				10 * 10_000_000_000,
			);

			assert_ok!(Assets::create(origin.clone(), 1984, MultiAddress::Id(ALICE.into()), 10));
			assert_ok!(Assets::mint(
				origin.clone(),
				1984,
				MultiAddress::Id(ALICE.into()),
				100 * USDT
			));
			assert_eq!(Assets::balance(1984, sp_runtime::AccountId32::from(ALICE)), 100 * USDT);

			System::reset_events();

			let para_acc: AccountId = Sibling::from(2001).into_account_truncating();
			println!("{:?}", para_acc);

			let assets = MultiAssets::from(vec![MultiAsset::from((
				xcm::v1::AssetId::Concrete(MultiLocation::new(
					0,
					X2(PalletInstance(50), GeneralIndex(1984)),
				)),
				xcm::v1::Fungibility::from(10 * USDT),
			))]);

			assert_ok!(
				pallet_xcm::Pallet::<statemine_runtime::Runtime>::limited_reserve_transfer_assets(
					origin.clone(),
					Box::new(VersionedMultiLocation::V1(MultiLocation::new(
						1,
						X1(Parachain(2001))
					))),
					Box::new(VersionedMultiLocation::V1(MultiLocation::new(
						0,
						X1(Junction::AccountId32 { id: ALICE, network: NetworkId::Any })
					))),
					Box::new(VersionedMultiAssets::V1(assets)),
					0,
					WeightLimit::Unlimited,
				)
			);
			println!("{:?}", System::events());
		});

		Bifrost::execute_with(|| {
			assert_eq!(
				Tokens::free_balance(CurrencyId::Token2(0), &AccountId::from(ALICE),),
				10 * USDT
			);

			assert_ok!(XcmInterface::transfer_statemine_assets(
				RuntimeOrigin::signed(ALICE.into()),
				5 * USDT,
				1984,
				Some(sp_runtime::AccountId32::from(ALICE))
			));
			println!("{:?}", System::events());

			assert_eq!(
				Tokens::free_balance(CurrencyId::Token2(0), &AccountId::from(ALICE),),
				5 * USDT
			);
		});
		Statemine::execute_with(|| {
			use statemine_runtime::*;
			assert_eq!(Assets::balance(1984, sp_runtime::AccountId32::from(ALICE)), 95 * USDT);
		})
	})
}
