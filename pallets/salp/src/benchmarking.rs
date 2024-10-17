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
#[cfg(feature = "runtime-benchmarks")]
use crate::{Pallet as Salp, *};
use bifrost_primitives::{CurrencyId, ParaId, XcmOperationType, KSM, VSKSM};
use bifrost_stable_pool::AtLeast64BitUnsignedOf;
use bifrost_xcm_interface::XcmWeightAndFee;
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_runtime::{
	traits::{AccountIdConversion, Bounded, UniqueSaturatedFrom},
	SaturatedConversion,
};

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::RuntimeEvent = generic_event.into();
	// compare to the last event record
	let frame_system::EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

fn create_fund<T: Config>(id: u32) -> ParaId {
	let cap = BalanceOf::<T>::max_value();
	let first_period = 0u32.into();
	let last_period = 7u32.into();
	let para_id = id;

	assert_ok!(Salp::<T>::create(RawOrigin::Root.into(), para_id, cap, first_period, last_period));

	para_id
}

fn contribute_fund<T: Config + bifrost_xcm_interface::Config>(
	index: ParaId,
) -> (T::AccountId, BalanceOf<T>)
where
	<<T as bifrost_xcm_interface::Config>::MultiCurrency as MultiCurrency<
		<T as frame_system::Config>::AccountId,
	>>::CurrencyId: From<CurrencyId>,
{
	let who: T::AccountId = whitelisted_caller();
	let value = T::MinContribution::get();
	assert_ok!(Salp::<T>::set_balance(&who, value));
	XcmWeightAndFee::<T>::insert(
		bifrost_xcm_interface::CurrencyIdOf::<T>::from(KSM.into()),
		XcmOperationType::UmpContributeTransact,
		(
			Weight::from_parts(4000000000, 100000),
			bifrost_xcm_interface::BalanceOf::<T>::from(4000000000u32),
		),
	);
	assert_ok!(Salp::<T>::contribute(RawOrigin::Signed(who.clone()).into(), index, value));
	QueryIdContributionInfo::<T>::insert(0u64, (index, who.clone(), value));
	MultisigConfirmAccount::<T>::put(who.clone());
	(who, value)
}

#[benchmarks(
where T: Config + bifrost_stable_pool::Config + bifrost_stable_asset::Config + orml_tokens::Config<CurrencyId = CurrencyId> + bifrost_vtoken_minting::Config + bifrost_xcm_interface::Config + zenlink_protocol::Config<AssetId = zenlink_protocol::AssetId>,
<<T as bifrost_xcm_interface::Config>::MultiCurrency as MultiCurrency<<T as frame_system::Config>::AccountId>>::CurrencyId: From<CurrencyId>
)]
mod benchmarks {
	use super::*;
	use scale_info::prelude::vec;

