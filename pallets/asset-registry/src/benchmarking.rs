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

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as AssetRegistry;
use bifrost_primitives::{CurrencyId, TokenSymbol};
use frame_benchmarking::{benchmarks, v1::BenchmarkError};
use frame_support::{assert_ok, traits::UnfilteredDispatchable};
use sp_runtime::traits::UniqueSaturatedFrom;
use xcm::v3::prelude::*;

benchmarks! {
	register_native_asset {
		let origin = T::RegisterOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let v3_location = VersionedMultiLocation::V3(MultiLocation::from(X1(Junction::Parachain(1000))));

	let call = Call::<T>::register_native_asset {
			currency_id: CurrencyId::Token(TokenSymbol::DOT),
			location: Box::new(v3_location.clone()),
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
		let origin = T::RegisterOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let v3_location = VersionedMultiLocation::V3(MultiLocation::from(X1(Junction::Parachain(1000))));

		assert_ok!(AssetRegistry::<T>::register_native_asset(
			origin.clone(),
			CurrencyId::Token(TokenSymbol::DOT),
			Box::new(v3_location.clone()),
			Box::new(AssetMetadata {
				name: b"Token Name".to_vec(),
				symbol: b"TN".to_vec(),
				decimals: 12,
				minimal_balance: BalanceOf::<T>::unique_saturated_from(1u128),
			})
		));

	let call = Call::<T>::update_native_asset {
			currency_id: CurrencyId::Token(TokenSymbol::DOT),
			location: Box::new(v3_location.clone()),
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
		let origin = T::RegisterOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
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
		let origin = T::RegisterOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
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
		let origin = T::RegisterOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
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
		let origin = T::RegisterOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
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
		let origin = T::RegisterOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let metadata = AssetMetadata {
			name: b"Bifrost Native Coin".to_vec(),
			symbol: b"BNC".to_vec(),
			decimals: 12,
			minimal_balance: BalanceOf::<T>::unique_saturated_from(0u128),
		};
		// v3
		let location = VersionedMultiLocation::V3(MultiLocation {
			parents: 1,
			interior: Junctions::X1(Parachain(2001)),
		});

		let multi_location: MultiLocation = location.clone().try_into().unwrap();

		assert_ok!(AssetRegistry::<T>::register_token_metadata(
			origin.clone(),
			Box::new(metadata.clone())
		));

		let call = Call::<T>::register_multilocation {
			currency_id: CurrencyId::Token2(0),
			location:Box::new(location.clone()),
			weight:Weight::from_parts(2000_000_000, u64::MAX),
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
		assert_eq!(CurrencyIdToWeights::<T>::get(CurrencyId::Token2(0)), Some(Weight::from_parts(2000_000_000, u64::MAX)));
	}

	force_set_multilocation {
		let origin = T::RegisterOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let metadata = AssetMetadata {
			name: b"Bifrost Native Coin".to_vec(),
			symbol: b"BNC".to_vec(),
			decimals: 12,
			minimal_balance: BalanceOf::<T>::unique_saturated_from(0u128),
		};
		// v3
		let location = VersionedMultiLocation::V3(MultiLocation {
			parents: 1,
			interior: Junctions::X1(Parachain(2001)),
		});

		let multi_location: MultiLocation = location.clone().try_into().unwrap();

		assert_ok!(AssetRegistry::<T>::register_token_metadata(
			origin.clone(),
			Box::new(metadata.clone())
		));

		let call = Call::<T>::force_set_multilocation {
			currency_id: CurrencyId::Token2(0),
			location:Box::new(location.clone()),
			weight:Weight::from_parts(2000_000_000, u64::MAX),
		};
	}: {call.dispatch_bypass_filter(origin)?}

	impl_benchmark_test_suite!(
	AssetRegistry,
	crate::mock::ExtBuilder::default().build(),
	crate::mock::Runtime
);

}
