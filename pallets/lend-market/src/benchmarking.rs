//! LendMarket pallet benchmarking.

#![cfg(feature = "runtime-benchmarks")]
pub use crate::{AccountBorrows, Pallet as LendMarket, *};
use bifrost_primitives::{currency::PHA, Balance, CurrencyId, DOT, DOT_U, KSM, VKSM, VSKSM};
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::{
	assert_ok,
	traits::tokens::{Fortitude, Precision},
};
use frame_system::{self, RawOrigin as SystemOrigin};
use rate_model::{InterestRateModel, JumpModel};
use sp_std::prelude::*;

pub const LKSM: CurrencyId = CurrencyId::Token2(11);
pub const LUSDT: CurrencyId = CurrencyId::Token2(12);
const SEED: u32 = 0;

const RATE_MODEL_MOCK: InterestRateModel = InterestRateModel::Jump(JumpModel {
	base_rate: Rate::from_inner(Rate::DIV / 100 * 2),
	jump_rate: Rate::from_inner(Rate::DIV / 100 * 10),
	full_rate: Rate::from_inner(Rate::DIV / 100 * 32),
	jump_utilization: Ratio::from_percent(80),
});

fn market_mock<T: Config>() -> Market<BalanceOf<T>> {
	Market {
		close_factor: Ratio::from_percent(50),
		collateral_factor: Ratio::from_percent(50),
		liquidation_threshold: Ratio::from_percent(55),
		liquidate_incentive: Rate::from_inner(Rate::DIV / 100 * 110),
		state: MarketState::Active,
		rate_model: InterestRateModel::Jump(JumpModel {
			base_rate: Rate::from_inner(Rate::DIV / 100 * 2),
			jump_rate: Rate::from_inner(Rate::DIV / 100 * 10),
			full_rate: Rate::from_inner(Rate::DIV / 100 * 32),
			jump_utilization: Ratio::from_percent(80),
		}),
		reserve_factor: Ratio::from_percent(15),
		liquidate_incentive_reserved_factor: Ratio::from_percent(3),
		supply_cap: 1_000_000_000_000_000_000_000u128, // set to 1B
		borrow_cap: 1_000_000_000_000_000_000_000u128, // set to 1B
		lend_token_id: LKSM,
	}
}

fn pending_market_mock<T: Config>(lend_token_id: CurrencyId) -> Market<BalanceOf<T>> {
	let mut market = market_mock::<T>();
	market.state = MarketState::Pending;
	market.lend_token_id = lend_token_id;
	market
}

const INITIAL_AMOUNT: u32 = 500_000_000;

fn transfer_initial_balance<
	T: Config + pallet_prices::Config + pallet_balances::Config<Balance = Balance>,
>(
	caller: T::AccountId,
) {
	let account_id = T::Lookup::unlookup(caller.clone());
	pallet_balances::Pallet::<T>::force_set_balance(
		SystemOrigin::Root.into(),
		account_id,
		10_000_000_000_000_u128,
	)
	.unwrap();
	<T as pallet::Config>::Assets::mint_into(DOT_U, &caller, INITIAL_AMOUNT.into()).unwrap();
	<T as pallet::Config>::Assets::mint_into(KSM, &caller, INITIAL_AMOUNT.into()).unwrap();
	<T as pallet::Config>::Assets::mint_into(VKSM, &caller, INITIAL_AMOUNT.into()).unwrap();
	<T as pallet::Config>::Assets::mint_into(DOT, &caller, INITIAL_AMOUNT.into()).unwrap();
	<T as pallet::Config>::Assets::mint_into(PHA, &caller, INITIAL_AMOUNT.into()).unwrap();
	pallet_prices::Pallet::<T>::set_price(SystemOrigin::Root.into(), DOT_U, 1.into()).unwrap();
	pallet_prices::Pallet::<T>::set_price(SystemOrigin::Root.into(), KSM, 1.into()).unwrap();
	pallet_prices::Pallet::<T>::set_price(SystemOrigin::Root.into(), VKSM, 1.into()).unwrap();
	pallet_prices::Pallet::<T>::set_price(SystemOrigin::Root.into(), DOT, 1.into()).unwrap();
	pallet_prices::Pallet::<T>::set_price(SystemOrigin::Root.into(), PHA, 1.into()).unwrap();
}

