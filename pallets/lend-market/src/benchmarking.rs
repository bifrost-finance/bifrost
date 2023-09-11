//! Loans pallet benchmarking.

#![cfg(feature = "runtime-benchmarks")]
use super::*;
use crate::{AccountBorrows, Pallet as Loans};

use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::assert_ok;
use frame_system::{self, RawOrigin as SystemOrigin};
use primitives::{
    tokens::{CDOT_6_13, DOT, KSM, PKSM, PSKSM, PUSDT, SKSM, USDT},
    Balance, CurrencyId,
};
use rate_model::{InterestRateModel, JumpModel};
use sp_std::prelude::*;

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
        ptoken_id: 1200,
    }
}

fn pending_market_mock<T: Config>(ptoken_id: CurrencyId) -> Market<BalanceOf<T>> {
    let mut market = market_mock::<T>();
    market.state = MarketState::Pending;
    market.ptoken_id = ptoken_id;
    market
}

const INITIAL_AMOUNT: u32 = 500_000_000;

fn transfer_initial_balance<
    T: Config
        + pallet_assets::Config<AssetId = CurrencyId, Balance = Balance>
        + pallet_prices::Config
        + pallet_balances::Config<Balance = Balance>,
>(
    caller: T::AccountId,
) {
    let account_id = T::Lookup::unlookup(caller.clone());
    pallet_assets::Pallet::<T>::force_create(
        SystemOrigin::Root.into(),
        KSM.into(),
        account_id.clone(),
        true,
        1,
    )
    .ok();
    pallet_assets::Pallet::<T>::force_create(
        SystemOrigin::Root.into(),
        SKSM.into(),
        account_id.clone(),
        true,
        1,
    )
    .ok();
    pallet_assets::Pallet::<T>::force_create(
        SystemOrigin::Root.into(),
        DOT.into(),
        account_id.clone(),
        true,
        1,
    )
    .ok();
    pallet_assets::Pallet::<T>::force_create(
        SystemOrigin::Root.into(),
        CDOT_6_13.into(),
        account_id.clone(),
        true,
        1,
    )
    .ok();
    pallet_assets::Pallet::<T>::force_create(
        SystemOrigin::Root.into(),
        USDT.into(),
        account_id.clone(),
        true,
        1,
    )
    .ok();
    pallet_assets::Pallet::<T>::force_set_metadata(
        SystemOrigin::Root.into(),
        KSM.into(),
        b"kusama".to_vec(),
        b"KSM".to_vec(),
        12,
        true,
    )
    .ok();
    pallet_assets::Pallet::<T>::force_set_metadata(
        SystemOrigin::Root.into(),
        SKSM.into(),
        b"xkusama".to_vec(),
        b"sKSM".to_vec(),
        12,
        true,
    )
    .ok();
    pallet_assets::Pallet::<T>::force_set_metadata(
        SystemOrigin::Root.into(),
        DOT.into(),
        b"polkadot".to_vec(),
        b"DOT".to_vec(),
        12,
        true,
    )
    .ok();
    pallet_assets::Pallet::<T>::force_set_metadata(
        SystemOrigin::Root.into(),
        CDOT_6_13.into(),
        b"cDot_6_13".to_vec(),
        b"cDot_6_13".to_vec(),
        12,
        true,
    )
    .ok();
    pallet_assets::Pallet::<T>::force_set_metadata(
        SystemOrigin::Root.into(),
        USDT.into(),
        b"tether".to_vec(),
        b"USDT".to_vec(),
        6,
        true,
    )
    .ok();
    pallet_balances::Pallet::<T>::set_balance(
        SystemOrigin::Root.into(),
        account_id,
        10_000_000_000_000_u128,
        0_u128,
    )
    .unwrap();
    <T as pallet::Config>::Assets::mint_into(USDT, &caller, INITIAL_AMOUNT.into()).unwrap();
    <T as pallet::Config>::Assets::mint_into(KSM, &caller, INITIAL_AMOUNT.into()).unwrap();
    <T as pallet::Config>::Assets::mint_into(SKSM, &caller, INITIAL_AMOUNT.into()).unwrap();
    <T as pallet::Config>::Assets::mint_into(DOT, &caller, INITIAL_AMOUNT.into()).unwrap();
    <T as pallet::Config>::Assets::mint_into(CDOT_6_13, &caller, INITIAL_AMOUNT.into()).unwrap();
    pallet_prices::Pallet::<T>::set_price(SystemOrigin::Root.into(), USDT, 1.into()).unwrap();
    pallet_prices::Pallet::<T>::set_price(SystemOrigin::Root.into(), KSM, 1.into()).unwrap();
    pallet_prices::Pallet::<T>::set_price(SystemOrigin::Root.into(), SKSM, 1.into()).unwrap();
    pallet_prices::Pallet::<T>::set_price(SystemOrigin::Root.into(), DOT, 1.into()).unwrap();
    pallet_prices::Pallet::<T>::set_price(SystemOrigin::Root.into(), CDOT_6_13, 1.into()).unwrap();
}