	#[benchmark]
	fn refund() {
		let fund_index = create_fund::<T>(1);
		let (caller, contribution) = contribute_fund::<T>(fund_index);
		assert_ok!(Pallet::<T>::confirm_contribute(
			RawOrigin::Signed(caller.clone()).into(),
			0u64,
			true
		));

		assert_ok!(Salp::<T>::fund_fail(RawOrigin::Root.into(), fund_index));
		assert_ok!(Salp::<T>::withdraw(RawOrigin::Root.into(), fund_index));
		let fund = Funds::<T>::get(fund_index).unwrap();
		let (_, status) = Salp::<T>::contribution(fund.trie_index, &caller);
		assert_eq!(status, ContributionStatus::Idle);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), fund_index, 0u32.into(), 7u32.into(), contribution);

		let (_, status) = Salp::<T>::contribution(fund.trie_index, &caller);
		assert_eq!(status, ContributionStatus::Idle);
		assert_last_event::<T>(
			Event::<T>::Refunded(
				caller.clone(),
				fund_index,
				0u32.into(),
				7u32.into(),
				contribution,
			)
			.into(),
		)
	}

	#[benchmark]
	fn redeem() {
		let fund_index = create_fund::<T>(1);
		let (caller, contribution) = contribute_fund::<T>(fund_index);
		assert_ok!(Pallet::<T>::confirm_contribute(
			RawOrigin::Signed(caller.clone()).into(),
			0u64,
			true
		));

		assert_ok!(Salp::<T>::fund_success(RawOrigin::Root.into(), fund_index));
		assert_ok!(Salp::<T>::unlock(
			RawOrigin::Signed(caller.clone()).into(),
			caller.clone(),
			fund_index
		));
		assert_ok!(Salp::<T>::fund_retire(RawOrigin::Root.into(), fund_index));
		assert_ok!(Salp::<T>::withdraw(RawOrigin::Root.into(), fund_index));
		assert_eq!(RedeemPool::<T>::get(), T::MinContribution::get());

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), fund_index, contribution);

		assert_eq!(RedeemPool::<T>::get(), 0_u32.saturated_into());
	}

	#[benchmark]
	fn fund_retire() {
		let fund_index = create_fund::<T>(1);
		let (caller, _) = contribute_fund::<T>(fund_index);
		assert_ok!(Pallet::<T>::confirm_contribute(
			RawOrigin::Signed(caller.clone()).into(),
			0u64,
			true
		));

		assert_ok!(Salp::<T>::fund_success(RawOrigin::Root.into(), fund_index));
		assert_ok!(Salp::<T>::unlock(
			RawOrigin::Signed(caller.clone()).into(),
			caller.clone(),
			fund_index
		));
		#[extrinsic_call]
		_(RawOrigin::Root, fund_index);
	}

	#[benchmark]
	fn fund_end() {
		let fund_index = create_fund::<T>(1);
		let (caller, _) = contribute_fund::<T>(fund_index);
		assert_ok!(Pallet::<T>::confirm_contribute(
			RawOrigin::Signed(caller.clone()).into(),
			0u64,
			true
		));

		assert_ok!(Salp::<T>::fund_fail(RawOrigin::Root.into(), fund_index));
		assert_ok!(Salp::<T>::withdraw(RawOrigin::Root.into(), fund_index));
		#[extrinsic_call]
		_(RawOrigin::Root, fund_index);
	}

	#[benchmark]
	fn edit() {
		create_fund::<T>(2001u32);
		#[extrinsic_call]
		_(
			RawOrigin::Root,
			2001u32,
			BalanceOf::<T>::max_value(),
			BalanceOf::<T>::max_value(),
			0u32.into(),
			8u32.into(),
			None,
		);
	}

	#[benchmark]
	fn withdraw() {
		let fund_index = create_fund::<T>(1);
		contribute_fund::<T>(fund_index);

		assert_ok!(Salp::<T>::fund_fail(RawOrigin::Root.into(), fund_index));
		#[extrinsic_call]
		_(RawOrigin::Root, fund_index)
	}

	#[benchmark]
	fn dissolve_refunded() {
		let fund_index = create_fund::<T>(1);
		let (caller, _) = contribute_fund::<T>(fund_index);
		assert_ok!(Pallet::<T>::confirm_contribute(
			RawOrigin::Signed(caller.clone()).into(),
			0u64,
			true
		));

		assert_ok!(Salp::<T>::fund_fail(RawOrigin::Root.into(), fund_index));
		assert_ok!(Salp::<T>::withdraw(RawOrigin::Root.into(), fund_index));
		assert_ok!(Salp::<T>::continue_fund(
			RawOrigin::Root.into(),
			fund_index,
			2,
			T::SlotLength::get() + 1
		));
		#[extrinsic_call]
		_(RawOrigin::Root, fund_index, 0, 7)
	}

	#[benchmark]
	fn dissolve() {
		let fund_index = create_fund::<T>(1);
		let (caller, _) = contribute_fund::<T>(fund_index);
		assert_ok!(Pallet::<T>::confirm_contribute(
			RawOrigin::Signed(caller.clone()).into(),
			0u64,
			true
		));

		assert_ok!(Salp::<T>::fund_success(RawOrigin::Root.into(), fund_index));
		assert_ok!(Salp::<T>::fund_retire(RawOrigin::Root.into(), fund_index));
		assert_ok!(Salp::<T>::withdraw(RawOrigin::Root.into(), fund_index));
		assert_ok!(Salp::<T>::fund_end(RawOrigin::Root.into(), fund_index));
		#[extrinsic_call]
		_(RawOrigin::Root, fund_index)
	}

	#[benchmark]
	fn buyback_vstoken_by_stable_pool() {
		let caller: T::AccountId = whitelisted_caller();
		let fee_account: T::AccountId = account("seed", 1, 1);
		let buyback_account: T::AccountId = T::BuybackPalletId::get().into_account_truncating();

		let amounts1: AtLeast64BitUnsignedOf<T> = 1_000_000_000_000u128.into();
		let amounts: <T as bifrost_stable_asset::pallet::Config>::Balance = amounts1.into();
		assert_ok!(bifrost_stable_pool::Pallet::<T>::create_pool(
			RawOrigin::Root.into(),
			vec![KSM.into(), VSKSM.into()],
			vec![1u128.into(), 1u128.into()],
			0u128.into(),
			0u128.into(),
			0u128.into(),
			220u128.into(),
			fee_account.clone(),
			fee_account.clone(),
			1000000000000u128.into()
		));
		assert_ok!(bifrost_stable_pool::Pallet::<T>::edit_token_rate(
			RawOrigin::Root.into(),
			0,
			vec![
				(VSKSM.into(), (1u128.into(), 1u128.into())),
				(KSM.into(), (10u128.into(), 30u128.into()))
			]
		));

		assert_ok!(<T as pallet::Config>::MultiCurrency::deposit(
			KSM,
			&buyback_account,
			BalanceOf::<T>::unique_saturated_from(1_000_000_000_000_000_000u128)
		));
		assert_ok!(<T as pallet::Config>::MultiCurrency::deposit(
			KSM,
			&caller,
			BalanceOf::<T>::unique_saturated_from(1_000_000_000_000_000_000u128)
		));
		assert_ok!(<T as pallet::Config>::MultiCurrency::deposit(
			VSKSM,
			&caller,
			BalanceOf::<T>::unique_saturated_from(1_000_000_000_000_000_000u128)
		));

		assert_ok!(bifrost_stable_pool::Pallet::<T>::add_liquidity(
			RawOrigin::Signed(caller.clone()).into(),
			0,
			vec![amounts, amounts],
			amounts
		));
		let minimum_mint_value =
			bifrost_vtoken_minting::BalanceOf::<T>::unique_saturated_from(0u128);
		let token_amount =
			bifrost_vtoken_minting::BalanceOf::<T>::unique_saturated_from(1_000_000_000_000u128);
		assert_ok!(bifrost_vtoken_minting::Pallet::<T>::set_minimum_mint(
			RawOrigin::Root.into(),
			KSM,
			minimum_mint_value
		));
		assert_ok!(bifrost_vtoken_minting::Pallet::<T>::mint(
			RawOrigin::Signed(caller.clone()).into(),
			KSM,
			token_amount,
			BoundedVec::default(),
			None
		));
		#[extrinsic_call]
		_(RawOrigin::Signed(caller), 0, KSM, 1_000_000_000u32.into())
	}

	//   `cargo test -p pallet-example-basic --all-features`, you will see one line per case:
	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
