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

use crate::kusama_integration_tests::*;
use bifrost_asset_registry::{
	AssetMetadata, AssetMetadatas, CurrencyIdToLocations, CurrencyIdToWeights, CurrencyMetadatas,
	Error, Event, LocationToCurrencyIds,
};
use frame_support::{assert_noop, assert_ok};
use xcm::{latest::prelude::*, VersionedMultiLocation};

#[test]
fn register_native_asset_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		ExtBuilder::default().build().execute_with(|| {
			let v0_location = VersionedMultiLocation::V0(xcm::v0::MultiLocation::X1(
				xcm::v0::Junction::Parachain(1000),
			));

			assert_ok!(AssetRegistry::register_native_asset(
				RuntimeOrigin::root(),
				CurrencyId::Token(TokenSymbol::DOT),
				Box::new(v0_location.clone()),
				Box::new(AssetMetadata {
					name: b"Token Name".to_vec(),
					symbol: b"TN".to_vec(),
					decimals: 12,
					minimal_balance: 1,
				})
			));
			System::assert_last_event(bifrost_kusama_runtime::RuntimeEvent::AssetRegistry(
				Event::AssetRegistered {
					asset_id: AssetIds::NativeAssetId(CurrencyId::Token(TokenSymbol::DOT)),
					metadata: AssetMetadata {
						name: b"Token Name".to_vec(),
						symbol: b"TN".to_vec(),
						decimals: 12,
						minimal_balance: 1,
					},
				},
			));

			assert_eq!(
				AssetMetadatas::<Runtime>::get(AssetIds::NativeAssetId(CurrencyId::Token(
					TokenSymbol::DOT
				))),
				Some(AssetMetadata {
					name: b"Token Name".to_vec(),
					symbol: b"TN".to_vec(),
					decimals: 12,
					minimal_balance: 1,
				})
			);
			// Can't duplicate
			assert_noop!(
				AssetRegistry::register_native_asset(
					RuntimeOrigin::root(),
					CurrencyId::Token(TokenSymbol::DOT),
					Box::new(v0_location.clone()),
					Box::new(AssetMetadata {
						name: b"Token Name".to_vec(),
						symbol: b"TN".to_vec(),
						decimals: 12,
						minimal_balance: 1,
					})
				),
				Error::<Runtime>::AssetIdExisted
			);
		});
	})
}

#[test]
fn update_native_asset_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		ExtBuilder::default().build().execute_with(|| {
			let v0_location = VersionedMultiLocation::V0(xcm::v0::MultiLocation::X1(
				xcm::v0::Junction::Parachain(1000),
			));
			assert_noop!(
				AssetRegistry::update_native_asset(
					RuntimeOrigin::root(),
					CurrencyId::Token(TokenSymbol::DOT),
					Box::new(v0_location.clone()),
					Box::new(AssetMetadata {
						name: b"New Token Name".to_vec(),
						symbol: b"NTN".to_vec(),
						decimals: 13,
						minimal_balance: 2,
					})
				),
				Error::<Runtime>::AssetIdNotExists
			);

			let new_location = VersionedMultiLocation::V0(xcm::v0::MultiLocation::X1(
				xcm::v0::Junction::Parachain(2000),
			));
			assert_ok!(AssetRegistry::register_native_asset(
				RuntimeOrigin::root(),
				CurrencyId::Token(TokenSymbol::DOT),
				Box::new(new_location.clone()),
				Box::new(AssetMetadata {
					name: b"Token Name".to_vec(),
					symbol: b"TN".to_vec(),
					decimals: 12,
					minimal_balance: 1,
				})
			));

			assert_ok!(AssetRegistry::update_native_asset(
				RuntimeOrigin::root(),
				CurrencyId::Token(TokenSymbol::DOT),
				Box::new(new_location.clone()),
				Box::new(AssetMetadata {
					name: b"New Token Name".to_vec(),
					symbol: b"NTN".to_vec(),
					decimals: 13,
					minimal_balance: 2,
				})
			));

			System::assert_last_event(bifrost_kusama_runtime::RuntimeEvent::AssetRegistry(
				Event::AssetUpdated {
					asset_id: AssetIds::NativeAssetId(CurrencyId::Token(TokenSymbol::DOT)),
					metadata: AssetMetadata {
						name: b"New Token Name".to_vec(),
						symbol: b"NTN".to_vec(),
						decimals: 13,
						minimal_balance: 2,
					},
				},
			));

			assert_eq!(
				AssetMetadatas::<Runtime>::get(AssetIds::NativeAssetId(CurrencyId::Token(
					TokenSymbol::DOT
				))),
				Some(AssetMetadata {
					name: b"New Token Name".to_vec(),
					symbol: b"NTN".to_vec(),
					decimals: 13,
					minimal_balance: 2,
				})
			);
		});
	})
}