fn set_account_borrows<T: Config>(
    who: T::AccountId,
    asset_id: AssetIdOf<T>,
    borrow_balance: BalanceOf<T>,
) {
    AccountBorrows::<T>::insert(
        asset_id,
        &who,
        BorrowSnapshot {
            principal: borrow_balance,
            borrow_index: Rate::one(),
        },
    );
    TotalBorrows::<T>::insert(asset_id, borrow_balance);
    T::Assets::burn_from(asset_id, &who, borrow_balance).unwrap();
}

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
    frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

benchmarks! {
    where_clause {
        where
            T: pallet_assets::Config<AssetId = CurrencyId, Balance = Balance> + pallet_prices::Config + pallet_balances::Config<Balance = Balance>
    }

    add_market {
    }: _(SystemOrigin::Root, SKSM, pending_market_mock::<T>(PSKSM))
    verify {
        assert_last_event::<T>(Event::<T>::NewMarket(SKSM, pending_market_mock::<T>(PSKSM)).into());
    }

    activate_market {
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), SKSM, pending_market_mock::<T>(PSKSM)));
    }: _(SystemOrigin::Root, SKSM)
    verify {
        assert_last_event::<T>(Event::<T>::ActivatedMarket(SKSM).into());
    }

    update_rate_model {
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), USDT, pending_market_mock::<T>(PUSDT)));
    }: _(SystemOrigin::Root, USDT, RATE_MODEL_MOCK)
    verify {
        let mut market = pending_market_mock::<T>(PUSDT);
        market.rate_model = RATE_MODEL_MOCK;
        assert_last_event::<T>(Event::<T>::UpdatedMarket(USDT, market).into());
    }

    update_market {
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), KSM, pending_market_mock::<T>(PKSM)));
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
        let mut market = pending_market_mock::<T>(PKSM);
        market.reserve_factor = Ratio::from_percent(50);
        market.close_factor = Ratio::from_percent(15);
        assert_last_event::<T>(Event::<T>::UpdatedMarket(KSM, market).into());
    }

    force_update_market {
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), USDT, pending_market_mock::<T>(PUSDT)));
    }: _(SystemOrigin::Root,USDT, pending_market_mock::<T>(PUSDT))
    verify {
        assert_last_event::<T>(Event::<T>::UpdatedMarket(USDT, pending_market_mock::<T>(PUSDT)).into());
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
        assert_ok!(Loans::<T>::add_reward(SystemOrigin::Signed(caller.clone()).into(), 1_000_000_000_000_u128));
        let receiver = T::Lookup::unlookup(caller.clone());
    }: _(SystemOrigin::Root, receiver, 500_000_000_000_u128)
    verify {
        assert_last_event::<T>(Event::<T>::RewardWithdrawn(caller, 500_000_000_000_u128).into());
    }

    update_market_reward_speed {
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), USDT, pending_market_mock::<T>(USDT)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), USDT));
    }: _(SystemOrigin::Root, USDT, Some(1_000_000), Some(1_000_000))
    verify {
        assert_last_event::<T>(Event::<T>::MarketRewardSpeedUpdated(USDT, 1_000_000, 1_000_000).into());
    }

    claim_reward {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), USDT, pending_market_mock::<T>(USDT)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), USDT));
        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), USDT, 100_000_000));
        assert_ok!(Loans::<T>::add_reward(SystemOrigin::Signed(caller.clone()).into(), 1_000_000_000_000_u128));
        assert_ok!(Loans::<T>::update_market_reward_speed(SystemOrigin::Root.into(), USDT, Some(1_000_000), Some(1_000_000)));
        let target_height = frame_system::Pallet::<T>::block_number().saturating_add(One::one());
        frame_system::Pallet::<T>::set_block_number(target_height);
    }: _(SystemOrigin::Signed(caller.clone()))
    verify {
        assert_last_event::<T>(Event::<T>::RewardPaid(caller, 1_000_000).into());
    }

    claim_reward_for_market {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), USDT, pending_market_mock::<T>(USDT)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), USDT));
        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), USDT, 100_000_000));
        assert_ok!(Loans::<T>::add_reward(SystemOrigin::Signed(caller.clone()).into(), 1_000_000_000_000_u128));
        assert_ok!(Loans::<T>::update_market_reward_speed(SystemOrigin::Root.into(), USDT, Some(1_000_000), Some(1_000_000)));
        let target_height = frame_system::Pallet::<T>::block_number().saturating_add(One::one());
        frame_system::Pallet::<T>::set_block_number(target_height);
    }: _(SystemOrigin::Signed(caller.clone()), USDT)
    verify {
        assert_last_event::<T>(Event::<T>::RewardPaid(caller, 1_000_000).into());
    }


    mint {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), USDT, pending_market_mock::<T>(USDT)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), USDT));
        let amount: u32 = 100_000;
    }: _(SystemOrigin::Signed(caller.clone()), USDT, amount.into())
    verify {
        assert_last_event::<T>(Event::<T>::Deposited(caller, USDT, amount.into()).into());
    }

    borrow {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
        let deposit_amount: u32 = 200_000_000;
        let borrowed_amount: u32 = 100_000_000;
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), USDT, pending_market_mock::<T>(PUSDT)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), USDT));
        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), USDT, deposit_amount.into()));
        assert_ok!(Loans::<T>::collateral_asset(SystemOrigin::Signed(caller.clone()).into(), USDT, true));
    }: _(SystemOrigin::Signed(caller.clone()), USDT, borrowed_amount.into())
    verify {
        assert_last_event::<T>(Event::<T>::Borrowed(caller, USDT, borrowed_amount.into()).into());
    }

    redeem {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
        let deposit_amount: u32 = 100_000_000;
        let redeem_amount: u32 = 100_000;
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), USDT, pending_market_mock::<T>(PUSDT)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), USDT));
        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), USDT, deposit_amount.into()));
    }: _(SystemOrigin::Signed(caller.clone()), USDT, redeem_amount.into())
    verify {
        assert_last_event::<T>(Event::<T>::Redeemed(caller, USDT, redeem_amount.into()).into());
    }

    redeem_all {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
        let deposit_amount: u32 = 100_000_000;
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), USDT, pending_market_mock::<T>(PUSDT)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), USDT));
        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), USDT, deposit_amount.into()));
    }: _(SystemOrigin::Signed(caller.clone()), USDT)
    verify {
        assert_last_event::<T>(Event::<T>::Redeemed(caller, USDT, deposit_amount.into()).into());
    }

    repay_borrow {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
        let deposit_amount: u32 = 200_000_000;
        let borrowed_amount: u32 = 100_000_000;
        let repay_amount: u32 = 100;
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), USDT, pending_market_mock::<T>(PUSDT)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), USDT));
        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), USDT, deposit_amount.into()));
        assert_ok!(Loans::<T>::collateral_asset(SystemOrigin::Signed(caller.clone()).into(), USDT, true));
        assert_ok!(Loans::<T>::borrow(SystemOrigin::Signed(caller.clone()).into(), USDT, borrowed_amount.into()));
    }: _(SystemOrigin::Signed(caller.clone()), USDT, repay_amount.into())
    verify {
        assert_last_event::<T>(Event::<T>::RepaidBorrow(caller, USDT, repay_amount.into()).into());
    }

    repay_borrow_all {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
        let deposit_amount: u32 = 200_000_000;
        let borrowed_amount: u32 = 100_000_000;
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), USDT, pending_market_mock::<T>(PUSDT)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), USDT));
        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), USDT, deposit_amount.into()));
        assert_ok!(Loans::<T>::collateral_asset(SystemOrigin::Signed(caller.clone()).into(), USDT, true));
        assert_ok!(Loans::<T>::borrow(SystemOrigin::Signed(caller.clone()).into(), USDT, borrowed_amount.into()));
    }: _(SystemOrigin::Signed(caller.clone()), USDT)
    verify {
        assert_last_event::<T>(Event::<T>::RepaidBorrow(caller, USDT, borrowed_amount.into()).into());
    }

    collateral_asset {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
        let deposit_amount: u32 = 200_000_000;
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), USDT, pending_market_mock::<T>(PUSDT)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), USDT));
        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), USDT, deposit_amount.into()));
    }: _(SystemOrigin::Signed(caller.clone()), USDT, true)
    verify {
        assert_last_event::<T>(Event::<T>::CollateralAssetAdded(caller, USDT).into());
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
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), SKSM, pending_market_mock::<T>(PSKSM)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), SKSM));
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), KSM, pending_market_mock::<T>(PKSM)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), KSM));
        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(bob.clone()).into(), KSM, deposit_amount.into()));
        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(alice.clone()).into(), SKSM, deposit_amount.into()));
        assert_ok!(Loans::<T>::collateral_asset(SystemOrigin::Signed(alice.clone()).into(), SKSM, true));
        set_account_borrows::<T>(alice.clone(), KSM, borrowed_amount.into());
    }: _(SystemOrigin::Signed(bob.clone()), alice.clone(), KSM, liquidate_amount.into(), SKSM)
    verify {
        assert_last_event::<T>(Event::<T>::LiquidatedBorrow(bob.clone(), alice.clone(), KSM, SKSM, liquidate_amount.into(), incentive_amount.into()).into());
    }

    add_reserves {
        let caller: T::AccountId = whitelisted_caller();
        let payer = T::Lookup::unlookup(caller.clone());
        transfer_initial_balance::<T>(caller.clone());
        let amount: u32 = 2000;
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), USDT, pending_market_mock::<T>(PUSDT)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), USDT));
    }: _(SystemOrigin::Root, payer, USDT, amount.into())
    verify {
        assert_last_event::<T>(Event::<T>::ReservesAdded(caller, USDT, amount.into(), amount.into()).into());
    }

    reduce_reserves {
        let caller: T::AccountId = whitelisted_caller();
        let payer = T::Lookup::unlookup(caller.clone());
        transfer_initial_balance::<T>(caller.clone());
        let add_amount: u32 = 2000;
        let reduce_amount: u32 = 1000;
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), USDT, pending_market_mock::<T>(PUSDT)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), USDT));
        assert_ok!(Loans::<T>::add_reserves(SystemOrigin::Root.into(), payer.clone(), USDT, add_amount.into()));
    }: _(SystemOrigin::Root, payer, USDT, reduce_amount.into())
    verify {
        assert_last_event::<T>(Event::<T>::ReservesReduced(caller, USDT, reduce_amount.into(), (add_amount-reduce_amount).into()).into());
    }

    update_liquidation_free_collateral {

    }: _(SystemOrigin::Root, vec![CDOT_6_13])
    verify {
        assert_last_event::<T>(Event::<T>::LiquidationFreeCollateralsUpdated(vec![CDOT_6_13]).into());
    }
}

impl_benchmark_test_suite!(Loans, crate::mock::new_test_ext(), crate::mock::Test);
