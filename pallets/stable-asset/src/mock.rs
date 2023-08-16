// This file is part of NUTS Finance.

// Copyright (C) 2017-2021 Parity Technologies (UK) Ltd.
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

use crate as stable_asset;
use frame_support::{
	dispatch::{DispatchError, DispatchResult},
	parameter_types,
	traits::{
		ConstU128, ConstU16, ConstU32, ConstU64, Currency, EnsureOrigin, Everything, GenesisBuild, Nothing,
		OnUnbalanced,
	},
	PalletId,
};
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		Tokens: orml_tokens::{Pallet, Call, Storage, Config<T>, Event<T>},
		Currencies: orml_currencies::{Pallet, Call},
		StableAsset: stable_asset::{Pallet, Call, Storage, Event<T>},
	}
);

pub type AccountId = u64;

impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Test {
	type MaxLocks = ();
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type HoldIdentifier = ();
	type FreezeIdentifier = ();
	type MaxHolds = ();
	type MaxFreezes = ();
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: i64| -> Balance {
		0
	};
}
impl orml_tokens::Config for Test {
	type Amount = i128;
	type Balance = Balance;
	type CurrencyId = i64;
	type DustRemovalWhitelist = Nothing;
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
	type CurrencyHooks = ();
}

parameter_types! {
	pub const GetNativeCurrencyId: i64 = 0;
}

pub type BlockNumber = u64;
pub type Amount = i128;
pub type AdaptedBasicCurrency = orml_currencies::BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;

impl orml_currencies::Config for Test {
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = AdaptedBasicCurrency;
	type WeightInfo = ();
}

pub type Balance = u128;
type AtLeast64BitUnsigned = u128;

pub type AssetId = i64;

use std::cell::RefCell;
use std::collections::HashMap;

pub struct Asset {
	total: Balance,
	balances: HashMap<AccountId, Balance>,
}

thread_local! {
	static ASSETS: RefCell<Vec<Asset>> = RefCell::new(Vec::new());
}

pub trait CreateAssets<AssetId> {
	fn create_asset() -> Result<AssetId, DispatchError>;
}

pub struct TestAssets;
impl CreateAssets<AssetId> for TestAssets {
	fn create_asset() -> Result<AssetId, DispatchError> {
		ASSETS.with(|d| -> Result<AssetId, DispatchError> {
			let mut d = d.borrow_mut();
			let id = AssetId::try_from(d.len()).map_err(|_| DispatchError::Other("Too large id"))?;
			d.push(Asset {
				total: 0,
				balances: HashMap::new(),
			});

			Ok(id)
		})
	}
}

impl MultiCurrency<AccountId> for TestAssets {
	type CurrencyId = AssetId;
	type Balance = Balance;

	fn minimum_balance(_currency_id: Self::CurrencyId) -> Self::Balance {
		todo!()
	}

	fn total_issuance(_currency_id: Self::CurrencyId) -> Self::Balance {
		todo!()
	}

	fn total_balance(_currency_id: Self::CurrencyId, _who: &AccountId) -> Self::Balance {
		todo!()
	}

	fn free_balance(asset: Self::CurrencyId, who: &AccountId) -> Self::Balance {
		ASSETS
			.with(|d| -> Option<Balance> {
				let i = usize::try_from(asset).ok()?;
				let d = d.borrow();
				let a = d.get(i)?;
				a.balances.get(who).copied()
			})
			.map(|x| x - 1)
			.unwrap_or(0)
	}

	fn ensure_can_withdraw(_currency_id: Self::CurrencyId, _who: &AccountId, _amount: Self::Balance) -> DispatchResult {
		todo!()
	}

	fn transfer(
		currency_id: Self::CurrencyId,
		from: &AccountId,
		to: &AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		Self::deposit(currency_id, to, amount)?;
		Self::withdraw(currency_id, from, amount)?;
		Ok(())
	}

	fn deposit(asset: Self::CurrencyId, dest: &AccountId, amount: Self::Balance) -> DispatchResult {
		ASSETS.with(|d| -> DispatchResult {
			let i = usize::try_from(asset).map_err(|_| DispatchError::Other("Index out of range"))?;
			let mut d = d.borrow_mut();
			let a = d.get_mut(i).ok_or(DispatchError::Other("Index out of range"))?;

			if let Some(x) = a.balances.get_mut(dest) {
				*x = x.checked_add(amount).ok_or(DispatchError::Other("Overflow"))?;
			} else {
				a.balances.insert(*dest, amount);
			}

			a.total = a.total.checked_add(amount).ok_or(DispatchError::Other("Overflow"))?;

			Ok(())
		})
	}

	fn withdraw(asset: Self::CurrencyId, dest: &AccountId, amount: Self::Balance) -> DispatchResult {
		ASSETS.with(|d| -> DispatchResult {
			let i = usize::try_from(asset).map_err(|_| DispatchError::Other("Index out of range"))?;
			let mut d = d.borrow_mut();
			let a = d.get_mut(i).ok_or(DispatchError::Other("Index out of range"))?;

			let x = a.balances.get_mut(dest).ok_or(DispatchError::Other("Not found"))?;

			*x = x.checked_sub(amount).ok_or(DispatchError::Other("Overflow"))?;

			a.total = a.total.checked_sub(amount).ok_or(DispatchError::Other("Overflow"))?;

			Ok(())
		})
	}

	fn can_slash(_currency_id: Self::CurrencyId, _who: &AccountId, _amount: Self::Balance) -> bool {
		todo!()
	}

	fn slash(_currency_id: Self::CurrencyId, _who: &AccountId, _amount: Self::Balance) -> Self::Balance {
		todo!()
	}
}

pub struct EmptyUnbalanceHandler;

type Imbalance = <pallet_balances::Pallet<Test> as Currency<AccountId>>::NegativeImbalance;

impl OnUnbalanced<Imbalance> for EmptyUnbalanceHandler {}

pub struct EnsureStableAsset;
impl EnsureOrigin<RuntimeOrigin> for EnsureStableAsset {
	type Success = AccountId;
	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		let result: Result<RawOrigin<AccountId>, RuntimeOrigin> = o.into();

		result.and_then(|o| match o {
			RawOrigin::Signed(id) => Ok(id),
			r => Err(RuntimeOrigin::from(r)),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::from(RawOrigin::Signed(Default::default())))
	}
}

pub struct EnsurePoolAssetId;
impl crate::traits::ValidateAssetId<i64> for EnsurePoolAssetId {
	fn validate(_: i64) -> bool {
		true
	}
}

parameter_types! {
	pub const StableAssetPalletId: PalletId = PalletId(*b"nuts/sta");
}

impl stable_asset::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AssetId = i64;
	type Balance = Balance;
	type Assets = TestAssets;
	type PalletId = StableAssetPalletId;

	type AtLeast64BitUnsigned = AtLeast64BitUnsigned;
	type FeePrecision = ConstU128<10_000_000_000>;
	type APrecision = ConstU128<100>;
	type PoolAssetLimit = ConstU32<5>;
	type SwapExactOverAmount = ConstU128<100>;
	type WeightInfo = ();
	type ListingOrigin = EnsureStableAsset;
	type EnsurePoolAssetId = EnsurePoolAssetId;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap()
		.into()
}
