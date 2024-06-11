// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
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

//! Unit tests for asset registry module.

#![cfg(test)]

use super::*;
use bifrost_primitives::TokenSymbol;
use frame_support::{assert_noop, assert_ok};
use mock::{
	AssetRegistry, CouncilAccount, ExtBuilder, Runtime, RuntimeEvent, RuntimeOrigin, System,
};
use xcm::prelude::*;

#[test]
fn versioned_multi_location_convert_work() {
	ExtBuilder::default().build().execute_with(|| {
		// V3
		let v3_location =
			VersionedMultiLocation::V3(MultiLocation::from(X1(Junction::Parachain(1000))));
		let location: MultiLocation = v3_location.try_into().unwrap();
		assert_eq!(
			location,
			MultiLocation { parents: 0, interior: Junctions::X1(Junction::Parachain(1000)) }
		);

		// V3
		let v3_location = VersionedMultiLocation::V3(MultiLocation {
			parents: 0,
			interior: Junctions::X1(Junction::Parachain(1000)),
		});
		let location: MultiLocation = v3_location.clone().try_into().unwrap();
		assert_eq!(
			location,
			MultiLocation { parents: 0, interior: Junctions::X1(Junction::Parachain(1000)) }
		);

		// handle all of VersionedMultiLocation
		assert!(match v3_location {
			VersionedMultiLocation::V3 { .. } => true,
			_ => false,
		});
	});
}

#[test]
fn register_native_asset_works() {
	ExtBuilder::default().build().execute_with(|| {
		let v3_location =
			VersionedMultiLocation::V3(MultiLocation::from(X1(Junction::Parachain(1000))));

		assert_ok!(AssetRegistry::register_native_asset(
			RuntimeOrigin::signed(CouncilAccount::get()),
			CurrencyId::Token(TokenSymbol::DOT),
			Box::new(v3_location.clone()),
			Box::new(AssetMetadata {
				name: b"Token Name".to_vec(),
				symbol: b"TN".to_vec(),
				decimals: 12,
				minimal_balance: 1,
			})
		));
		System::assert_last_event(RuntimeEvent::AssetRegistry(crate::Event::AssetRegistered {
			asset_id: AssetIds::NativeAssetId(CurrencyId::Token(TokenSymbol::DOT)),
			metadata: AssetMetadata {
				name: b"Token Name".to_vec(),
				symbol: b"TN".to_vec(),
				decimals: 12,
				minimal_balance: 1,
			},
		}));

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
				RuntimeOrigin::signed(CouncilAccount::get()),
				CurrencyId::Token(TokenSymbol::DOT),
				Box::new(v3_location),
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
	let v3_location =
		VersionedMultiLocation::V3(MultiLocation::from(X1(Junction::Parachain(1000))));

	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			AssetRegistry::update_native_asset(
				RuntimeOrigin::signed(CouncilAccount::get()),
				CurrencyId::Token(TokenSymbol::DOT),
				Box::new(v3_location.clone()),
				Box::new(AssetMetadata {
					name: b"New Token Name".to_vec(),
					symbol: b"NTN".to_vec(),
					decimals: 13,
					minimal_balance: 2,
				})
			),
			Error::<Runtime>::AssetIdNotExists
		);

		assert_ok!(AssetRegistry::register_native_asset(
			RuntimeOrigin::signed(CouncilAccount::get()),
			CurrencyId::Token(TokenSymbol::DOT),
			Box::new(v3_location.clone()),
			Box::new(AssetMetadata {
				name: b"Token Name".to_vec(),
				symbol: b"TN".to_vec(),
				decimals: 12,
				minimal_balance: 1,
			})
		));

		assert_ok!(AssetRegistry::update_native_asset(
			RuntimeOrigin::signed(CouncilAccount::get()),
			CurrencyId::Token(TokenSymbol::DOT),
			Box::new(v3_location.clone()),
			Box::new(AssetMetadata {
				name: b"New Token Name".to_vec(),
				symbol: b"NTN".to_vec(),
				decimals: 13,
				minimal_balance: 2,
			})
		));

		System::assert_last_event(RuntimeEvent::AssetRegistry(crate::Event::AssetUpdated {
			asset_id: AssetIds::NativeAssetId(CurrencyId::Token(TokenSymbol::DOT)),
			metadata: AssetMetadata {
				name: b"New Token Name".to_vec(),
				symbol: b"NTN".to_vec(),
				decimals: 13,
				minimal_balance: 2,
			},
		}));

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

#[test]
fn register_token_metadata_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let metadata = AssetMetadata {
			name: b"Bifrost Native Coin".to_vec(),
			symbol: b"BNC".to_vec(),
			decimals: 12,
			minimal_balance: 0,
		};

		assert_ok!(AssetRegistry::register_token_metadata(
			RuntimeOrigin::signed(CouncilAccount::get()),
			Box::new(metadata.clone())
		));

		assert_eq!(CurrencyMetadatas::<Runtime>::get(CurrencyId::Token2(0)), Some(metadata.clone()))
	})
}

