use crate::{Pallet as LeverageStaking, *};
// use frame_benchmarking::{account, benchmarks, vec, whitelisted_caller};
// use frame_support::assert_ok;
use frame_benchmarking::{account, v2::*};
use frame_support::assert_ok;
use frame_system::RawOrigin as SystemOrigin;
use lend_market::{self, InterestRateModel, JumpModel, Market, MarketState};
pub use node_primitives::{
	AccountId, Balance, CurrencyId, CurrencyIdMapping, SlpOperator, SlpxOperator, TokenSymbol, BNC,
	DOT, GLMR, KSM, VDOT, *,
};
use orml_traits::MultiCurrency;
use sp_runtime::{
	traits::{StaticLookup, UniqueSaturatedFrom},
	FixedPointNumber,
};

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

// benchmarks! {
// 	flash_loan_deposit {
// 	let fee_account: T::AccountId = whitelisted_caller();
// 	let coin0 = DOT;
// 	let coin1 = VDOT;
// 	let rate = FixedU128::from_inner(unit(1_000_000));
// }: _(SystemOrigin::Signed(fee_account),
// 	coin0.into(),
// 	rate,
// 	Some(10000u128.into()),
// )

// 	impl_benchmark_test_suite!(LeverageStaking, crate::mock::ExtBuilder::new_test_ext().build(),
// crate::mock::Test); }

fn init<
	T: Config
		+ bifrost_stable_pool::Config
		+ bifrost_vtoken_minting::Config
		+ pallet_prices::Config
		+ pallet_balances::Config<Balance = Balance>,
>() -> Result<(), BenchmarkError> {
	let caller: AccountIdOf<T> = account("caller", 1, SEED);
	<T as bifrost_stable_pool::Config>::MultiCurrency::deposit(
		DOT.into(),
		&caller,
		<T as nutsfinance_stable_asset::Config>::Balance::from(unit(1_000_000).into()),
	)?;
	<T as bifrost_stable_pool::Config>::MultiCurrency::deposit(
		VDOT.into(),
		&caller,
		<T as nutsfinance_stable_asset::Config>::Balance::from(unit(1_000_000).into()),
	)?;
	let a = <T as lend_market::Config>::Assets::balance(DOT, &caller);
	log::debug!("aaaa{:?}", a);
	let fee_account: AccountIdOf<T> = account("caller", 2, 2);
	pallet_balances::Pallet::<T>::force_set_balance(
		SystemOrigin::Root.into(),
		// caller.into(),
		T::Lookup::unlookup(caller.clone()),
		10_000_000_000_000_u128,
	)
	.unwrap();
	pallet_balances::Pallet::<T>::force_set_balance(
		SystemOrigin::Root.into(),
		T::Lookup::unlookup(fee_account.clone()),
		// fee_account.into(),
		10_000_000_000_000_u128,
	)
	.unwrap();
	pallet_prices::Pallet::<T>::set_price(SystemOrigin::Root.into(), DOT, 1.into()).unwrap();
	pallet_prices::Pallet::<T>::set_price(SystemOrigin::Root.into(), VDOT, 1.into()).unwrap();

	let a = <T as lend_market::Config>::Assets::balance(DOT, &caller); // <T as pallet_balances::Config>::Currency::free_balance(&caller);
	log::debug!("bbbb{:?}", a);
	assert_ok!(lend_market::Pallet::<T>::add_market(
		SystemOrigin::Root.into(),
		DOT,
		market_mock(VKSM)
	));
	assert_ok!(lend_market::Pallet::<T>::activate_market(SystemOrigin::Root.into(), DOT));
	assert_ok!(lend_market::Pallet::<T>::add_market(
		SystemOrigin::Root.into(),
		VDOT,
		market_mock(VBNC)
	));
	assert_ok!(lend_market::Pallet::<T>::activate_market(SystemOrigin::Root.into(), VDOT));
	// <T as lend_market::Config>::Assets::mint_into(DOT, &caller, unit(1_000_000))?;
	let a = <T as lend_market::Config>::Assets::balance(DOT, &caller); // <T as pallet_balances::Config>::Currency::free_balance(&caller);
	log::debug!("dddd{:?}", a);
	let coin0 = DOT;
	let coin1 = VDOT;
	let amounts = vec![
		<T as nutsfinance_stable_asset::Config>::Balance::from(unit(100u128).into()),
		<T as nutsfinance_stable_asset::Config>::Balance::from(unit(100u128).into()),
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
			(DOT.into(), (1u128.into(), 1u128.into())),
			(VDOT.into(), (90_000_000u128.into(), 100_000_000u128.into()))
		]
	));
	assert_ok!(bifrost_stable_pool::Pallet::<T>::add_liquidity(
		SystemOrigin::Signed(caller.clone()).into(),
		0,
		amounts,
		<T as nutsfinance_stable_asset::Config>::Balance::zero()
	));

	// 	TimestampPallet::set_timestamp(6000);

	assert_ok!(bifrost_vtoken_minting::Pallet::<T>::set_minimum_mint(
		SystemOrigin::Root.into(),
		DOT,
		bifrost_vtoken_minting::BalanceOf::<T>::unique_saturated_from(0u128)
	));
	assert_ok!(bifrost_vtoken_minting::Pallet::<T>::mint(
		SystemOrigin::Signed(caller.clone()).into(),
		DOT,
		bifrost_vtoken_minting::BalanceOf::<T>::unique_saturated_from(unit(100u128)),
		BoundedVec::default()
	));

	// assert_ok!(bifrost_stable_pool::Pallet::<T>::create_pool(
	// 	SystemOrigin::Root.into(),
	// 	vec![DOT.into(), VDOT.into()],
	// 	vec![1u128.into(), 1u128.into()],
	// 	10000000u128.into(),
	// 	20000000u128.into(),
	// 	50000000u128.into(),
	// 	10000u128.into(),
	// 	fee_account.clone(),
	// 	fee_account,
	// 	unit(1).into(),
	// ));
	// 	assert_ok!(StablePool::edit_token_rate(
	// 		SystemOrigin::Root.into(),
	// 		0,
	// 		vec![(DOT, (1, 1)), (VDOT, (90_000_000, 100_000_000))]
	// 	));
	// let amounts = vec![unit(100), unit(100)];
	// 	assert_ok!(StablePool::add_liquidity(RuntimeOrigin::signed(0), 0, amounts, 0));

	Ok(())
}

