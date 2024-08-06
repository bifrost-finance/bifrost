// Copyright (C) 2020-2024  Intergalactic, Limited (GIB).
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as EVMAccounts;

use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
use sp_std::prelude::*;

benchmarks! {
	where_clause {
		where T::AccountId: AsRef<[u8; 32]> + frame_support::pallet_prelude::IsType<AccountId32>,
	}

	bind_evm_address {
		let user: T::AccountId = account("user", 0, 1);
		let evm_address = Pallet::<T>::evm_address(&user);
		assert!(!AccountExtension::<T>::contains_key(evm_address));

	}: _(RawOrigin::Signed(user.clone()))
	verify {
		assert!(AccountExtension::<T>::contains_key(evm_address));
	}

	add_contract_deployer {
		let user: T::AccountId = account("user", 0, 1);
		let evm_address = Pallet::<T>::evm_address(&user);
		assert!(!ContractDeployer::<T>::contains_key(evm_address));

	}: _(RawOrigin::Root, evm_address)
	verify {
		assert!(ContractDeployer::<T>::contains_key(evm_address));
	}

	remove_contract_deployer {
		let user: T::AccountId = account("user", 0, 1);
		let evm_address = Pallet::<T>::evm_address(&user);

		EVMAccounts::<T>::add_contract_deployer(RawOrigin::Root.into(), evm_address)?;

		assert!(ContractDeployer::<T>::contains_key(evm_address));

	}: _(RawOrigin::Root, evm_address)
	verify {
		assert!(!ContractDeployer::<T>::contains_key(evm_address));
	}

	renounce_contract_deployer {
		let user: T::AccountId = account("user", 0, 1);
		let evm_address = Pallet::<T>::evm_address(&user);

		EVMAccounts::<T>::add_contract_deployer(RawOrigin::Root.into(), evm_address)?;
		EVMAccounts::<T>::bind_evm_address(RawOrigin::Signed(user.clone()).into())?;

		assert!(ContractDeployer::<T>::contains_key(evm_address));

	}: _(RawOrigin::Signed(user))
	verify {
		assert!(!ContractDeployer::<T>::contains_key(evm_address));
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::ExtBuilder::default().build(), crate::mock::Test);
}