#[test]
fn register_token_metadata() {
	sp_io::TestExternalities::default().execute_with(|| {
		ExtBuilder::default().build().execute_with(|| {
			let metadata = AssetMetadata {
				name: b"Bifrost Native Coin".to_vec(),
				symbol: b"BNC".to_vec(),
				decimals: 12,
				minimal_balance: 0,
			};

			assert_ok!(AssetRegistry::register_token_metadata(
				RuntimeOrigin::root(),
				Box::new(metadata.clone())
			));

			assert_eq!(
				CurrencyMetadatas::<Runtime>::get(CurrencyId::Token2(0)),
				Some(metadata.clone())
			)
		})
	})
}

#[test]
fn register_vtoken_metadata() {
	sp_io::TestExternalities::default().execute_with(|| {
		ExtBuilder::default().build().execute_with(|| {
			let metadata = AssetMetadata {
				name: b"Bifrost Native Coin".to_vec(),
				symbol: b"BNC".to_vec(),
				decimals: 12,
				minimal_balance: 0,
			};
			let v_metadata = AssetMetadata {
				name: b"Voucher BNC".to_vec(),
				symbol: b"vBNC".to_vec(),
				decimals: 12,
				minimal_balance: 0,
			};
			assert_noop!(
				AssetRegistry::register_vtoken_metadata(RuntimeOrigin::root(), 1),
				Error::<Runtime>::CurrencyIdNotExists
			);

			assert_ok!(AssetRegistry::register_token_metadata(
				RuntimeOrigin::root(),
				Box::new(metadata.clone())
			));

			assert_ok!(AssetRegistry::register_vtoken_metadata(RuntimeOrigin::root(), 0));

			assert_eq!(
				CurrencyMetadatas::<Runtime>::get(CurrencyId::VToken2(0)),
				Some(v_metadata.clone())
			)
		})
	})
}

#[test]
fn register_vsbond_metadata() {
	sp_io::TestExternalities::default().execute_with(|| {
		ExtBuilder::default().build().execute_with(|| {
			let metadata = AssetMetadata {
				name: b"KSM Native Token".to_vec(),
				symbol: b"KSM".to_vec(),
				decimals: 12,
				minimal_balance: 0,
			};
			let name = "vsBOND-KSM-2001-10-20".as_bytes().to_vec();
			let v_metadata = AssetMetadata {
				name: name.clone(),
				symbol: name,
				decimals: 12,
				minimal_balance: 0,
			};
			assert_noop!(
				AssetRegistry::register_vtoken_metadata(RuntimeOrigin::root(), 1),
				Error::<Runtime>::CurrencyIdNotExists
			);

			assert_ok!(AssetRegistry::register_token_metadata(
				RuntimeOrigin::root(),
				Box::new(metadata.clone())
			));

			assert_ok!(AssetRegistry::register_vsbond_metadata(
				RuntimeOrigin::root(),
				0,
				2001,
				10,
				20
			));

			assert_eq!(
				CurrencyMetadatas::<Runtime>::get(CurrencyId::VSBond2(0, 2001, 10, 20)),
				Some(v_metadata.clone())
			)
		})
	})
}

#[test]
fn register_multilocation() {
	sp_io::TestExternalities::default().execute_with(|| {
		ExtBuilder::default().build().execute_with(|| {
			let metadata = AssetMetadata {
				name: b"Bifrost Native Coin".to_vec(),
				symbol: b"BNC".to_vec(),
				decimals: 12,
				minimal_balance: 0,
			};
			// v1
			let location = VersionedMultiLocation::V1(MultiLocation {
				parents: 1,
				interior: xcm::v1::Junctions::X1(xcm::v1::Junction::Parachain(2001)),
			});
			let multi_location: MultiLocation = location.clone().try_into().unwrap();

			assert_noop!(
				AssetRegistry::register_multilocation(
					RuntimeOrigin::root(),
					CurrencyId::Token2(0),
					Box::new(location.clone()),
					2000_000_000
				),
				Error::<Runtime>::CurrencyIdNotExists
			);

			assert_ok!(AssetRegistry::register_token_metadata(
				RuntimeOrigin::root(),
				Box::new(metadata.clone())
			));

			assert_ok!(AssetRegistry::register_multilocation(
				RuntimeOrigin::root(),
				CurrencyId::Token2(0),
				Box::new(location.clone()),
				2000_000_000
			));

			assert_noop!(
				AssetRegistry::register_multilocation(
					RuntimeOrigin::root(),
					CurrencyId::Token2(0),
					Box::new(location.clone()),
					2000_000_000
				),
				Error::<Runtime>::CurrencyIdExisted
			);

			assert_eq!(
				LocationToCurrencyIds::<Runtime>::get(multi_location.clone()),
				Some(CurrencyId::Token2(0))
			);
			assert_eq!(
				CurrencyIdToLocations::<Runtime>::get(CurrencyId::Token2(0)),
				Some(multi_location.clone())
			);
			assert_eq!(
				CurrencyIdToWeights::<Runtime>::get(CurrencyId::Token2(0)),
				Some(2000_000_000)
			);
		})
	})
}
