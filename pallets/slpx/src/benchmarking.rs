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

use crate::*;
use bifrost_asset_registry::CurrencyIdToLocations;
use bifrost_primitives::{CurrencyId, KSM, VKSM};
use frame_benchmarking::v2::*;
use frame_support::{assert_ok, sp_runtime::traits::UniqueSaturatedFrom, BoundedVec};
use frame_system::RawOrigin;

fn init_whitelist<T: Config + bifrost_asset_registry::Config>() -> (T::AccountId, H160) {
	let caller: T::AccountId = whitelisted_caller();
	assert_ok!(Pallet::<T>::add_whitelist(
		RawOrigin::Root.into(),
		SupportChain::Astar,
		caller.clone()
	));
	let addr: [u8; 20] = hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();
	let receiver = H160::from(addr);
	let evm_caller_account_id = Pallet::<T>::h160_to_account_id(receiver);
	assert_ok!(<T as Config>::MultiCurrency::deposit(
		KSM,
		&evm_caller_account_id,
		BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128),
	));

	assert_ok!(<T as Config>::MultiCurrency::deposit(
		VKSM,
		&evm_caller_account_id,
		BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128),
	));

	CurrencyIdToLocations::<T>::insert(KSM, MultiLocation::default());
	CurrencyIdToLocations::<T>::insert(VKSM, MultiLocation::default());

	(caller, receiver)
}

#[benchmarks(where  T: Config + bifrost_asset_registry::Config + bifrost_stable_pool::Config + bifrost_stable_asset::Config + orml_tokens::Config<CurrencyId = CurrencyId>)]
mod benchmarks {
	use super::*;
	use frame_benchmarking::impl_benchmark_test_suite;

	#[benchmark]
	fn add_whitelist() {
		let contract: T::AccountId = whitelisted_caller();

		#[extrinsic_call]
		_(RawOrigin::Root, SupportChain::Astar, contract.clone());

		assert_eq!(WhitelistAccountId::<T>::get(SupportChain::Astar).first(), Some(&contract));
	}

	#[benchmark]
	fn remove_whitelist() {
		let contract: T::AccountId = whitelisted_caller();
		let whitelist = BoundedVec::try_from(vec![contract.clone()]).unwrap();

		WhitelistAccountId::<T>::insert(SupportChain::Astar, whitelist);

		#[extrinsic_call]
		_(RawOrigin::Root, SupportChain::Astar, contract.clone());

		assert_eq!(WhitelistAccountId::<T>::get(SupportChain::Astar).first(), None);
	}

	#[benchmark]
	fn set_execution_fee() {
		#[extrinsic_call]
		_(RawOrigin::Root, CurrencyId::Token2(0), 10u32.into());

		assert_eq!(ExecutionFee::<T>::get(CurrencyId::Token2(0)), Some(10u32.into()));
	}

	#[benchmark]
	fn set_transfer_to_fee() {
		#[extrinsic_call]
		_(RawOrigin::Root, SupportChain::Moonbeam, 10u32.into());

		assert_eq!(TransferToFee::<T>::get(SupportChain::Moonbeam), Some(10u32.into()));
	}

	#[benchmark]
	fn mint() {
		let (caller, receiver) = init_whitelist::<T>();

		#[extrinsic_call]
		_(
			RawOrigin::Signed(caller),
			receiver,
			KSM,
			TargetChain::Astar(receiver),
			BoundedVec::default(),
		);
	}

	#[benchmark]
	fn mint_with_channel_id() {
		let (caller, receiver) = init_whitelist::<T>();

		#[extrinsic_call]
		_(
			RawOrigin::Signed(caller),
			receiver,
			KSM,
			TargetChain::Astar(receiver),
			BoundedVec::default(),
			0u32,
		);
	}

	#[benchmark]
	fn redeem() {
		let (caller, receiver) = init_whitelist::<T>();
		#[extrinsic_call]
		_(RawOrigin::Signed(caller), receiver, VKSM, TargetChain::Astar(receiver));
	}

	//   `cargo test -p pallet-example-basic --all-features`, you will see one line per case:
	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
