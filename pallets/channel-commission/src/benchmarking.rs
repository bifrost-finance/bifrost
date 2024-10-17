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

use bifrost_primitives::{CurrencyId, KSM, VKSM};
use frame_benchmarking::v1::{account, benchmarks, whitelisted_caller, BenchmarkError};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_runtime::traits::UniqueSaturatedFrom;

use super::*;
use crate::Pallet as ChannelCommission;

benchmarks! {
	register_channel {
		// assume we have 30 vtoken at most
		let x in 1 .. 30;

		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let channel_name =  b"Bifrost".to_vec();
		let receiver = whitelisted_caller();

		// set_commission_tokens
		for i in 0 .. x {
			let i: u8 = i.try_into().unwrap();
			let vtoken = CurrencyId::VToken2(i);
			let commission_token = CurrencyId::Token2(i);
			assert_ok!(ChannelCommission::<T>::set_commission_tokens(
				origin.clone(),
				vtoken, Some(commission_token)
			));
		}

	}: _<T::RuntimeOrigin>(origin.clone(), channel_name, receiver)

	remove_channel {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let channel_name = b"Bifrost".to_vec();
		let receiver = whitelisted_caller();
		let channel_id = 0;

		assert_ok!(ChannelCommission::<T>::register_channel(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			channel_name, receiver
		));
	}: _<T::RuntimeOrigin>(origin.clone(),channel_id)

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
	}: _<T::RuntimeOrigin>(origin.clone(),channel_id, new_receiver)

	set_channel_commission_token {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let channel_name = b"Bifrost".to_vec();
		let receiver = whitelisted_caller();
		let commission_rate = Percent::from_percent(0);
		let channel_id = 0;
		let vtoken = CurrencyId::VToken2(0);
		let commission_token = CurrencyId::Token2(0);
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		assert_ok!(ChannelCommission::<T>::set_commission_tokens(
			origin.clone(),
			vtoken, Some(commission_token)
		));

		assert_ok!(ChannelCommission::<T>::register_channel(
			origin.clone(),
			channel_name, receiver
		));
	}: _<T::RuntimeOrigin>(origin.clone(),channel_id, vtoken, commission_rate)

	set_commission_tokens {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let commission_token = CurrencyId::Token2(0);
		let vtoken = CurrencyId::VToken2(0);

	}: _<T::RuntimeOrigin>(origin.clone(), vtoken, Some(commission_token))

	claim_commissions {
		let test_account: T::AccountId = account("seed",1,1);
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let channel_name = b"Bifrost".to_vec();
		let receiver: T::AccountId = whitelisted_caller();
		let channel_id = 0;
		let vtoken = VKSM;
		let commission_token = KSM;
		let commission_account: T::AccountId = T::CommissionPalletId::get().into_account_truncating();

		assert_ok!(ChannelCommission::<T>::set_commission_tokens(
			origin.clone(),
			vtoken, Some(commission_token)
		));

		assert_ok!(ChannelCommission::<T>::register_channel(
			origin.clone(),
			channel_name, receiver.clone()
		));

		// set some amount into ChannelClaimableCommissions storage
		let amount = BalanceOf::<T>::unique_saturated_from(1000u32);
		ChannelClaimableCommissions::<T>::insert(channel_id, commission_token, amount);
		// deposit some amount into the commission pool
		T::MultiCurrency::deposit(commission_token, &commission_account, 4000000000u32.into())?;
		// deposit some amount into the receiver account to avoid existential deposit error
		T::MultiCurrency::deposit(commission_token, &receiver, 4000000000u32.into())?;
	}: _(RawOrigin::Signed(test_account), channel_id)

	on_initialize {
		// assume we have 30 vtoken at most
		let x in 1 .. 30;

		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let channel_name =  b"Bifrost".to_vec();
		let receiver: T::AccountId = whitelisted_caller();
		let share = Permill::from_percent(20);
		let commission_account: T::AccountId = T::CommissionPalletId::get().into_account_truncating();

		// token_id
		for i in 0 .. x {
			let i: u8 = i.try_into().unwrap();
			let vtoken = CurrencyId::VToken2(i);
			let commission_token = CurrencyId::Token2(i);

			// set_commission_tokens
			assert_ok!(ChannelCommission::<T>::set_commission_tokens(
				origin.clone(),
				vtoken, Some(commission_token)
			));

			let old_amount: BalanceOf<T> = 9000u128.unique_saturated_into();
			let new_amount: BalanceOf<T> = 10000u128.unique_saturated_into();
			VtokenIssuanceSnapshots::<T>::insert(vtoken, (old_amount, new_amount));

			let old_amount: BalanceOf<T> = 10000u128.unique_saturated_into();
			let new_amount: BalanceOf<T> = 2000u128.unique_saturated_into();
			PeriodVtokenTotalMint::<T>::insert(vtoken, (old_amount,new_amount));

			let old_amount: BalanceOf<T> = 0u128.unique_saturated_into();
			let new_amount: BalanceOf<T> = 1000u128.unique_saturated_into();
			PeriodVtokenTotalRedeem::<T>::insert(vtoken, (old_amount, new_amount));

			let old_amount: BalanceOf<T> = 0u128.unique_saturated_into();
			let new_amount: BalanceOf<T> = 100u128.unique_saturated_into();
			PeriodTotalCommissions::<T>::insert(vtoken, (old_amount, new_amount));

			// register_channel
			assert_ok!(ChannelCommission::<T>::register_channel(
				origin.clone(),
				channel_name.clone(), receiver.clone()
			));

			// set channel share
			let old_amount: BalanceOf<T> = 2000u128.unique_saturated_into();
			let new_amount: BalanceOf<T> = 500u128.unique_saturated_into();
			ChannelVtokenShares::<T>::insert(0, vtoken, share);
			PeriodChannelVtokenMint::<T>::insert(0, vtoken, (old_amount, new_amount));
		}

		let block_num =BlockNumberFor::<T>::from(101u32);
	}: {ChannelCommission::<T>::on_initialize(block_num);}

	set_channel_vtoken_shares {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let vtoken_set = CurrencyId::VToken2(0);
		let shares = Permill::from_percent(1);
		let channel_id = 0;

		// assume we have 60 channels at most
		let x in 1 .. 60;
		let channel_name =  b"Bifrost".to_vec();
		let receiver: T::AccountId = whitelisted_caller();

		// set_commission_tokens
		assert_ok!(ChannelCommission::<T>::set_commission_tokens(
			origin.clone(),
			vtoken_set, Some(CurrencyId::Token2(0))
		));

		// register channels
		for i in 0 .. x {
			assert_ok!(ChannelCommission::<T>::register_channel(origin.clone(), channel_name.clone(), receiver.clone()));
		}

	}: _<T::RuntimeOrigin>(origin.clone(), channel_id, vtoken_set, shares)

	impl_benchmark_test_suite!(ChannelCommission,crate::mock::ExtBuilder::default().build(),crate::mock::Runtime);
}
