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
use crate::{Pallet, *};
pub use bifrost_primitives::{Balance, CurrencyId, KSM, VKSM, *};
use frame_benchmarking::{account, v2::*};
use frame_support::assert_ok;
use frame_system::RawOrigin as SystemOrigin;
use lend_market::{self, InterestRateModel, JumpModel, Market, MarketState};
use orml_traits::MultiCurrency;
use sp_runtime::{
	traits::{StaticLookup, UniqueSaturatedFrom},
	FixedPointNumber,
};
use sp_std::vec;

pub fn unit(d: u128) -> u128 {
	d.saturating_mul(10_u128.pow(12))
}

pub const fn market_mock(lend_token_id: CurrencyId) -> Market<Balance> {
	Market {
		close_factor: Ratio::from_percent(50),
		collateral_factor: Ratio::from_percent(50),
		liquidation_threshold: Ratio::from_percent(55),
		liquidate_incentive: Rate::from_inner(Rate::DIV / 100 * 110),
		liquidate_incentive_reserved_factor: Ratio::from_percent(3),
		state: MarketState::Pending,
		rate_model: InterestRateModel::Jump(JumpModel {
			base_rate: Rate::from_inner(Rate::DIV / 100 * 2),
			jump_rate: Rate::from_inner(Rate::DIV / 100 * 10),
			full_rate: Rate::from_inner(Rate::DIV / 100 * 32),
			jump_utilization: Ratio::from_percent(80),
		}),
		reserve_factor: Ratio::from_percent(15),
		supply_cap: 1_000_000_000_000_000_000_000u128, // set to 1B
		borrow_cap: 1_000_000_000_000_000_000_000u128, // set to 1B
		lend_token_id,
	}
}

fn init<
	T: Config
		+ bifrost_stable_pool::Config
		+ bifrost_vtoken_minting::Config
		+ pallet_prices::Config
		+ pallet_balances::Config<Balance = Balance>,
>() -> Result<(), BenchmarkError> {
	let caller: AccountIdOf<T> = account("caller", 1, SEED);
	let account_id = T::Lookup::unlookup(caller.clone());
	pallet_balances::Pallet::<T>::force_set_balance(
		SystemOrigin::Root.into(),
		account_id,
		10_000_000_000_000_u128,
	)
	.unwrap();
	pallet_prices::Pallet::<T>::set_price(SystemOrigin::Root.into(), KSM, 1.into()).unwrap();
	pallet_prices::Pallet::<T>::set_price(SystemOrigin::Root.into(), VKSM, 1.into()).unwrap();
	<T as bifrost_stable_pool::Config>::MultiCurrency::deposit(
		KSM.into(),
		&caller,
		<T as bifrost_stable_asset::Config>::Balance::from(unit(1_000_000).into()),
	)?;
	<T as bifrost_stable_pool::Config>::MultiCurrency::deposit(
		VKSM.into(),
		&caller,
		<T as bifrost_stable_asset::Config>::Balance::from(unit(1_000_000).into()),
	)?;
	let fee_account: AccountIdOf<T> = account("caller", 2, 2);
	pallet_balances::Pallet::<T>::force_set_balance(
		SystemOrigin::Root.into(),
		T::Lookup::unlookup(caller.clone()),
		10_000_000_000_000_u128,
	)
	.unwrap();
	pallet_balances::Pallet::<T>::force_set_balance(
		SystemOrigin::Root.into(),
		T::Lookup::unlookup(fee_account.clone()),
		10_000_000_000_000_u128,
	)
	.unwrap();

	assert_ok!(lend_market::Pallet::<T>::add_market(
		SystemOrigin::Root.into(),
		KSM,
		market_mock(VKSM)
	));
	assert_ok!(lend_market::Pallet::<T>::activate_market(SystemOrigin::Root.into(), KSM));
	assert_ok!(lend_market::Pallet::<T>::add_market(
		SystemOrigin::Root.into(),
		VKSM,
		market_mock(VBNC)
	));
	assert_ok!(lend_market::Pallet::<T>::activate_market(SystemOrigin::Root.into(), VKSM));
	assert_ok!(lend_market::Pallet::<T>::mint(
		SystemOrigin::Signed(caller.clone()).into(),
		KSM,
		unit(100)
	));

	let coin0 = KSM;
	let coin1 = VKSM;
	let amounts = vec![
		<T as bifrost_stable_asset::Config>::Balance::from(unit(100u128).into()),
		<T as bifrost_stable_asset::Config>::Balance::from(unit(100u128).into()),
	];
	assert_ok!(bifrost_stable_pool::Pallet::<T>::create_pool(
		SystemOrigin::Root.into(),
		vec![coin0.into(), coin1.into()],
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
		SystemOrigin::Root.into(),
		0,
		vec![
			(KSM.into(), (1u128.into(), 1u128.into())),
			(VKSM.into(), (90_000_000u128.into(), 100_000_000u128.into()))
		]
	));
	assert_ok!(bifrost_stable_pool::Pallet::<T>::add_liquidity(
		SystemOrigin::Signed(caller.clone()).into(),
		0,
		amounts,
		<T as bifrost_stable_asset::Config>::Balance::zero()
	));

	assert_ok!(bifrost_vtoken_minting::Pallet::<T>::mint(
		SystemOrigin::Signed(caller.clone()).into(),
		KSM,
		bifrost_vtoken_minting::BalanceOf::<T>::unique_saturated_from(unit(100u128)),
		BoundedVec::default(),
		None
	));
	assert_ok!(lend_market::Pallet::<T>::mint(
		SystemOrigin::Signed(caller.clone()).into(),
		VKSM,
		lend_market::BalanceOf::<T>::unique_saturated_from(unit(1u128))
	));
	assert_ok!(lend_market::Pallet::<T>::add_market_bond(
		SystemOrigin::Root.into(),
		KSM,
		vec![VKSM]
	));

	Ok(())
}

const SEED: u32 = 1;

#[benchmarks(where T: Config + bifrost_stable_pool::Config + bifrost_vtoken_minting::Config + bifrost_stable_asset::pallet::Config + pallet_prices::Config + pallet_balances::Config<Balance = Balance> )]
mod benchmarks {
	use lend_market::AccountIdOf;

	use super::*;

	#[benchmark]
	fn flash_loan_deposit() -> Result<(), BenchmarkError> {
		init::<T>()?;
		let caller: AccountIdOf<T> = account("caller", 1, SEED);
		let coin0 = KSM;
		let rate = FixedU128::from_inner(unit(990_000));

		#[extrinsic_call]
		Pallet::<T>::flash_loan_deposit(SystemOrigin::Signed(caller.clone()), coin0.into(), rate);

		Ok(())
	}

	impl_benchmark_test_suite!(
		Pallet,
		crate::mock::ExtBuilder::default().new_test_ext().build(),
		crate::mock::Test
	);
}
