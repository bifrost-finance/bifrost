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

use crate::{Pallet as StablePool, *};
use frame_benchmarking::{account, benchmarks, vec, whitelisted_caller};
use frame_support::{assert_ok, sp_runtime::traits::UniqueSaturatedFrom};
use frame_system::{Pallet as System, RawOrigin};
pub use node_primitives::{
	AccountId, Balance, CurrencyId, CurrencyIdMapping, SlpOperator, SlpxOperator, TokenSymbol, BNC,
	DOT, DOT_TOKEN_ID, GLMR, KSM, VDOT,
};

benchmarks! {
	create_pool {
		let fee_account: T::AccountId = whitelisted_caller();
		let coin0 = DOT;
		let coin1 = VDOT;
		let pool_asset = BNC;
	}: _(RawOrigin::Root,
		pool_asset.into(),
		vec![coin0.into(), coin1.into()],
		vec![1u128.into(), 1u128.into()],
		10000000u128.into(),
		20000000u128.into(),
		50000000u128.into(),
		10000u128.into(),
		fee_account.clone(),
		fee_account,
		1000000000000000000u128.into())

	edit_token_rate {}: _(RawOrigin::Root, 0, vec![(VDOT.into(), (9u128.into(), 10u128.into()))])

	add_liquidity {
		let test_account: T::AccountId = whitelisted_caller();
		let fee_account: T::AccountId = account("seed",1,1);
		let coin0 = BNC;
		let coin1 = KSM;
		let pool_asset = BNC;
		T::MultiCurrency::mint_into(
			BNC.into(),
			&fee_account,
			<T as nutsfinance_stable_asset::Config>::Balance::from(100_000_000_000u128.into())
		)?;
		T::MultiCurrency::mint_into(
			KSM.into(),
			&fee_account,
			<T as nutsfinance_stable_asset::Config>::Balance::from(100_000_000_000u128.into())
		)?;
		// T::MultiCurrency::mint_into(
		// 	VDOT.into(),
		// 	&fee_account,
		// 	<T as nutsfinance_stable_asset::Config>::Balance::from(100_000_000_000u128.into())
		// )?;
		let amounts = vec![<T as nutsfinance_stable_asset::Config>::Balance::from(10_000_000_000u128.into()), <T as nutsfinance_stable_asset::Config>::Balance::from(10_000_000_000u128.into())];
		assert_ok!(StablePool::<T>::create_pool(
		RawOrigin::Root.into(),
		pool_asset.into(),
		vec![coin0.into(), coin1.into()],
		vec![1u128.into(), 1u128.into()],
		10000000u128.into(),
		20000000u128.into(),
		50000000u128.into(),
		10000u128.into(),
		fee_account.clone(),
		fee_account.clone(),
		1000000000000000000u128.into()));
		assert_ok!(StablePool::<T>::edit_token_rate(RawOrigin::Root.into(), 0, vec![(BNC.into(), (9u128.into(), 10u128.into()))]));
		}: _(RawOrigin::Signed(fee_account), 0, amounts, <T as nutsfinance_stable_asset::Config>::Balance::zero())

	swap {
		let test_account: T::AccountId = whitelisted_caller();
		let fee_account: T::AccountId = account("seed",1,1);
		let coin0 = BNC;
		let coin1 = BNC;
		let pool_asset = BNC;
		T::MultiCurrency::mint_into(
			BNC.into(),
			&fee_account,
			<T as nutsfinance_stable_asset::Config>::Balance::from(1000_000_000_000u128.into())
		)?;
		let amounts = vec![<T as nutsfinance_stable_asset::Config>::Balance::from(100_000_000_000u128.into()), <T as nutsfinance_stable_asset::Config>::Balance::from(100_000_000u128.into())];
		assert_ok!(StablePool::<T>::create_pool(
		RawOrigin::Root.into(),
		pool_asset.into(),
		vec![coin0.into(), coin1.into()],
		vec![1u128.into(), 1u128.into()],
		10000000u128.into(),
		20000000u128.into(),
		50000000u128.into(),
		10000u128.into(),
		fee_account.clone(),
		fee_account.clone(),
		1000000000000000000u128.into()));
		assert_ok!(StablePool::<T>::edit_token_rate(RawOrigin::Root.into(), 0, vec![(BNC.into(), (9u128.into(), 10u128.into()))]));
		assert_ok!(StablePool::<T>::add_liquidity(RawOrigin::Signed(fee_account.clone()).into(), 0, amounts, <T as nutsfinance_stable_asset::Config>::Balance::zero()));
		// assert_ok!(StableAsset::swap(RawOrigin::signed(fee_account), 0, 0, 1, <T as nutsfinance_stable_asset::Config>::Balance::from(5000000u128.into()), 0, 2));
	}: _(RawOrigin::Signed(fee_account), 0, 0, 1, <T as nutsfinance_stable_asset::Config>::Balance::from(50_000_000_000u128.into()), <T as nutsfinance_stable_asset::Config>::Balance::zero())

	redeem_proportion {
		let test_account: T::AccountId = whitelisted_caller();
		let fee_account: T::AccountId = account("seed",1,1);
		let coin0 = BNC;
		let coin1 = BNC;
		let pool_asset = BNC;
		T::MultiCurrency::mint_into(
			BNC.into(),
			&fee_account,
			<T as nutsfinance_stable_asset::Config>::Balance::from(1000_000_000_000u128.into())
		)?;
		let amounts = vec![<T as nutsfinance_stable_asset::Config>::Balance::from(100_000_000_000u128.into()), <T as nutsfinance_stable_asset::Config>::Balance::from(100_000_000u128.into())];
		assert_ok!(StablePool::<T>::create_pool(
		RawOrigin::Root.into(),
		pool_asset.into(),
		vec![coin0.into(), coin1.into()],
		vec![1u128.into(), 1u128.into()],
		0u128.into(),
		0u128.into(),
		0u128.into(),
		220u128.into(),
		fee_account.clone(),
		fee_account.clone(),
		1000000000000u128.into()));
		assert_ok!(StablePool::<T>::edit_token_rate(RawOrigin::Root.into(), 0, vec![(BNC.into(), (9u128.into(), 10u128.into()))]));
		assert_ok!(StablePool::<T>::add_liquidity(RawOrigin::Signed(fee_account.clone()).into(), 0, amounts, <T as nutsfinance_stable_asset::Config>::Balance::zero()));
		// assert_ok!(StablePool::<T>::swap(RawOrigin::Signed(fee_account.clone()).into(), 0, 0, 1, <T as nutsfinance_stable_asset::Config>::Balance::from(50_000_000_000u128.into()), <T as nutsfinance_stable_asset::Config>::Balance::zero()));
	}: _(RawOrigin::Signed(fee_account), 0, <T as nutsfinance_stable_asset::Config>::Balance::from(5_000_000u128.into()), vec![<T as nutsfinance_stable_asset::Config>::Balance::zero(), <T as nutsfinance_stable_asset::Config>::Balance::zero()])

	redeem_single {
		let test_account: T::AccountId = whitelisted_caller();
		let fee_account: T::AccountId = account("seed",1,1);
		let coin0 = BNC;
		let coin1 = BNC;
		let pool_asset = BNC;
		T::MultiCurrency::mint_into(
			BNC.into(),
			&fee_account,
			<T as nutsfinance_stable_asset::Config>::Balance::from(1000_000_000_000u128.into())
		)?;
		let amounts = vec![<T as nutsfinance_stable_asset::Config>::Balance::from(100_000_000_000u128.into()), <T as nutsfinance_stable_asset::Config>::Balance::from(100_000_000u128.into())];
		assert_ok!(StablePool::<T>::create_pool(
		RawOrigin::Root.into(),
		pool_asset.into(),
		vec![coin0.into(), coin1.into()],
		vec![1u128.into(), 1u128.into()],
		0u128.into(),
		0u128.into(),
		0u128.into(),
		220u128.into(),
		fee_account.clone(),
		fee_account.clone(),
		1000000000000u128.into()));
		assert_ok!(StablePool::<T>::edit_token_rate(RawOrigin::Root.into(), 0, vec![(BNC.into(), (9u128.into(), 10u128.into()))]));
		assert_ok!(StablePool::<T>::add_liquidity(RawOrigin::Signed(fee_account.clone()).into(), 0, amounts, <T as nutsfinance_stable_asset::Config>::Balance::zero()));
	}: _(RawOrigin::Signed(fee_account), 0, <T as nutsfinance_stable_asset::Config>::Balance::from(5_000_000u128.into()), 0, <T as nutsfinance_stable_asset::Config>::Balance::zero(), 2)

	redeem_multi {
		let test_account: T::AccountId = whitelisted_caller();
		let fee_account: T::AccountId = account("seed",1,1);
		let coin0 = BNC;
		let coin1 = BNC;
		let pool_asset = BNC;
		T::MultiCurrency::mint_into(
			BNC.into(),
			&fee_account,
			<T as nutsfinance_stable_asset::Config>::Balance::from(1000_000_000_000u128.into())
		)?;
		let amounts = vec![<T as nutsfinance_stable_asset::Config>::Balance::from(100_000_000_000u128.into()), <T as nutsfinance_stable_asset::Config>::Balance::from(100_000_000_000u128.into())];
		assert_ok!(StablePool::<T>::create_pool(
		RawOrigin::Root.into(),
		pool_asset.into(),
		vec![coin0.into(), coin1.into()],
		vec![1u128.into(), 1u128.into()],
		0u128.into(),
		0u128.into(),
		0u128.into(),
		220u128.into(),
		fee_account.clone(),
		fee_account.clone(),
		1000000000000u128.into()));
		assert_ok!(StablePool::<T>::edit_token_rate(RawOrigin::Root.into(), 0, vec![(BNC.into(), (9u128.into(), 10u128.into()))]));
		assert_ok!(StablePool::<T>::add_liquidity(RawOrigin::Signed(fee_account.clone()).into(), 0, amounts, <T as nutsfinance_stable_asset::Config>::Balance::zero()));
		let redeem_amounts = vec![<T as nutsfinance_stable_asset::Config>::Balance::from(90_000_000u128.into()), <T as nutsfinance_stable_asset::Config>::Balance::from(90_000_000u128.into())];
	}: _(RawOrigin::Signed(fee_account), 0, redeem_amounts, <T as nutsfinance_stable_asset::Config>::Balance::from(200_000_000_000u128.into()))

	modify_a {
		let test_account: T::AccountId = whitelisted_caller();
		let fee_account: T::AccountId = account("seed",1,1);
		let coin0 = BNC;
		let coin1 = BNC;
		let pool_asset = BNC;
		assert_ok!(StablePool::<T>::create_pool(
			RawOrigin::Root.into(),
			pool_asset.into(),
			vec![coin0.into(), coin1.into()],
			vec![1u128.into(), 1u128.into()],
			0u128.into(),
			0u128.into(),
			0u128.into(),
			220u128.into(),
			fee_account.clone(),
			fee_account.clone(),
			1000000000000u128.into()));
	}: _(RawOrigin::Root, 0, 9u128.into(), 9u32.into())

	modify_fees {
		let test_account: T::AccountId = whitelisted_caller();
		let fee_account: T::AccountId = account("seed",1,1);
		let coin0 = BNC;
		let coin1 = BNC;
		let pool_asset = BNC;
		assert_ok!(StablePool::<T>::create_pool(
			RawOrigin::Root.into(),
			pool_asset.into(),
			vec![coin0.into(), coin1.into()],
			vec![1u128.into(), 1u128.into()],
			0u128.into(),
			0u128.into(),
			0u128.into(),
			220u128.into(),
			fee_account.clone(),
			fee_account.clone(),
			1000000000000u128.into()));
	}: _(RawOrigin::Root, 0, Some(1000u128.into()), Some(1000u128.into()), Some(1000u128.into()))

	modify_recipients {
		let test_account: T::AccountId = whitelisted_caller();
		let fee_account: T::AccountId = account("seed",1,1);
		let coin0 = BNC;
		let coin1 = BNC;
		let pool_asset = BNC;
		assert_ok!(StablePool::<T>::create_pool(
			RawOrigin::Root.into(),
			pool_asset.into(),
			vec![coin0.into(), coin1.into()],
			vec![1u128.into(), 1u128.into()],
			0u128.into(),
			0u128.into(),
			0u128.into(),
			220u128.into(),
			fee_account.clone(),
			fee_account.clone(),
			1000000000000u128.into()));
	}: _(RawOrigin::Root, 0, Some(test_account.clone()), Some(test_account))

	impl_benchmark_test_suite!(StablePool, crate::mock::ExtBuilder::build(), crate::mock::Test);
}
