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

use crate as stable_asset;
use bifrost_primitives::StableAssetPalletId;
use frame_support::{
	derive_impl,
	dispatch::DispatchResult,
	parameter_types,
	traits::{ConstU128, ConstU32, Currency, EnsureOrigin, Nothing, OnUnbalanced},
};
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use sp_runtime::{traits::IdentityLookup, BuildStorage, DispatchError};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		Tokens: orml_tokens,
		Currencies: bifrost_currencies,
		StableAsset: stable_asset,
	}
);

pub type AccountId = u64;

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type AccountData = pallet_balances::AccountData<Balance>;
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
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = ();
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
pub type AdaptedBasicCurrency =
	bifrost_currencies::BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;

impl bifrost_currencies::Config for Test {
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = AdaptedBasicCurrency;
	type WeightInfo = ();
}

pub type Balance = u128;
type AtLeast64BitUnsigned = u128;

pub type AssetId = i64;

use std::{cell::RefCell, collections::HashMap};

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
			let id =
				AssetId::try_from(d.len()).map_err(|_| DispatchError::Other("Too large id"))?;
			d.push(Asset { total: 0, balances: HashMap::new() });

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

	fn ensure_can_withdraw(
		_currency_id: Self::CurrencyId,
		_who: &AccountId,
		_amount: Self::Balance,
	) -> DispatchResult {
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
			let i =
				usize::try_from(asset).map_err(|_| DispatchError::Other("Index out of range"))?;
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

	fn withdraw(
		asset: Self::CurrencyId,
		dest: &AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		ASSETS.with(|d| -> DispatchResult {
			let i =
				usize::try_from(asset).map_err(|_| DispatchError::Other("Index out of range"))?;
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

	fn slash(
		_currency_id: Self::CurrencyId,
		_who: &AccountId,
		_amount: Self::Balance,
	) -> Self::Balance {
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
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
