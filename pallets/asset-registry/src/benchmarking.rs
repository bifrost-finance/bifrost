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

#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, dispatch::UnfilteredDispatchable};
use primitives::{CurrencyId, TokenSymbol};
use sp_runtime::traits::UniqueSaturatedFrom;

use super::*;
#[allow(unused_imports)]
use crate::Pallet as AssetRegistry;

benchmarks! {
	register_foreign_asset {
		let origin = T::RegisterOrigin::successful_origin();
		let v0_location = VersionedMultiLocation::V0(xcm::v0::MultiLocation::X1(
			xcm::v0::Junction::Parachain(1000),
		));

	let call = Call::<T>::register_foreign_asset {
			location: Box::new(v0_location.clone()),
			metadata: Box::new(AssetMetadata {
				name: b"Token Name".to_vec(),
				symbol: b"TN".to_vec(),
				decimals: 12,
				minimal_balance: BalanceOf::<T>::unique_saturated_from(1u128),
			})
		};
	}: {call.dispatch_bypass_filter(origin)?}
	verify {
		let location: MultiLocation = v0_location.try_into().unwrap();

		let currency_id = CurrencyId::ForeignAsset(0);
		assert_eq!(CurrencyIdToLocations::<T>::get(currency_id), Some(location.clone()));
		assert_eq!(
			AssetMetadatas::<T>::get(AssetIds::ForeignAssetId(0)),
			Some(AssetMetadata {
				name: b"Token Name".to_vec(),
				symbol: b"TN".to_vec(),
				decimals: 12,
				minimal_balance: BalanceOf::<T>::unique_saturated_from(1u128),
			})
		);
		assert_eq!(
			LocationToCurrencyIds::<T>::get(location),
			Some(CurrencyId::ForeignAsset(0))
		);
	}
	update_foreign_asset {
		let origin = T::RegisterOrigin::successful_origin();
		let v0_location = VersionedMultiLocation::V0(xcm::v0::MultiLocation::X1(
			xcm::v0::Junction::Parachain(1000),
		));

		assert_ok!(AssetRegistry::<T>::register_foreign_asset(
			origin.clone(),
			Box::new(v0_location.clone()),
			Box::new(AssetMetadata {
				name: b"Token Name".to_vec(),
				symbol: b"TN".to_vec(),
				decimals: 12,
				minimal_balance: BalanceOf::<T>::unique_saturated_from(1u128),
			})
		));

	let call = Call::<T>::update_foreign_asset {
			foreign_asset_id: 0,
			location: Box::new(v0_location.clone()),
			metadata: Box::new(AssetMetadata {
				name: b"Token Name".to_vec(),
				symbol: b"TN".to_vec(),
				decimals: 12,
				minimal_balance: BalanceOf::<T>::unique_saturated_from(1u128),
			})
		};
	}: {call.dispatch_bypass_filter(origin)?}

	register_native_asset {
		let origin = T::RegisterOrigin::successful_origin();
		let v0_location = VersionedMultiLocation::V0(xcm::v0::MultiLocation::X1(
			xcm::v0::Junction::Parachain(1000),
		));

	let call = Call::<T>::register_native_asset {
			currency_id: CurrencyId::Token(TokenSymbol::DOT),
			location: Box::new(v0_location.clone()),
			metadata: Box::new(AssetMetadata {
				name: b"Token Name".to_vec(),
				symbol: b"TN".to_vec(),
				decimals: 12,
				minimal_balance: BalanceOf::<T>::unique_saturated_from(1u128),
			})
		};
	}: {call.dispatch_bypass_filter(origin)?}
	verify {
		assert_eq!(
			AssetMetadatas::<T>::get(AssetIds::NativeAssetId(CurrencyId::Token(
				TokenSymbol::DOT
			))),
			Some(AssetMetadata {
				name: b"Token Name".to_vec(),
				symbol: b"TN".to_vec(),
				decimals: 12,
				minimal_balance: BalanceOf::<T>::unique_saturated_from(1u128),
			})
		);
	}

	update_native_asset {
		let origin = T::RegisterOrigin::successful_origin();
		let v0_location = VersionedMultiLocation::V0(xcm::v0::MultiLocation::X1(
			xcm::v0::Junction::Parachain(1000),
		));

		assert_ok!(AssetRegistry::<T>::register_native_asset(
			origin.clone(),
			CurrencyId::Token(TokenSymbol::DOT),
			Box::new(v0_location.clone()),
			Box::new(AssetMetadata {
				name: b"Token Name".to_vec(),
				symbol: b"TN".to_vec(),
				decimals: 12,
				minimal_balance: BalanceOf::<T>::unique_saturated_from(1u128),
			})
		));

	let call = Call::<T>::update_native_asset {
			currency_id: CurrencyId::Token(TokenSymbol::DOT),
			location: Box::new(v0_location.clone()),
			metadata: Box::new(AssetMetadata {
				name: b"Token Name".to_vec(),
				symbol: b"TN".to_vec(),
				decimals: 13,
				minimal_balance: BalanceOf::<T>::unique_saturated_from(2u128),
			})
		};
	}: {call.dispatch_bypass_filter(origin)?}
	verify {
		assert_eq!(
			AssetMetadatas::<T>::get(AssetIds::NativeAssetId(CurrencyId::Token(
				TokenSymbol::DOT
			))),
			Some(AssetMetadata {
				name: b"Token Name".to_vec(),
				symbol: b"TN".to_vec(),
				decimals: 13,
				minimal_balance: BalanceOf::<T>::unique_saturated_from(2u128),
			})
		);
	}

	register_token_metadata {
		let origin = T::RegisterOrigin::successful_origin();
		let metadata = AssetMetadata {
			name: b"Bifrost Native Coin".to_vec(),
			symbol: b"BNC".to_vec(),
			decimals: 12,
			minimal_balance: BalanceOf::<T>::unique_saturated_from(0u128),
		};

		let call = Call::<T>::register_token_metadata {
			metadata: Box::new(metadata.clone())
		};
	}: {call.dispatch_bypass_filter(origin)?}
	verify {
		assert_eq!(CurrencyMetadatas::<T>::get(CurrencyId::Token2(0)), Some(metadata.clone()))
	}

	register_vtoken_metadata {
		let origin = T::RegisterOrigin::successful_origin();
		let metadata = AssetMetadata {
			name: b"Bifrost Native Coin".to_vec(),
			symbol: b"BNC".to_vec(),
			decimals: 12,
			minimal_balance: BalanceOf::<T>::unique_saturated_from(0u128),
		};
		let v_metadata = AssetMetadata {
			name: b"Voucher BNC".to_vec(),
			symbol: b"vBNC".to_vec(),
			decimals: 12,
			minimal_balance: BalanceOf::<T>::unique_saturated_from(0u128),
		};
		assert_ok!(AssetRegistry::<T>::register_token_metadata(
			origin.clone(),
			Box::new(metadata.clone())
		));

		let call = Call::<T>::register_vtoken_metadata {
			token_id: 0
		};
	}: {call.dispatch_bypass_filter(origin)?}
	verify {
		assert_eq!(
			CurrencyMetadatas::<T>::get(CurrencyId::VToken2(0)),
			Some(v_metadata.clone())
		)
	}

	register_vstoken_metadata {
		let origin = T::RegisterOrigin::successful_origin();
		let metadata = AssetMetadata {
			name: b"KSM Native Token".to_vec(),
			symbol: b"KSM".to_vec(),
			decimals: 12,
			minimal_balance: BalanceOf::<T>::unique_saturated_from(0u128),
		};
		let v_metadata = AssetMetadata {
			name: b"Voucher Slot KSM".to_vec(),
			symbol: b"vsKSM".to_vec(),
			decimals: 12,
			minimal_balance: BalanceOf::<T>::unique_saturated_from(0u128),
		};
		assert_ok!(AssetRegistry::<T>::register_token_metadata(
			origin.clone(),
			Box::new(metadata.clone())
		));

		let call = Call::<T>::register_vstoken_metadata {
			token_id: 0
		};
	}: {call.dispatch_bypass_filter(origin)?}
	verify {
		assert_eq!(
			CurrencyMetadatas::<T>::get(CurrencyId::VSToken2(0)),
			Some(v_metadata.clone())
		)
	}

	register_vsbond_metadata {
		let origin = T::RegisterOrigin::successful_origin();
				let metadata = AssetMetadata {
			name: b"KSM Native Token".to_vec(),
			symbol: b"KSM".to_vec(),
			decimals: 12,
			minimal_balance: BalanceOf::<T>::unique_saturated_from(0u128),
		};
		let name = "vsBOND-KSM-2001-10-20".as_bytes().to_vec();
		let v_metadata = AssetMetadata {
			name: name.clone(),
			symbol: name,
			decimals: 12,
			minimal_balance: BalanceOf::<T>::unique_saturated_from(0u128),
		};
		assert_ok!(AssetRegistry::<T>::register_token_metadata(
			origin.clone(),
			Box::new(metadata.clone())
		));

		let call = Call::<T>::register_vsbond_metadata {
			token_id: 0,
			para_id:2001,
			first_slot:10,
			last_slot:20
		};
	}: {call.dispatch_bypass_filter(origin)?}
	verify {
		assert_eq!(
			CurrencyMetadatas::<T>::get(CurrencyId::VSBond2(0, 2001, 10, 20)),
			Some(v_metadata.clone())
		)
	}

	register_multilocation {
		let origin = T::RegisterOrigin::successful_origin();
		let metadata = AssetMetadata {
			name: b"Bifrost Native Coin".to_vec(),
			symbol: b"BNC".to_vec(),
			decimals: 12,
			minimal_balance: BalanceOf::<T>::unique_saturated_from(0u128),
		};
		// v1
		let location = VersionedMultiLocation::V1(MultiLocation {
			parents: 1,
			interior: xcm::v1::Junctions::X1(xcm::v1::Junction::Parachain(2001)),
		});
		let multi_location: MultiLocation = location.clone().try_into().unwrap();

		assert_ok!(AssetRegistry::<T>::register_token_metadata(
			origin.clone(),
			Box::new(metadata.clone())
		));

		let call = Call::<T>::register_multilocation {
			currency_id: CurrencyId::Token2(0),
			location:Box::new(location.clone()),
			weight:2000_000_000,
		};
	}: {call.dispatch_bypass_filter(origin)?}
	verify {
		assert_eq!(
			LocationToCurrencyIds::<T>::get(multi_location.clone()),
			Some(CurrencyId::Token2(0))
		);
		assert_eq!(
			CurrencyIdToLocations::<T>::get(CurrencyId::Token2(0)),
			Some(multi_location.clone())
		);
		assert_eq!(CurrencyIdToWeights::<T>::get(CurrencyId::Token2(0)), Some(2000_000_000));
	}
}

impl_benchmark_test_suite!(
	AssetRegistry,
	crate::mock::ExtBuilder::default().build(),
	crate::mock::Runtime
);