const SEED: u32 = 1;

#[benchmarks(where T: Config + bifrost_stable_pool::Config + bifrost_vtoken_minting::Config + nutsfinance_stable_asset::pallet::Config + pallet_prices::Config + pallet_balances::Config<Balance = Balance> )]
mod benchmarks {
	use lend_market::AccountIdOf;

	use super::*;

	#[benchmark]
	fn flash_loan_deposit() -> Result<(), BenchmarkError> {
		log::info!("1111");
		// env_logger::try_init().unwrap_or(());
		init::<T>()?;
		let caller: AccountIdOf<T> = account("caller", 1, SEED);
		// <T as bifrost_stable_pool::Config>::MultiCurrency::deposit(
		// 	BNC.into(),
		// 	&caller,
		// 	<T as nutsfinance_stable_asset::Config>::Balance::from(unit(1_000_000).into()),
		// )?;
		// let caller = funded_account::<T>("caller", 0);

		let coin0 = DOT;
		let rate = FixedU128::from_inner(unit(1_000_000));
		let a = <T as lend_market::Config>::Assets::balance(DOT, &caller); // <T as pallet_balances::Config>::Currency::free_balance(&caller);
		log::debug!("fdsf{:?}", a);

		#[extrinsic_call]
		LeverageStaking::<T>::flash_loan_deposit(
			SystemOrigin::Signed(caller.clone()),
			coin0.into(),
			rate,
			Some(unit(1).into()),
		);

		Ok(())
	}

	impl_benchmark_test_suite!(LeverageStaking, crate::mock::default(), crate::mock::Test);
}
