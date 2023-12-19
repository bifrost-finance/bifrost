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

use bifrost_primitives::CurrencyId;
use frame_benchmarking::v1::{account, benchmarks, whitelisted_caller, BenchmarkError};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_runtime::traits::UniqueSaturatedFrom;

use super::*;
#[allow(unused_imports)]
use crate::Pallet as ChannelCommission;

benchmarks! {
	register_channel {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let channel_name =  b"Bifrost".to_vec();
		let receiver = whitelisted_caller();

	}: _<T::RuntimeOrigin>(origin, channel_name, receiver)

	remove_channel {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let channel_name = b"Bifrost".to_vec();
		let receiver = whitelisted_caller();
		let channel_id = 0;

		assert_ok!(ChannelCommission::<T>::register_channel(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			channel_name, receiver
		));
	}: _<T::RuntimeOrigin>(origin,channel_id)

	update_channel_receive_account {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let channel_name = b"Bifrost".to_vec();
		let receiver = whitelisted_caller();
		let new_receiver = account("new_receiver", 0, 0);
		let channel_id = 0;

		assert_ok!(ChannelCommission::<T>::register_channel(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			channel_name, receiver
		));
	}: _<T::RuntimeOrigin>(origin,channel_id, new_receiver)

	set_channel_commission_token {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let channel_name = b"Bifrost".to_vec();
		let receiver = whitelisted_caller();
		let commission_rate = 0;
		let channel_id = 0;
		let vtoken = CurrencyId::VToken2(0);

		assert_ok!(ChannelCommission::<T>::register_channel(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			channel_name, receiver
		));
	}: _<T::RuntimeOrigin>(origin,channel_id, vtoken, Some(commission_rate))

	set_commission_tokens {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let commission_token = CurrencyId::VToken2(0);
		let vtoken = CurrencyId::VToken2(0);

	}: _<T::RuntimeOrigin>(origin, vtoken, commission_token)

	claim_commissions {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let channel_name = b"Bifrost".to_vec();
		let receiver = whitelisted_caller();
		let channel_id = 0;
		let vtoken = CurrencyId::VToken2(0);
		let commission_rate = 0;
		let commission_token = CurrencyId::VToken2(0);
		let commission_account = T::CommissionPalletId::get().into_account_truncating();

		assert_ok!(ChannelCommission::<T>::set_commission_tokens(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			vtoken, commission_token
		));

		assert_ok!(ChannelCommission::<T>::register_channel(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			channel_name, receiver
		));

		// set some amount into ChannelClaimableCommissions storage
		let amount = BalanceOf::<T>::unique_saturated_from(1000u32);
		ChannelClaimableCommissions::<T>::insert(channel_id, commission_token, amount);
		// deposit some amount into the commission pool
		assert_ok!(T::MultiCurrency::deposit(commission_token, &commission_account, 10000u32.into()));

	}: _<T::RuntimeOrigin>(origin, channel_id)

	impl_benchmark_test_suite!(ChannelCommission,crate::mock::ExtBuilder::default().build(),crate::mock::Runtime);
}
