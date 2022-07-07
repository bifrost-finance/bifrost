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
use crate::{Pallet as SystemStaking, *};
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, vec, whitelisted_caller};
use frame_support::{
	assert_ok,
	sp_runtime::{traits::UniqueSaturatedFrom, Perbill, Permill},
	traits::OnInitialize,
};
use frame_system::{Pallet as System, RawOrigin};
use node_primitives::{CurrencyId, PoolId, TokenSymbol};

benchmarks! {
	on_initialize {
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		const MOVR: CurrencyId = CurrencyId::Token(TokenSymbol::MOVR);
		assert_ok!(SystemStaking::<T>::token_config(
			RawOrigin::Root.into(),
			KSM,
			Some(1),
			Some(Permill::from_percent(80)),
			Some(false),
			Some(BalanceOf::<T>::unique_saturated_from(1000u128)),
			Some(vec![1 as PoolId]),
			Some(vec![Perbill::from_percent(100)]),
		));
		assert_ok!(SystemStaking::<T>::token_config(
			RawOrigin::Root.into(),
			MOVR,
			Some(2),
			Some(Permill::from_percent(80)),
			Some(false),
			Some(BalanceOf::<T>::unique_saturated_from(1000u128)),
			Some(vec![1 as PoolId]),
			Some(vec![Perbill::from_percent(100)]),
		));
		System::<T>::set_block_number(
			System::<T>::block_number() + 1u32.into()
		);
		SystemStaking::<T>::on_initialize(System::<T>::block_number());
		System::<T>::set_block_number(
			System::<T>::block_number() + 1u32.into()
		);
		SystemStaking::<T>::on_initialize(System::<T>::block_number());
	}:{SystemStaking::<T>::on_initialize(System::<T>::block_number());}

	token_config {
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		let pool_id = PoolId::from(1u32);
	}: _(RawOrigin::Root, KSM, Some(1), Some(Permill::from_percent(80)),Some(false),Some(token_amount),Some(vec![pool_id]),Some(vec![Perbill::from_percent(100)]))

	refresh_token_info {
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		assert_ok!(SystemStaking::<T>::token_config(
			RawOrigin::Root.into(),
			KSM,
			Some(1),
			Some(Permill::from_percent(80)),
			Some(false),
			Some(BalanceOf::<T>::unique_saturated_from(1000u128)),
			Some(vec![1 as PoolId]),
			Some(vec![Perbill::from_percent(100)]),
		));
	}: _(RawOrigin::Root,KSM)

	payout {
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		assert_ok!(SystemStaking::<T>::token_config(
			RawOrigin::Root.into(),
			KSM,
			Some(1),
			Some(Permill::from_percent(80)),
			Some(false),
			Some(BalanceOf::<T>::unique_saturated_from(1000u128)),
			Some(vec![1 as PoolId]),
			Some(vec![Perbill::from_percent(100)]),
		));
	}: _(RawOrigin::Root,KSM)

	on_redeem_success {
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		assert_ok!(SystemStaking::<T>::token_config(
			RawOrigin::Root.into(),
			KSM,
			Some(1),
			Some(Permill::from_percent(80)),
			Some(false),
			Some(BalanceOf::<T>::unique_saturated_from(1000u128)),
			Some(vec![1 as PoolId]),
			Some(vec![Perbill::from_percent(100)]),
		));
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
	}:{SystemStaking::<T>::on_redeem_success(KSM,caller,token_amount);}

	on_redeemed {
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		assert_ok!(SystemStaking::<T>::token_config(
			RawOrigin::Root.into(),
			KSM,
			Some(1),
			Some(Permill::from_percent(80)),
			Some(false),
			Some(BalanceOf::<T>::unique_saturated_from(1000u128)),
			Some(vec![1 as PoolId]),
			Some(vec![Perbill::from_percent(100)]),
		));
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		let fee_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
	}:{SystemStaking::<T>::on_redeemed(caller,KSM,token_amount,token_amount,fee_amount);}

	delete_token {
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		assert_ok!(SystemStaking::<T>::token_config(
			RawOrigin::Root.into(),
			KSM,
			Some(1),
			Some(Permill::from_percent(80)),
			Some(false),
			Some(BalanceOf::<T>::unique_saturated_from(1000u128)),
			Some(vec![1 as PoolId]),
			Some(vec![Perbill::from_percent(100)]),
		));
	}: _(RawOrigin::Root,KSM)
}

impl_benchmark_test_suite!(
	SystemStaking,
	crate::mock::ExtBuilder::default().one_hundred_for_alice_n_bob().build(),
	crate::mock::Runtime
);