#[test]
fn register_vtoken_metadata_should_work() {
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
			AssetRegistry::register_vtoken_metadata(
				RuntimeOrigin::signed(CouncilAccount::get()),
				1
			),
			Error::<Runtime>::CurrencyIdNotExists
		);

		assert_ok!(AssetRegistry::register_token_metadata(
			RuntimeOrigin::signed(CouncilAccount::get()),
			Box::new(metadata.clone())
		));

		assert_ok!(AssetRegistry::register_vtoken_metadata(
			RuntimeOrigin::signed(CouncilAccount::get()),
			0
		));

		assert_eq!(
			CurrencyMetadatas::<Runtime>::get(CurrencyId::VToken2(0)),
			Some(v_metadata.clone())
		)
	})
}

#[test]
fn register_vstoken_metadata_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let metadata = AssetMetadata {
			name: b"KSM Native Token".to_vec(),
			symbol: b"KSM".to_vec(),
			decimals: 12,
			minimal_balance: 0,
		};
		let v_metadata = AssetMetadata {
			name: b"Voucher Slot KSM".to_vec(),
			symbol: b"vsKSM".to_vec(),
			decimals: 12,
			minimal_balance: 0,
		};
		assert_noop!(
			AssetRegistry::register_vtoken_metadata(
				RuntimeOrigin::signed(CouncilAccount::get()),
				1
			),
			Error::<Runtime>::CurrencyIdNotExists
		);

		assert_ok!(AssetRegistry::register_token_metadata(
			RuntimeOrigin::signed(CouncilAccount::get()),
			Box::new(metadata.clone())
		));

		assert_ok!(AssetRegistry::register_vstoken_metadata(
			RuntimeOrigin::signed(CouncilAccount::get()),
			0
		));

		assert_eq!(
			CurrencyMetadatas::<Runtime>::get(CurrencyId::VSToken2(0)),
			Some(v_metadata.clone())
		)
	})
}

