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
use frame_support::{assert_noop, assert_ok};
use mock::{
	AssetRegistry, CouncilAccount, ExtBuilder, Runtime, RuntimeEvent, RuntimeOrigin, System,
};

#[test]
fn versioned_multi_location_convert_work() {
	ExtBuilder::default().build().execute_with(|| {
		let versioned_location = VersionedLocation::V4(Location::from([Parachain(1000)]));
		let location: Location = versioned_location.try_into().unwrap();
		assert_eq!(location, Location::new(0, [Parachain(1000)]));
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

		assert_eq!(CurrencyMetadatas::<Runtime>::get(Token2(0)), Some(metadata.clone()))
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
fn register_multilocation_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let metadata = AssetMetadata {
			name: b"Bifrost Native Coin".to_vec(),
			symbol: b"BNC".to_vec(),
			decimals: 12,
			minimal_balance: 0,
		};

		let versioned_location = VersionedLocation::V4(Location::new(1, [Parachain(2001)]));
		let location: Location = versioned_location.clone().try_into().unwrap();

		assert_noop!(
			AssetRegistry::register_location(
				RuntimeOrigin::signed(CouncilAccount::get()),
				Token2(0),
				Box::new(versioned_location.clone()),
				Weight::from_parts(2000_000_000, 0)
			),
			Error::<Runtime>::CurrencyIdNotExists
		);

		assert_ok!(AssetRegistry::register_token_metadata(
			RuntimeOrigin::signed(CouncilAccount::get()),
			Box::new(metadata.clone())
		));

		assert_ok!(AssetRegistry::register_location(
			RuntimeOrigin::signed(CouncilAccount::get()),
			Token2(0),
			Box::new(versioned_location.clone()),
			Weight::from_parts(2000_000_000, 0)
		));

		assert_noop!(
			AssetRegistry::register_location(
				RuntimeOrigin::signed(CouncilAccount::get()),
				Token2(0),
				Box::new(versioned_location.clone()),
				Weight::from_parts(2000_000_000, 0)
			),
			Error::<Runtime>::CurrencyIdExisted
		);

		assert_eq!(LocationToCurrencyIds::<Runtime>::get(location.clone()), Some(Token2(0)));
		assert_eq!(CurrencyIdToLocations::<Runtime>::get(Token2(0)), Some(location));
		assert_eq!(
			CurrencyIdToWeights::<Runtime>::get(Token2(0)),
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
		let versioned_location = VersionedLocation::V4(Location::new(1, [Parachain(2001)]));
		let location: Location = versioned_location.clone().try_into().unwrap();

		assert_noop!(
			AssetRegistry::force_set_location(
				RuntimeOrigin::signed(CouncilAccount::get()),
				Token2(0),
				Box::new(versioned_location.clone()),
				Weight::from_parts(2000_000_000, 0)
			),
			Error::<Runtime>::CurrencyIdNotExists
		);

		assert_ok!(AssetRegistry::register_token_metadata(
			RuntimeOrigin::signed(CouncilAccount::get()),
			Box::new(metadata.clone())
		));

		assert_ok!(AssetRegistry::force_set_location(
			RuntimeOrigin::signed(CouncilAccount::get()),
			Token2(0),
			Box::new(versioned_location.clone()),
			Weight::from_parts(2000_000_000, 0)
		));

		assert_ok!(AssetRegistry::force_set_location(
			RuntimeOrigin::signed(CouncilAccount::get()),
			Token2(0),
			Box::new(versioned_location.clone()),
			Weight::from_parts(2000_000_000, 0)
		));

		assert_eq!(LocationToCurrencyIds::<Runtime>::get(location.clone()), Some(Token2(0)));
		assert_eq!(CurrencyIdToLocations::<Runtime>::get(Token2(0)), Some(location));
		assert_eq!(
			CurrencyIdToWeights::<Runtime>::get(Token2(0)),
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
