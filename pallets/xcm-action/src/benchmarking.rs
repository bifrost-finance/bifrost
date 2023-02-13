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
#![allow(non_upper_case_globals)]
#![allow(unused_imports)]

use crate::{Pallet as XcmAction, *};
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::sp_runtime::traits::UniqueSaturatedFrom;
use frame_system::RawOrigin;
use node_primitives::{CurrencyId, TokenSymbol};
use xcm::{latest::prelude::*, VersionedMultiLocation};

benchmarks! {
	mint {
	let location = VersionedMultiLocation::V1(MultiLocation {
		parents: 1,
		interior: xcm::v1::Junctions::X1(xcm::v1::Junction::Parachain(2001)),
	});
	let multi_location: MultiLocation = location.clone().try_into().unwrap();
	let caller: T::AccountId = whitelisted_caller();
	let addr: [u8; 20] = hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();
	let receiver = H160::from(addr);
	// const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
	let token_amount = BalanceOf::<T>::unique_saturated_from(10u128);
	}: _(RawOrigin::Signed(caller.clone()), receiver, Box::new(multi_location), token_amount, 4_000_000_000u64)

  redeem {
	let location = VersionedMultiLocation::V1(MultiLocation {
		parents: 1,
		interior: xcm::v1::Junctions::X1(xcm::v1::Junction::Parachain(2001)),
	});
	let multi_location: MultiLocation = location.clone().try_into().unwrap();
	let caller: T::AccountId = whitelisted_caller();
	let addr: [u8; 20] = hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();
	let receiver = H160::from(addr);
	// const vKSM: CurrencyId = CurrencyId::VToken(TokenSymbol::KSM);
	let token_amount = BalanceOf::<T>::unique_saturated_from(10u128);
	}: _(RawOrigin::Signed(caller.clone()), receiver, Box::new(multi_location), token_amount)

  swap {
	let location1 = VersionedMultiLocation::V1(MultiLocation {
		parents: 1,
		interior: xcm::v1::Junctions::X1(xcm::v1::Junction::Parachain(2001)),
	});
	let multi_location1: MultiLocation = location1.clone().try_into().unwrap();
	let location2 = VersionedMultiLocation::V1(MultiLocation {
		parents: 1,
		interior: xcm::v1::Junctions::X1(xcm::v1::Junction::Parachain(2001)),
	});
	let multi_location2: MultiLocation = location2.clone().try_into().unwrap();
	let caller: T::AccountId = whitelisted_caller();
	let addr: [u8; 20] = hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();
	let receiver = H160::from(addr);
	// const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
	// const vKSM: CurrencyId = CurrencyId::VToken(TokenSymbol::KSM);
	let token_amount = BalanceOf::<T>::unique_saturated_from(10u128);
	}: _(RawOrigin::Signed(caller.clone()), receiver, token_amount, token_amount, Box::new(multi_location1), Box::new(multi_location2), 4_000_000_000u64)
}

impl_benchmark_test_suite!(
	XcmAction,
	crate::mock::ExtBuilder::default()
		.one_hundred_precision_for_each_currency_type_for_whitelist_account()
		.build(),
	crate::mock::Test
);