#[test]
fn register_vsbond_metadata_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let metadata = AssetMetadata {
			name: b"KSM Native Token".to_vec(),
			symbol: b"KSM".to_vec(),
			decimals: 12,
			minimal_balance: 0,
		};
		let name = "vsBOND-KSM-2001-10-20".as_bytes().to_vec();
		let v_metadata =
			AssetMetadata { name: name.clone(), symbol: name, decimals: 12, minimal_balance: 0 };
		assert_noop!(
			AssetRegistry::register_vtoken_metadata(
				RuntimeOrigin::signed(CouncilAccount::get()),
				1
			),
			Error::<Runtime>::CurrencyIdNotExists
		);

		assert_ok!(AssetRegistry::register_token_metadata(
			RuntimeOrigin::signed(CouncilAccount::get()),
			Box::new(metadata.clone())
		));

		assert_ok!(AssetRegistry::register_vsbond_metadata(
			RuntimeOrigin::signed(CouncilAccount::get()),
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
}

#[test]
fn register_multilocation_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let metadata = AssetMetadata {
			name: b"Bifrost Native Coin".to_vec(),
			symbol: b"BNC".to_vec(),
			decimals: 12,
			minimal_balance: 0,
		};
		// V3
		let location = VersionedMultiLocation::V3(MultiLocation {
			parents: 1,
			interior: Junctions::X1(Junction::Parachain(2001)),
		});
		let multi_location: MultiLocation = location.clone().try_into().unwrap();

		assert_noop!(
			AssetRegistry::register_multilocation(
				RuntimeOrigin::signed(CouncilAccount::get()),
				CurrencyId::Token2(0),
				Box::new(location.clone()),
				Weight::from_parts(2000_000_000, 0)
			),
			Error::<Runtime>::CurrencyIdNotExists
		);

		assert_ok!(AssetRegistry::register_token_metadata(
			RuntimeOrigin::signed(CouncilAccount::get()),
			Box::new(metadata.clone())
		));

		assert_ok!(AssetRegistry::register_multilocation(
			RuntimeOrigin::signed(CouncilAccount::get()),
			CurrencyId::Token2(0),
			Box::new(location.clone()),
			Weight::from_parts(2000_000_000, 0)
		));

		assert_noop!(
			AssetRegistry::register_multilocation(
				RuntimeOrigin::signed(CouncilAccount::get()),
				CurrencyId::Token2(0),
				Box::new(location.clone()),
				Weight::from_parts(2000_000_000, 0)
			),
			Error::<Runtime>::CurrencyIdExisted
		);

		assert_eq!(
			LocationToCurrencyIds::<Runtime>::get(multi_location),
			Some(CurrencyId::Token2(0))
		);
		assert_eq!(
			CurrencyIdToLocations::<Runtime>::get(CurrencyId::Token2(0)),
			Some(multi_location)
		);
		assert_eq!(
			CurrencyIdToWeights::<Runtime>::get(CurrencyId::Token2(0)),
			Some(Weight::from_parts(2000_000_000, 0))
		);
	})
}

#[test]
fn force_set_multilocation_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let metadata = AssetMetadata {
			name: b"Bifrost Native Coin".to_vec(),
			symbol: b"BNC".to_vec(),
			decimals: 12,
			minimal_balance: 0,
		};
		// V3
		let location = VersionedMultiLocation::V3(MultiLocation {
			parents: 1,
			interior: Junctions::X1(Junction::Parachain(2001)),
		});
		let multi_location: MultiLocation = location.clone().try_into().unwrap();

		assert_noop!(
			AssetRegistry::force_set_multilocation(
				RuntimeOrigin::signed(CouncilAccount::get()),
				CurrencyId::Token2(0),
				Box::new(location.clone()),
				Weight::from_parts(2000_000_000, 0)
			),
			Error::<Runtime>::CurrencyIdNotExists
		);

		assert_ok!(AssetRegistry::register_token_metadata(
			RuntimeOrigin::signed(CouncilAccount::get()),
			Box::new(metadata.clone())
		));

		assert_ok!(AssetRegistry::force_set_multilocation(
			RuntimeOrigin::signed(CouncilAccount::get()),
			CurrencyId::Token2(0),
			Box::new(location.clone()),
			Weight::from_parts(2000_000_000, 0)
		));

		assert_ok!(AssetRegistry::force_set_multilocation(
			RuntimeOrigin::signed(CouncilAccount::get()),
			CurrencyId::Token2(0),
			Box::new(location.clone()),
			Weight::from_parts(2000_000_000, 0)
		));

		assert_eq!(
			LocationToCurrencyIds::<Runtime>::get(multi_location),
			Some(CurrencyId::Token2(0))
		);
		assert_eq!(
			CurrencyIdToLocations::<Runtime>::get(CurrencyId::Token2(0)),
			Some(multi_location)
		);
		assert_eq!(
			CurrencyIdToWeights::<Runtime>::get(CurrencyId::Token2(0)),
			Some(Weight::from_parts(2000_000_000, 0))
		);
	})
}