fn set_account_borrows<T: Config>(
	who: T::AccountId,
	asset_id: AssetIdOf<T>,
	borrow_balance: BalanceOf<T>,
) {
	AccountBorrows::<T>::insert(
		asset_id,
		&who,
		BorrowSnapshot { principal: borrow_balance, borrow_index: Rate::one() },
	);
	TotalBorrows::<T>::insert(asset_id, borrow_balance);
	T::Assets::burn_from(
		asset_id,
		&who,
		borrow_balance,
		Preservation::Protect,
		Precision::Exact,
		Fortitude::Force,
	)
	.unwrap();
}

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

benchmarks! {
	where_clause {
		where
			T: pallet_prices::Config + pallet_balances::Config<Balance = Balance> // pallet_assets::Config<AssetId = CurrencyId, Balance = Balance> +
	}

	add_market {
	}: _(SystemOrigin::Root, VKSM, pending_market_mock::<T>(VSKSM))
	verify {
		assert_last_event::<T>(Event::<T>::NewMarket(VKSM, pending_market_mock::<T>(VSKSM)).into());
	}

	activate_market {
		assert_ok!(LendMarket::<T>::add_market(SystemOrigin::Root.into(), VKSM, pending_market_mock::<T>(VSKSM)));
	}: _(SystemOrigin::Root, VKSM)
	verify {
		assert_last_event::<T>(Event::<T>::ActivatedMarket(VKSM).into());
	}

	update_rate_model {
		assert_ok!(LendMarket::<T>::add_market(SystemOrigin::Root.into(), DOT_U, pending_market_mock::<T>(LUSDT)));
	}: _(SystemOrigin::Root, DOT_U, RATE_MODEL_MOCK)
	verify {
		let mut market = pending_market_mock::<T>(LUSDT);
		market.rate_model = RATE_MODEL_MOCK;
		assert_last_event::<T>(Event::<T>::UpdatedMarket(DOT_U, market).into());
	}

	update_market {
		assert_ok!(LendMarket::<T>::add_market(SystemOrigin::Root.into(), KSM, pending_market_mock::<T>(LKSM)));
	}: _(
		SystemOrigin::Root,
		KSM,
		Some(Ratio::from_percent(50)),
		Some(Ratio::from_percent(55)),
		Some(Ratio::from_percent(50)),
		Some(Ratio::from_percent(15)),
		Some(Ratio::from_percent(3)),
		Some(Rate::from_inner(Rate::DIV / 100 * 110)),
		Some(1_000_000_000_000_000_000_000u128),
		Some(1_000_000_000_000_000_000_000u128)
	)
	verify {
		let mut market = pending_market_mock::<T>(LKSM);
		market.reserve_factor = Ratio::from_percent(50);
		market.close_factor = Ratio::from_percent(15);
		assert_last_event::<T>(Event::<T>::UpdatedMarket(KSM, market).into());
	}

	force_update_market {
		assert_ok!(LendMarket::<T>::add_market(SystemOrigin::Root.into(), DOT_U, pending_market_mock::<T>(LUSDT)));
		let caller: T::AccountId = whitelisted_caller();
		transfer_initial_balance::<T>(caller.clone());
	}: _(SystemOrigin::Root,DOT_U, pending_market_mock::<T>(LUSDT))
	verify {
		assert_last_event::<T>(Event::<T>::UpdatedMarket(DOT_U, pending_market_mock::<T>(LUSDT)).into());
	}

	add_reward {
		let caller: T::AccountId = whitelisted_caller();
		transfer_initial_balance::<T>(caller.clone());
	}: _(SystemOrigin::Signed(caller.clone()), 1_000_000_000_000_u128)
	verify {
		assert_last_event::<T>(Event::<T>::RewardAdded(caller, 1_000_000_000_000_u128).into());
	}

	withdraw_missing_reward {
		let caller: T::AccountId = whitelisted_caller();
		transfer_initial_balance::<T>(caller.clone());
		assert_ok!(LendMarket::<T>::add_reward(SystemOrigin::Signed(caller.clone()).into(), 1_000_000_000_000_u128));
		let receiver = T::Lookup::unlookup(caller.clone());
	}: _(SystemOrigin::Root, receiver, 500_000_000_000_u128)
	verify {
		assert_last_event::<T>(Event::<T>::RewardWithdrawn(caller, 500_000_000_000_u128).into());
	}

	update_market_reward_speed {
		assert_ok!(LendMarket::<T>::add_market(SystemOrigin::Root.into(), DOT_U, pending_market_mock::<T>(DOT_U)));
		assert_ok!(LendMarket::<T>::activate_market(SystemOrigin::Root.into(), DOT_U));
	}: _(SystemOrigin::Root, DOT_U, Some(1_000_000), Some(1_000_000))
	verify {
		assert_last_event::<T>(Event::<T>::MarketRewardSpeedUpdated(DOT_U, 1_000_000, 1_000_000).into());
	}

	claim_reward {
		let caller: T::AccountId = whitelisted_caller();
		transfer_initial_balance::<T>(caller.clone());
		assert_ok!(LendMarket::<T>::add_market(SystemOrigin::Root.into(), DOT_U, pending_market_mock::<T>(DOT_U)));
		assert_ok!(LendMarket::<T>::activate_market(SystemOrigin::Root.into(), DOT_U));
		assert_ok!(LendMarket::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), DOT_U, 100_000_000));
		assert_ok!(LendMarket::<T>::add_reward(SystemOrigin::Signed(caller.clone()).into(), 1_000_000_000_000_u128));
		assert_ok!(LendMarket::<T>::update_market_reward_speed(SystemOrigin::Root.into(), DOT_U, Some(1_000_000), Some(1_000_000)));
		let target_height = frame_system::Pallet::<T>::block_number().saturating_add(One::one());
		frame_system::Pallet::<T>::set_block_number(target_height);
	}: _(SystemOrigin::Signed(caller.clone()))
	verify {
		assert_last_event::<T>(Event::<T>::RewardPaid(caller, 1_000_000).into());
	}

	claim_reward_for_market {
		let caller: T::AccountId = whitelisted_caller();
		transfer_initial_balance::<T>(caller.clone());
		assert_ok!(LendMarket::<T>::add_market(SystemOrigin::Root.into(), DOT_U, pending_market_mock::<T>(DOT_U)));
		assert_ok!(LendMarket::<T>::activate_market(SystemOrigin::Root.into(), DOT_U));
		assert_ok!(LendMarket::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), DOT_U, 100_000_000));
		assert_ok!(LendMarket::<T>::add_reward(SystemOrigin::Signed(caller.clone()).into(), 1_000_000_000_000_u128));
		assert_ok!(LendMarket::<T>::update_market_reward_speed(SystemOrigin::Root.into(), DOT_U, Some(1_000_000), Some(1_000_000)));
		let target_height = frame_system::Pallet::<T>::block_number().saturating_add(One::one());
		frame_system::Pallet::<T>::set_block_number(target_height);
	}: _(SystemOrigin::Signed(caller.clone()), DOT_U)
	verify {
		assert_last_event::<T>(Event::<T>::RewardPaid(caller, 1_000_000).into());
	}


	mint {
		let caller: T::AccountId = whitelisted_caller();
		transfer_initial_balance::<T>(caller.clone());
		assert_ok!(LendMarket::<T>::add_market(SystemOrigin::Root.into(), DOT_U, pending_market_mock::<T>(DOT_U)));
		assert_ok!(LendMarket::<T>::activate_market(SystemOrigin::Root.into(), DOT_U));
		let amount: u32 = 100_000;
	}: _(SystemOrigin::Signed(caller.clone()), DOT_U, amount.into())
	verify {
		assert_last_event::<T>(Event::<T>::Deposited(caller, DOT_U, amount.into()).into());
	}

	borrow {
		let caller: T::AccountId = whitelisted_caller();
		transfer_initial_balance::<T>(caller.clone());
		let deposit_amount: u32 = 200_000_000;
		let borrowed_amount: u32 = 100_000_000;
		assert_ok!(LendMarket::<T>::add_market_bond(
			SystemOrigin::Root.into(),
			DOT_U,
			vec![DOT_U]
		));
		assert_ok!(LendMarket::<T>::add_market(SystemOrigin::Root.into(), DOT_U, pending_market_mock::<T>(LUSDT)));
		assert_ok!(LendMarket::<T>::activate_market(SystemOrigin::Root.into(), DOT_U));
		assert_ok!(LendMarket::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), DOT_U, deposit_amount.into()));
		assert_ok!(LendMarket::<T>::collateral_asset(SystemOrigin::Signed(caller.clone()).into(), DOT_U, true));
	}: _(SystemOrigin::Signed(caller.clone()), DOT_U, borrowed_amount.into())
	verify {
		assert_last_event::<T>(Event::<T>::Borrowed(caller, DOT_U, borrowed_amount.into()).into());
	}

	redeem {
		let caller: T::AccountId = whitelisted_caller();
		transfer_initial_balance::<T>(caller.clone());
		let deposit_amount: u32 = 100_000_000;
		let redeem_amount: u32 = 100_000;
		assert_ok!(LendMarket::<T>::add_market(SystemOrigin::Root.into(), DOT_U, pending_market_mock::<T>(LUSDT)));
		assert_ok!(LendMarket::<T>::activate_market(SystemOrigin::Root.into(), DOT_U));
		assert_ok!(LendMarket::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), DOT_U, deposit_amount.into()));
	}: _(SystemOrigin::Signed(caller.clone()), DOT_U, redeem_amount.into())
	verify {
		assert_last_event::<T>(Event::<T>::Redeemed(caller, DOT_U, redeem_amount.into()).into());
	}

	redeem_all {
		let caller: T::AccountId = whitelisted_caller();
		transfer_initial_balance::<T>(caller.clone());
		let deposit_amount: u32 = 100_000_000;
		assert_ok!(LendMarket::<T>::add_market(SystemOrigin::Root.into(), DOT_U, pending_market_mock::<T>(LUSDT)));
		assert_ok!(LendMarket::<T>::activate_market(SystemOrigin::Root.into(), DOT_U));
		assert_ok!(LendMarket::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), DOT_U, deposit_amount.into()));
	}: _(SystemOrigin::Signed(caller.clone()), DOT_U)
	verify {
		assert_last_event::<T>(Event::<T>::Redeemed(caller, DOT_U, deposit_amount.into()).into());
	}

	repay_borrow {
		let caller: T::AccountId = whitelisted_caller();
		transfer_initial_balance::<T>(caller.clone());
		let deposit_amount: u32 = 200_000_000;
		let borrowed_amount: u32 = 100_000_000;
		let repay_amount: u32 = 100;
		assert_ok!(LendMarket::<T>::add_market_bond(
			SystemOrigin::Root.into(),
			DOT_U,
			vec![DOT_U]
		));
		assert_ok!(LendMarket::<T>::add_market(SystemOrigin::Root.into(), DOT_U, pending_market_mock::<T>(LUSDT)));
		assert_ok!(LendMarket::<T>::activate_market(SystemOrigin::Root.into(), DOT_U));
		assert_ok!(LendMarket::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), DOT_U, deposit_amount.into()));
		assert_ok!(LendMarket::<T>::collateral_asset(SystemOrigin::Signed(caller.clone()).into(), DOT_U, true));
		assert_ok!(LendMarket::<T>::borrow(SystemOrigin::Signed(caller.clone()).into(), DOT_U, borrowed_amount.into()));
	}: _(SystemOrigin::Signed(caller.clone()), DOT_U, repay_amount.into())
	verify {
		assert_last_event::<T>(Event::<T>::RepaidBorrow(caller, DOT_U, repay_amount.into()).into());
	}

	repay_borrow_all {
		let caller: T::AccountId = whitelisted_caller();
		transfer_initial_balance::<T>(caller.clone());
		let deposit_amount: u32 = 200_000_000;
		let borrowed_amount: u32 = 100_000_000;
		assert_ok!(LendMarket::<T>::add_market(SystemOrigin::Root.into(), DOT_U, pending_market_mock::<T>(LUSDT)));
		assert_ok!(LendMarket::<T>::add_market_bond(
			SystemOrigin::Root.into(),
			DOT_U,
			vec![DOT_U]
		));
		assert_ok!(LendMarket::<T>::activate_market(SystemOrigin::Root.into(), DOT_U));
		assert_ok!(LendMarket::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), DOT_U, deposit_amount.into()));
		assert_ok!(LendMarket::<T>::collateral_asset(SystemOrigin::Signed(caller.clone()).into(), DOT_U, true));
		assert_ok!(LendMarket::<T>::borrow(SystemOrigin::Signed(caller.clone()).into(), DOT_U, borrowed_amount.into()));
	}: _(SystemOrigin::Signed(caller.clone()), DOT_U)
	verify {
		assert_last_event::<T>(Event::<T>::RepaidBorrow(caller, DOT_U, borrowed_amount.into()).into());
	}

	collateral_asset {
		let caller: T::AccountId = whitelisted_caller();
		transfer_initial_balance::<T>(caller.clone());
		let deposit_amount: u32 = 200_000_000;
		assert_ok!(LendMarket::<T>::add_market(SystemOrigin::Root.into(), DOT_U, pending_market_mock::<T>(LUSDT)));
		assert_ok!(LendMarket::<T>::activate_market(SystemOrigin::Root.into(), DOT_U));
		assert_ok!(LendMarket::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), DOT_U, deposit_amount.into()));
	}: _(SystemOrigin::Signed(caller.clone()), DOT_U, true)
	verify {
		assert_last_event::<T>(Event::<T>::CollateralAssetAdded(caller, DOT_U).into());
	}

	liquidate_borrow {
		let alice: T::AccountId = account("Sample", 100, SEED);
		let bob: T::AccountId = account("Sample", 101, SEED);
		transfer_initial_balance::<T>(alice.clone());
		transfer_initial_balance::<T>(bob.clone());
		let deposit_amount: u32 = 200_000_000;
		let borrowed_amount: u32 = 200_000_000;
		let liquidate_amount: u32 = 100_000_000;
		let incentive_amount: u32 = 110_000_000;
		assert_ok!(LendMarket::<T>::add_market(SystemOrigin::Root.into(), VKSM, pending_market_mock::<T>(VSKSM)));
		assert_ok!(LendMarket::<T>::activate_market(SystemOrigin::Root.into(), VKSM));
		assert_ok!(LendMarket::<T>::add_market(SystemOrigin::Root.into(), KSM, pending_market_mock::<T>(LKSM)));
		assert_ok!(LendMarket::<T>::activate_market(SystemOrigin::Root.into(), KSM));
		assert_ok!(LendMarket::<T>::mint(SystemOrigin::Signed(bob.clone()).into(), KSM, deposit_amount.into()));
		assert_ok!(LendMarket::<T>::mint(SystemOrigin::Signed(alice.clone()).into(), VKSM, deposit_amount.into()));
		assert_ok!(LendMarket::<T>::collateral_asset(SystemOrigin::Signed(alice.clone()).into(), VKSM, true));
		set_account_borrows::<T>(alice.clone(), KSM, borrowed_amount.into());
	}: _(SystemOrigin::Signed(bob.clone()), alice.clone(), KSM, liquidate_amount.into(), VKSM)
	verify {
		assert_last_event::<T>(Event::<T>::LiquidatedBorrow(bob.clone(), alice.clone(), KSM, VKSM, liquidate_amount.into(), incentive_amount.into()).into());
	}

	add_reserves {
		let caller: T::AccountId = whitelisted_caller();
		let payer = T::Lookup::unlookup(caller.clone());
		transfer_initial_balance::<T>(caller.clone());
		let amount: u32 = 2000;
		assert_ok!(LendMarket::<T>::add_market(SystemOrigin::Root.into(), DOT_U, pending_market_mock::<T>(LUSDT)));
		assert_ok!(LendMarket::<T>::activate_market(SystemOrigin::Root.into(), DOT_U));
	}: _(SystemOrigin::Root, payer, DOT_U, amount.into())
	verify {
		assert_last_event::<T>(Event::<T>::ReservesAdded(caller, DOT_U, amount.into(), amount.into()).into());
	}

	reduce_reserves {
		let caller: T::AccountId = whitelisted_caller();
		let payer = T::Lookup::unlookup(caller.clone());
		transfer_initial_balance::<T>(caller.clone());
		let add_amount: u32 = 2000;
		let reduce_amount: u32 = 1000;
		assert_ok!(LendMarket::<T>::add_market(SystemOrigin::Root.into(), DOT_U, pending_market_mock::<T>(LUSDT)));
		assert_ok!(LendMarket::<T>::activate_market(SystemOrigin::Root.into(), DOT_U));
		assert_ok!(LendMarket::<T>::add_reserves(SystemOrigin::Root.into(), payer.clone(), DOT_U, add_amount.into()));
	}: _(SystemOrigin::Root, payer, DOT_U, reduce_amount.into())
	verify {
		assert_last_event::<T>(Event::<T>::ReservesReduced(caller, DOT_U, reduce_amount.into(), (add_amount-reduce_amount).into()).into());
	}

	update_liquidation_free_collateral {

	}: _(SystemOrigin::Root, vec![PHA])
	verify {
		assert_last_event::<T>(Event::<T>::LiquidationFreeCollateralsUpdated(vec![PHA]).into());
	}
}

impl_benchmark_test_suite!(LendMarket, crate::mock::new_test_ext(), crate::mock::Test);
