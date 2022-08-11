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
use bifrost_asset_registry::{
	AssetMetadata, AssetMetadatas, CurrencyIdToLocations, Error, Event, LocationToCurrencyIds,
};
use frame_support::{assert_noop, assert_ok};
use xcm::{latest::prelude::*, VersionedMultiLocation};
use xcm_emulator::TestExt;

#[test]
fn register_foreign_asset_work() {
	Bifrost::execute_with(|| {
		let v0_location = VersionedMultiLocation::V0(xcm::v0::MultiLocation::X1(
			xcm::v0::Junction::Parachain(1000),
		));

		assert_ok!(AssetRegistry::register_foreign_asset(
			Origin::root(),
			Box::new(v0_location.clone()),
			Box::new(AssetMetadata {
				name: b"Token Name".to_vec(),
				symbol: b"TN".to_vec(),
				decimals: 12,
				minimal_balance: 1,
			})
		));

		let location: MultiLocation = v0_location.try_into().unwrap();
		System::assert_last_event(bifrost_kusama_runtime::Event::AssetRegistry(
			Event::ForeignAssetRegistered {
				asset_id: 0,
				asset_address: location.clone(),
				metadata: AssetMetadata {
					name: b"Token Name".to_vec(),
					symbol: b"TN".to_vec(),
					decimals: 12,
					minimal_balance: 1,
				},
			},
		));

		assert_eq!(
			CurrencyIdToLocations::<Runtime>::get(CurrencyId::ForeignAsset(0)),
			Some(location.clone())
		);
		assert_eq!(
			AssetMetadatas::<Runtime>::get(AssetIds::ForeignAssetId(0)),
			Some(AssetMetadata {
				name: b"Token Name".to_vec(),
				symbol: b"TN".to_vec(),
				decimals: 12,
				minimal_balance: 1,
			})
		);
		assert_eq!(
			LocationToCurrencyIds::<Runtime>::get(location),
			Some(CurrencyId::ForeignAsset(0))
		);
	});
}

#[test]
fn update_foreign_asset_work() {
	ExtBuilder::default().build().execute_with(|| {
		let v0_location = VersionedMultiLocation::V0(xcm::v0::MultiLocation::X1(
			xcm::v0::Junction::Parachain(1000),
		));

		assert_ok!(AssetRegistry::register_foreign_asset(
			Origin::root(),
			Box::new(v0_location.clone()),
			Box::new(AssetMetadata {
				name: b"Token Name".to_vec(),
				symbol: b"TN".to_vec(),
				decimals: 12,
				minimal_balance: 1,
			})
		));

		assert_ok!(AssetRegistry::update_foreign_asset(
			Origin::root(),
			0,
			Box::new(v0_location.clone()),
			Box::new(AssetMetadata {
				name: b"New Token Name".to_vec(),
				symbol: b"NTN".to_vec(),
				decimals: 13,
				minimal_balance: 2,
			})
		));

		let location: MultiLocation = v0_location.try_into().unwrap();
		System::assert_last_event(bifrost_kusama_runtime::Event::AssetRegistry(
			Event::ForeignAssetUpdated {
				asset_id: 0,
				asset_address: location.clone(),
				metadata: AssetMetadata {
					name: b"New Token Name".to_vec(),
					symbol: b"NTN".to_vec(),
					decimals: 13,
					minimal_balance: 2,
				},
			},
		));

		assert_eq!(
			AssetMetadatas::<Runtime>::get(AssetIds::ForeignAssetId(0)),
			Some(AssetMetadata {
				name: b"New Token Name".to_vec(),
				symbol: b"NTN".to_vec(),
				decimals: 13,
				minimal_balance: 2,
			})
		);
		assert_eq!(
			CurrencyIdToLocations::<Runtime>::get(CurrencyId::ForeignAsset(0)),
			Some(location.clone())
		);
		assert_eq!(
			LocationToCurrencyIds::<Runtime>::get(location.clone()),
			Some(CurrencyId::ForeignAsset(0))
		);

		// modify location
		let new_location = VersionedMultiLocation::V0(xcm::v0::MultiLocation::X1(
			xcm::v0::Junction::Parachain(2000),
		));
		assert_ok!(AssetRegistry::update_foreign_asset(
			Origin::root(),
			0,
			Box::new(new_location.clone()),
			Box::new(AssetMetadata {
				name: b"New Token Name".to_vec(),
				symbol: b"NTN".to_vec(),
				decimals: 13,
				minimal_balance: 2,
			})
		));
		assert_eq!(
			AssetMetadatas::<Runtime>::get(AssetIds::ForeignAssetId(0)),
			Some(AssetMetadata {
				name: b"New Token Name".to_vec(),
				symbol: b"NTN".to_vec(),
				decimals: 13,
				minimal_balance: 2,
			})
		);
		let new_location: MultiLocation = new_location.try_into().unwrap();
		assert_eq!(
			CurrencyIdToLocations::<Runtime>::get(CurrencyId::ForeignAsset(0)),
			Some(new_location.clone())
		);
		assert_eq!(LocationToCurrencyIds::<Runtime>::get(location), None);
		assert_eq!(
			LocationToCurrencyIds::<Runtime>::get(new_location),
			Some(CurrencyId::ForeignAsset(0))
		);
	});
}

#[test]
fn register_native_asset_works() {
	ExtBuilder::default().build().execute_with(|| {
		let v0_location = VersionedMultiLocation::V0(xcm::v0::MultiLocation::X1(
			xcm::v0::Junction::Parachain(1000),
		));

		assert_ok!(AssetRegistry::register_native_asset(
			Origin::root(),
			CurrencyId::Token(TokenSymbol::DOT),
			Box::new(v0_location.clone()),
			Box::new(AssetMetadata {
				name: b"Token Name".to_vec(),
				symbol: b"TN".to_vec(),
				decimals: 12,
				minimal_balance: 1,
			})
		));
		System::assert_last_event(bifrost_kusama_runtime::Event::AssetRegistry(
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
				Origin::root(),
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
}

#[test]
fn update_native_asset_works() {
	ExtBuilder::default().build().execute_with(|| {
		let v0_location = VersionedMultiLocation::V0(xcm::v0::MultiLocation::X1(
			xcm::v0::Junction::Parachain(1000),
		));
		assert_noop!(
			AssetRegistry::update_native_asset(
				Origin::root(),
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
			Origin::root(),
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
			Origin::root(),
			CurrencyId::Token(TokenSymbol::DOT),
			Box::new(new_location.clone()),
			Box::new(AssetMetadata {
				name: b"New Token Name".to_vec(),
				symbol: b"NTN".to_vec(),
				decimals: 13,
				minimal_balance: 2,
			})
		));

		System::assert_last_event(bifrost_kusama_runtime::Event::AssetRegistry(
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
}