#[test]
fn update_currency_metadata_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let caller = CouncilAccount::get();
		let currency_id = CurrencyId::Token2(0);
		let name = b"Updated Name".to_vec();
		let symbol = b"UN".to_vec();
		let decimals: u8 = 10;
		let minimal_balance = 1000u32.into();

		// Pre-insert a currency_metadata to update
		CurrencyMetadatas::<Runtime>::insert(
			currency_id,
			AssetMetadata {
				name: b"Old Name".to_vec(),
				symbol: b"ON".to_vec(),
				decimals: 8,
				minimal_balance: 1u32.into(),
			},
		);

		// Ensure the origin has the required permissions
		let origin = RuntimeOrigin::signed(caller);
		assert_ok!(AssetRegistry::update_currency_metadata(
			origin,
			currency_id,
			Some(name.clone()),
			Some(symbol.clone()),
			Some(decimals),
			Some(minimal_balance)
		));

		System::assert_last_event(RuntimeEvent::AssetRegistry(crate::Event::CurrencyIdUpdated {
			currency_id,
			metadata: AssetMetadata {
				name: name.clone(),
				symbol: symbol.clone(),
				decimals,
				minimal_balance,
			},
		}));

		// Verify the updated metadata
		let updated_metadata = CurrencyMetadatas::<Runtime>::get(currency_id).unwrap();
		assert_eq!(updated_metadata.name, name);
		assert_eq!(updated_metadata.symbol, symbol);
		assert_eq!(updated_metadata.decimals, decimals);
		assert_eq!(updated_metadata.minimal_balance, minimal_balance);
	})
}

#[test]
fn update_currency_metadata_should_work_no_change() {
	ExtBuilder::default().build().execute_with(|| {
		let caller = CouncilAccount::get();
		let currency_id = CurrencyId::Token2(0);
		let name = None;
		let symbol = None;
		let decimals = None;
		let minimal_balance = None;

		let old_metadata = AssetMetadata {
			name: b"Old Name".to_vec(),
			symbol: b"ON".to_vec(),
			decimals: 8,
			minimal_balance: 1u32.into(),
		};

		// Pre-insert a currency_metadata to update
		CurrencyMetadatas::<Runtime>::insert(currency_id, old_metadata.clone());

		// Ensure the origin has the required permissions
		let origin = RuntimeOrigin::signed(caller);
		assert_ok!(AssetRegistry::update_currency_metadata(
			origin,
			currency_id,
			name,
			symbol,
			decimals,
			minimal_balance
		));

		// Verify the event
		System::assert_last_event(RuntimeEvent::AssetRegistry(crate::Event::CurrencyIdUpdated {
			currency_id,
			metadata: old_metadata.clone(),
		}));

		// Verify the updated metadata
		let updated_metadata = CurrencyMetadatas::<Runtime>::get(currency_id).unwrap();
		assert_eq!(updated_metadata, old_metadata);
	});
}

#[test]
fn update_currency_metadata_nonexistent_currency_id() {
	ExtBuilder::default().build().execute_with(|| {
		let caller = CouncilAccount::get();
		let currency_id = CurrencyId::Token2(1); // Non-existent currency ID
		let name = Some(b"Updated Name".to_vec());
		let symbol = Some(b"UN".to_vec());
		let decimals = Some(10);
		let minimal_balance = Some(1000u32.into());

		// Ensure the origin has the required permissions
		let origin = RuntimeOrigin::signed(caller);
		assert_noop!(
			AssetRegistry::update_currency_metadata(
				origin,
				currency_id,
				name,
				symbol,
				decimals,
				minimal_balance
			),
			Error::<Runtime>::CurrencyIdNotExists
		);
	});
}
