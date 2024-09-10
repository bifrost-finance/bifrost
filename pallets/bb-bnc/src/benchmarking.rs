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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{account, benchmarks, v1::BenchmarkError};
use frame_support::{assert_ok, traits::EnsureOrigin};

use bifrost_primitives::{CurrencyId, TokenSymbol};
use frame_system::RawOrigin;
use sp_runtime::traits::UniqueSaturatedFrom;
use sp_std::vec;

use crate::{BalanceOf, Call, Config, Pallet as BbBNC, Pallet};
use orml_traits::MultiCurrency;

benchmarks! {
	set_config {
	}: _(RawOrigin::Root,
		Some((4 * 365 * 86400 / 12u32).into()),
			Some((7 * 86400 / 12u32).into()))

	create_lock {
		let test_account: T::AccountId = account("seed",1,1);
		assert_ok!(BbBNC::<T>::set_config(
			RawOrigin::Root.into(),
			Some((4 * 365 * 86400 / 12u32).into()),
			Some((7 * 86400 / 12u32).into())
		));
		T::MultiCurrency::deposit(CurrencyId::Native(TokenSymbol::BNC), &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
		T::MultiCurrency::deposit(CurrencyId::VToken(TokenSymbol::BNC), &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
		let rewards = vec![(CurrencyId::Native(TokenSymbol::BNC), BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128))];
		assert_ok!(BbBNC::<T>::notify_rewards(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			account("seed",1,1),
			Some((7 * 86400 / 12u32).into()),rewards
		));

	}: _(RawOrigin::Signed(test_account),BalanceOf::<T>::unique_saturated_from(50000000000u128),(365 * 86400 / 12u32).into())

	increase_amount {
		let test_account: T::AccountId = account("seed",1,1);
		assert_ok!(BbBNC::<T>::set_config(
			RawOrigin::Root.into(),
			Some((4 * 365 * 86400 / 12u32).into()),
			Some((7 * 86400 / 12u32).into())
		));
		T::MultiCurrency::deposit(CurrencyId::Native(TokenSymbol::BNC), &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
		T::MultiCurrency::deposit(CurrencyId::VToken(TokenSymbol::BNC), &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
		let rewards = vec![(CurrencyId::Native(TokenSymbol::BNC), BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128))];
		assert_ok!(BbBNC::<T>::notify_rewards(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			account("seed",1,1),
			Some((7 * 86400 / 12u32).into()),rewards
		));

		assert_ok!(BbBNC::<T>::create_lock(
			RawOrigin::Signed(test_account.clone()).into(),
			BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128),
			(365 * 86400 / 12u32).into()
		));

	}: _(RawOrigin::Signed(test_account),0,BalanceOf::<T>::unique_saturated_from(50000000000u128))

	increase_unlock_time {
		let test_account: T::AccountId = account("seed",1,1);
		assert_ok!(BbBNC::<T>::set_config(
			RawOrigin::Root.into(),
			Some((4 * 365 * 86400 / 12u32).into()),
			Some((7 * 86400 / 12u32).into())
		));
		T::MultiCurrency::deposit(CurrencyId::Native(TokenSymbol::BNC), &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
		T::MultiCurrency::deposit(CurrencyId::VToken(TokenSymbol::BNC), &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
		let rewards = vec![(CurrencyId::Native(TokenSymbol::BNC), BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128))];
		assert_ok!(BbBNC::<T>::notify_rewards(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			account("seed",1,1),
			Some((7 * 86400 / 12u32).into()),rewards
		));

		assert_ok!(BbBNC::<T>::create_lock(
			RawOrigin::Signed(test_account.clone()).into(),
			BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128),
			(365 * 86400 / 12u32).into()
		));

	}: _(RawOrigin::Signed(test_account),0,(7 * 86400 / 12u32 + 365 * 86400 / 12u32).into())

	withdraw {
		let test_account: T::AccountId = account("seed",1,1);
		assert_ok!(BbBNC::<T>::set_config(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			Some((4 * 365 * 86400 / 12u32).into()),
			Some((7 * 86400 / 12u32).into())
		));
		T::MultiCurrency::deposit(CurrencyId::Native(TokenSymbol::BNC), &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
		T::MultiCurrency::deposit(CurrencyId::VToken(TokenSymbol::BNC), &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
		let rewards = vec![(CurrencyId::Native(TokenSymbol::BNC), BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128))];
		assert_ok!(BbBNC::<T>::notify_rewards(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			account("seed",1,1),
			Some((7 * 86400 / 12u32).into()),rewards
		));

		assert_ok!(BbBNC::<T>::create_lock(
			RawOrigin::Signed(test_account.clone()).into(),
			BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128),
			(365 * 86400 / 12u32).into()
		));

		<frame_system::Pallet<T>>::set_block_number((2 * 365 * 86400 / 12u32).into());

	}: _(RawOrigin::Signed(test_account),0)

	get_rewards {
		let test_account: T::AccountId = account("seed",1,1);
		assert_ok!(BbBNC::<T>::set_config(
			RawOrigin::Root.into(),
			Some((4 * 365 * 86400 / 12u32).into()),
			Some((7 * 86400 / 12u32).into())
		));
		T::MultiCurrency::deposit(CurrencyId::Native(TokenSymbol::BNC), &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
		T::MultiCurrency::deposit(CurrencyId::VToken(TokenSymbol::BNC), &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
		let rewards = vec![(CurrencyId::Native(TokenSymbol::BNC), BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128))];
		assert_ok!(BbBNC::<T>::notify_rewards(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			account("seed",1,1),
			Some((7 * 86400 / 12u32).into()),rewards
		));

		assert_ok!(BbBNC::<T>::create_lock(
			RawOrigin::Signed(test_account.clone()).into(),
			BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128),
			(365 * 86400 / 12u32).into()
		));

		<frame_system::Pallet<T>>::set_block_number((2 * 365 * 86400 / 12u32).into());

	}: _(RawOrigin::Signed(test_account))

	notify_rewards {
		assert_ok!(BbBNC::<T>::set_config(
			RawOrigin::Root.into(),
			Some((4 * 365 * 86400 / 12u32).into()),
			Some((7 * 86400 / 12u32).into())
		));
		let rewards = vec![(CurrencyId::Native(TokenSymbol::BNC), BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128))];
		T::MultiCurrency::deposit(CurrencyId::Native(TokenSymbol::BNC), &account("seed",1,1), BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
	}: _(RawOrigin::Root,account("seed",1,1),Some((7 * 86400 / 12u32).into()),rewards)

	set_markup_coefficient {
		assert_ok!(BbBNC::<T>::set_config(
			RawOrigin::Root.into(),
			Some((4 * 365 * 86400 / 12u32).into()),
			Some((7 * 86400 / 12u32).into())
		));
		T::MultiCurrency::deposit(CurrencyId::Native(TokenSymbol::BNC), &account("seed",1,1), BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
	}: _(RawOrigin::Root, CurrencyId::VToken(TokenSymbol::BNC), 10_000.into(), 10_000_000_000_000.into())

	deposit_markup {
		let test_account: T::AccountId = account("seed",1,1);
		assert_ok!(BbBNC::<T>::set_config(
			RawOrigin::Root.into(),
			Some((4 * 365 * 86400 / 12u32).into()),
			Some((7 * 86400 / 12u32).into())
		));
		T::MultiCurrency::deposit(CurrencyId::Native(TokenSymbol::BNC), &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
		T::MultiCurrency::deposit(CurrencyId::VToken(TokenSymbol::BNC), &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
		let rewards = vec![(CurrencyId::Native(TokenSymbol::BNC), BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128))];
		assert_ok!(BbBNC::<T>::notify_rewards(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			account("seed",1,1),
			Some((7 * 86400 / 12u32).into()),rewards
		));

		assert_ok!(BbBNC::<T>::create_lock(
			RawOrigin::Signed(test_account.clone()).into(),
			BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128),
			(365 * 86400 / 12u32).into()
		));
		assert_ok!(BbBNC::<T>::set_markup_coefficient(
			RawOrigin::Root.into(),
			CurrencyId::VToken(TokenSymbol::BNC),
			1_000.into(),
			10_000_000_000_000.into()
		));
		<frame_system::Pallet<T>>::set_block_number((2 * 365 * 86400 / 12u32).into());

	}: _(RawOrigin::Signed(test_account), CurrencyId::VToken(TokenSymbol::BNC), BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128))

	withdraw_markup {
		let test_account: T::AccountId = account("seed",1,1);
		assert_ok!(BbBNC::<T>::set_config(
			RawOrigin::Root.into(),
			Some((4 * 365 * 86400 / 12u32).into()),
			Some((7 * 86400 / 12u32).into())
		));
		T::MultiCurrency::deposit(CurrencyId::Native(TokenSymbol::BNC), &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
		T::MultiCurrency::deposit(CurrencyId::VToken(TokenSymbol::BNC), &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
		let rewards = vec![(CurrencyId::Native(TokenSymbol::BNC), BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128))];
		assert_ok!(BbBNC::<T>::notify_rewards(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			account("seed",1,1),
			Some((7 * 86400 / 12u32).into()),rewards
		));

		assert_ok!(BbBNC::<T>::create_lock(
			RawOrigin::Signed(test_account.clone()).into(),
			BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128),
			(365 * 86400 / 12u32).into()
		));
		assert_ok!(BbBNC::<T>::set_markup_coefficient(
			RawOrigin::Root.into(),
			CurrencyId::VToken(TokenSymbol::BNC),
			1_000.into(),
			10_000_000_000_000.into()
		));
		assert_ok!(BbBNC::<T>::deposit_markup(RawOrigin::Signed(test_account.clone()).into(), CurrencyId::VToken(TokenSymbol::BNC), BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128)));
		<frame_system::Pallet<T>>::set_block_number((2 * 365 * 86400 / 12u32).into());

	}: _(RawOrigin::Signed(test_account), CurrencyId::VToken(TokenSymbol::BNC))

	redeem_unlock {
		let test_account: T::AccountId = account("seed",1,1);
		assert_ok!(BbBNC::<T>::set_config(
			RawOrigin::Root.into(),
			Some((4 * 365 * 86400 / 12u32).into()),
			Some((7 * 86400 / 12u32).into())
		));
		T::MultiCurrency::deposit(CurrencyId::Native(TokenSymbol::BNC), &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
		T::MultiCurrency::deposit(CurrencyId::VToken(TokenSymbol::BNC), &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
		let rewards = vec![(CurrencyId::Native(TokenSymbol::BNC), BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128))];
		assert_ok!(BbBNC::<T>::notify_rewards(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			account("seed",1,1),
			Some((7 * 86400 / 12u32).into()),rewards
		));

		assert_ok!(BbBNC::<T>::create_lock(
			RawOrigin::Signed(test_account.clone()).into(),
			BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128),
			(365 * 86400 / 12u32).into()
		));
		assert_ok!(BbBNC::<T>::set_markup_coefficient(
			RawOrigin::Root.into(),
			CurrencyId::VToken(TokenSymbol::BNC),
			1_000.into(),
			10_000_000_000_000.into()
		));
		assert_ok!(BbBNC::<T>::deposit_markup(RawOrigin::Signed(test_account.clone()).into(), CurrencyId::VToken(TokenSymbol::BNC), BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128)));
		<frame_system::Pallet<T>>::set_block_number((2 * 86400 / 12u32).into());

	}: _(RawOrigin::Signed(test_account), 0)

	refresh {
		let test_account: T::AccountId = account("seed",1,1);
		assert_ok!(BbBNC::<T>::set_config(
			RawOrigin::Root.into(),
			Some((4 * 365 * 86400 / 12u32).into()),
			Some((7 * 86400 / 12u32).into())
		));
		T::MultiCurrency::deposit(CurrencyId::Native(TokenSymbol::BNC), &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
		T::MultiCurrency::deposit(CurrencyId::VToken(TokenSymbol::BNC), &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
		let rewards = vec![(CurrencyId::Native(TokenSymbol::BNC), BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128))];
		assert_ok!(BbBNC::<T>::notify_rewards(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			account("seed",1,1),
			Some((7 * 86400 / 12u32).into()),rewards
		));

		assert_ok!(BbBNC::<T>::create_lock(
			RawOrigin::Signed(test_account.clone()).into(),
			BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128),
			(365 * 86400 / 12u32).into()
		));
		assert_ok!(BbBNC::<T>::set_markup_coefficient(
			RawOrigin::Root.into(),
			CurrencyId::VToken(TokenSymbol::BNC),
			1_000.into(),
			10_000_000_000_000.into()
		));
		assert_ok!(BbBNC::<T>::deposit_markup(RawOrigin::Signed(test_account.clone()).into(), CurrencyId::VToken(TokenSymbol::BNC), BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128)));
		<frame_system::Pallet<T>>::set_block_number((2 * 86400 / 12u32).into());

	}: _(RawOrigin::Signed(test_account), CurrencyId::VToken(TokenSymbol::BNC))

		impl_benchmark_test_suite!(BbBNC,crate::mock::ExtBuilder::default().build(),crate::mock::Runtime);
}
