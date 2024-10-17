use crate::{
	mock::{
		new_test_ext, LendMarket, MockOraclePriceProvider, RuntimeOrigin, ALICE, BOB, DOT, DOT_U,
		KSM, *,
	},
	tests::unit,
	Error, MarketState,
};
use bifrost_primitives::Rate;
use frame_support::{assert_err, assert_noop, assert_ok};
use sp_runtime::FixedPointNumber;

#[test]
fn liquidate_borrow_allowed_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), KSM, vec![DOT, BNC, KSM]));
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), DOT, vec![DOT, BNC, KSM]));
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), BNC, vec![DOT, BNC, KSM])); // Borrower should have a positive shortfall
		let dot_market = LendMarket::market(DOT).unwrap();
		assert_noop!(
			LendMarket::liquidate_borrow_allowed(&ALICE, DOT, 100, &dot_market),
			Error::<Test>::InsufficientShortfall
		);
		initial_setup();
		alice_borrows_100_ksm();
		// Adjust KSM price to make shortfall
		MockOraclePriceProvider::set_price(KSM, 2.into());
		let ksm_market = LendMarket::market(KSM).unwrap();
		// Here the balance sheet of Alice is:
		// Collateral   LendMarket
		// CDOT $200    KSM $400
		// DOT_U $200
		assert_noop!(
			LendMarket::liquidate_borrow_allowed(&ALICE, KSM, unit(51), &ksm_market),
			Error::<Test>::TooMuchRepay
		);
		assert_ok!(LendMarket::liquidate_borrow_allowed(&ALICE, KSM, unit(50), &ksm_market));
	})
}

#[test]
fn lf_liquidate_borrow_fails_due_to_lf_collateral() {
	new_test_ext().execute_with(|| {
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), KSM, vec![DOT, BNC, KSM]));
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), DOT, vec![DOT, BNC, KSM]));
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), BNC, vec![DOT, BNC, KSM]));
		LendMarket::update_liquidation_free_collateral(RuntimeOrigin::root(), vec![PHA]).unwrap();

		assert_err!(
			LendMarket::liquidate_borrow(RuntimeOrigin::signed(ALICE), BOB, DOT, unit(100), PHA),
			Error::<Test>::CollateralReserved
		);
		assert_err!(
			LendMarket::liquidate_borrow(RuntimeOrigin::signed(ALICE), BOB, DOT, unit(100), DOT_U),
			Error::<Test>::InsufficientShortfall
		);
	})
}

#[test]
fn lf_liquidate_borrow_allowed_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), KSM, vec![DOT, BNC, KSM]));
		assert_ok!(LendMarket::add_market_bond(
			RuntimeOrigin::root(),
			DOT,
			vec![DOT, BNC, KSM, DOT_U, PHA]
		));
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), BNC, vec![DOT, BNC, KSM]));
		LendMarket::update_liquidation_free_collateral(RuntimeOrigin::root(), vec![PHA]).unwrap();
		// Bob deposits $200 DOT
		LendMarket::mint(RuntimeOrigin::signed(BOB), DOT, unit(200)).unwrap();
		LendMarket::mint(RuntimeOrigin::signed(ALICE), DOT_U, unit(200)).unwrap();
		LendMarket::mint(RuntimeOrigin::signed(ALICE), PHA, unit(200)).unwrap();
		LendMarket::collateral_asset(RuntimeOrigin::signed(ALICE), DOT_U, true).unwrap();
		LendMarket::collateral_asset(RuntimeOrigin::signed(ALICE), PHA, true).unwrap();

		// ALICE
		// Collateral                 Borrowed
		// DOT_U  $100                 DOT $200
		// CDOT  $100
		LendMarket::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(200)).unwrap();

		// CDOT's price is highly relative to DOT's price in real runtime. Thus we must update them
		// at the same time.
		MockOraclePriceProvider::set_price(DOT, 2.into());
		MockOraclePriceProvider::set_price(PHA, 2.into());
		// ALICE
		// Collateral                 Borrowed
		// DOT_U  $100                 DOT $400
		// CDOT  $200

		let dot_market = LendMarket::market(DOT).unwrap();
		// The max repay amount = (400 - 200) * 50% = $100
		assert_err!(
			LendMarket::liquidate_borrow_allowed(&ALICE, DOT, unit(51), &dot_market),
			Error::<Test>::TooMuchRepay
		);
		assert_ok!(LendMarket::liquidate_borrow_allowed(&ALICE, DOT, unit(50), &dot_market));

		// Remove CDOT from lf collateral
		LendMarket::update_liquidation_free_collateral(RuntimeOrigin::root(), vec![]).unwrap();
		// The max repay amount = 400 * 50 = $200
		assert_ok!(LendMarket::liquidate_borrow_allowed(&ALICE, DOT, unit(100), &dot_market));
	})
}

#[test]
fn deposit_of_borrower_must_be_collateral() {
	new_test_ext().execute_with(|| {
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), KSM, vec![DOT, BNC, KSM]));
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), DOT, vec![DOT, BNC, KSM]));
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), BNC, vec![DOT, BNC, KSM]));
		initial_setup();
		alice_borrows_100_ksm();
		// Adjust KSM price to make shortfall
		MockOraclePriceProvider::set_price(KSM, 2.into());
		let market = LendMarket::market(KSM).unwrap();
		assert_noop!(
			LendMarket::liquidate_borrow_allowed(&ALICE, KSM, unit(51), &market),
			Error::<Test>::TooMuchRepay
		);
		assert_noop!(
			LendMarket::liquidate_borrow(RuntimeOrigin::signed(BOB), ALICE, KSM, 10, DOT),
			Error::<Test>::DepositsAreNotCollateral
		);
	})
}

#[test]
fn collateral_value_must_be_greater_than_liquidation_value() {
	new_test_ext().execute_with(|| {
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), KSM, vec![DOT, BNC, KSM]));
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), DOT, vec![DOT, BNC, KSM]));
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), BNC, vec![DOT, BNC, KSM]));
		initial_setup();
		alice_borrows_100_ksm();
		MockOraclePriceProvider::set_price(KSM, Rate::from_float(2000.0));
		LendMarket::mutate_market(KSM, |market| {
			market.liquidate_incentive = Rate::from_float(200.0);
			market.clone()
		})
		.unwrap();
		assert_noop!(
			LendMarket::liquidate_borrow(RuntimeOrigin::signed(BOB), ALICE, KSM, unit(50), DOT_U),
			Error::<Test>::InsufficientCollateral
		);
	})
}

#[test]
fn full_workflow_works_as_expected() {
	new_test_ext().execute_with(|| {
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), KSM, vec![DOT, BNC, KSM]));
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), DOT, vec![DOT, BNC, KSM]));
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), BNC, vec![DOT, BNC, KSM]));
		initial_setup();
		alice_borrows_100_ksm();
		// adjust KSM price to make ALICE generate shortfall
		MockOraclePriceProvider::set_price(KSM, 2.into());
		// BOB repay the KSM borrow balance and get DOT from ALICE
		assert_ok!(LendMarket::liquidate_borrow(
			RuntimeOrigin::signed(BOB),
			ALICE,
			KSM,
			unit(50),
			DOT_U
		));

		// KSM price = 2
		// incentive = repay KSM value * 1.1 = (50 * 2) * 1.1 = 110
		// Alice DOT_U: cash - deposit = 1000 - 200 = 800
		// Alice DOT_U collateral: deposit - incentive = 200 - 110 = 90
		// Alice KSM: cash + borrow = 1000 + 100 = 1100
		// Alice KSM borrow balance: origin borrow balance - liquidate amount = 100 - 50 = 50
		// Bob KSM: cash - deposit - repay = 1000 - 200 - 50 = 750
		// Bob DOT collateral: incentive = 110-(110/1.1*0.03)=107
		assert_eq!(<Test as Config>::Assets::balance(DOT_U, &ALICE), unit(800),);
		assert_eq!(
			ExchangeRate::<Test>::get(DOT_U)
				.saturating_mul_int(AccountDeposits::<Test>::get(DOT_U, ALICE).voucher_balance),
			unit(90),
		);
		assert_eq!(<Test as Config>::Assets::balance(KSM, &ALICE), unit(1100),);
		assert_eq!(AccountBorrows::<Test>::get(KSM, ALICE).principal, unit(50));
		assert_eq!(<Test as Config>::Assets::balance(KSM, &BOB), unit(750));
		assert_eq!(
			ExchangeRate::<Test>::get(DOT_U)
				.saturating_mul_int(AccountDeposits::<Test>::get(DOT_U, BOB).voucher_balance),
			unit(107),
		);
		// 3 dollar reserved in our incentive reward account
		let incentive_reward_account = LendMarket::incentive_reward_account_id().unwrap();
		println!("incentive reserve account:{:?}", incentive_reward_account.clone());
		assert_eq!(
			ExchangeRate::<Test>::get(DOT_U).saturating_mul_int(
				AccountDeposits::<Test>::get(DOT_U, incentive_reward_account.clone())
					.voucher_balance
			),
			unit(3),
		);
		assert_eq!(<Test as Config>::Assets::balance(DOT_U, &ALICE), unit(800),);
		// reduce 2 dollar from incentive reserve to alice account
		assert_ok!(LendMarket::reduce_incentive_reserves(
			RuntimeOrigin::root(),
			ALICE,
			DOT_U,
			unit(2),
		));
		// still 1 dollar left in reserve account
		assert_eq!(
			ExchangeRate::<Test>::get(DOT_U).saturating_mul_int(
				AccountDeposits::<Test>::get(DOT_U, incentive_reward_account).voucher_balance
			),
			unit(1),
		);
		// 2 dollar transfer to alice
		assert_eq!(<Test as Config>::Assets::balance(DOT_U, &ALICE), unit(800) + unit(2),);
	})
}

#[test]
fn liquidator_cannot_take_inactive_market_currency() {
	new_test_ext().execute_with(|| {
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), KSM, vec![DOT, BNC, KSM]));
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), DOT, vec![DOT, BNC, KSM]));
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), BNC, vec![DOT, BNC, KSM]));
		initial_setup();
		alice_borrows_100_ksm();
		// Adjust KSM price to make shortfall
		MockOraclePriceProvider::set_price(KSM, 2.into());
		assert_ok!(LendMarket::mutate_market(DOT, |stored_market| {
			stored_market.state = MarketState::Supervision;
			stored_market.clone()
		}));
		assert_noop!(
			LendMarket::liquidate_borrow(RuntimeOrigin::signed(BOB), ALICE, KSM, unit(50), DOT),
			Error::<Test>::MarketNotActivated
		);
	})
}

#[test]
fn liquidator_can_not_repay_more_than_the_close_factor_pct_multiplier() {
	new_test_ext().execute_with(|| {
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), KSM, vec![DOT, BNC, KSM]));
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), DOT, vec![DOT, BNC, KSM]));
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), BNC, vec![DOT, BNC, KSM]));
		initial_setup();
		alice_borrows_100_ksm();
		MockOraclePriceProvider::set_price(KSM, 20.into());
		assert_noop!(
			LendMarket::liquidate_borrow(RuntimeOrigin::signed(BOB), ALICE, KSM, unit(51), DOT),
			Error::<Test>::TooMuchRepay
		);
	})
}

#[test]
fn liquidator_must_not_be_borrower() {
	new_test_ext().execute_with(|| {
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), KSM, vec![DOT, BNC, KSM]));
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), DOT, vec![DOT, BNC, KSM]));
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), BNC, vec![DOT, BNC, KSM]));
		initial_setup();
		assert_noop!(
			LendMarket::liquidate_borrow(RuntimeOrigin::signed(ALICE), ALICE, KSM, 0, DOT),
			Error::<Test>::LiquidatorIsBorrower
		);
	})
}

fn alice_borrows_100_ksm() {
	assert_ok!(LendMarket::borrow(RuntimeOrigin::signed(ALICE), KSM, unit(100)));
}

fn initial_setup() {
	assert_ok!(LendMarket::add_market_bond(
		RuntimeOrigin::root(),
		KSM,
		vec![DOT, BNC, KSM, DOT_U, PHA]
	));
	assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), DOT, vec![DOT, BNC, DOT_U, PHA]));
	assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), BNC, vec![DOT, BNC, DOT_U, PHA]));
	// Bob deposits 200 KSM
	assert_ok!(LendMarket::mint(RuntimeOrigin::signed(BOB), KSM, unit(200)));
	// Alice deposits 200 DOT as collateral
	assert_ok!(LendMarket::mint(RuntimeOrigin::signed(ALICE), DOT_U, unit(200)));
	assert_ok!(LendMarket::collateral_asset(RuntimeOrigin::signed(ALICE), DOT_U, true));
	assert_ok!(LendMarket::mint(RuntimeOrigin::signed(ALICE), PHA, unit(200)));
	assert_ok!(LendMarket::collateral_asset(RuntimeOrigin::signed(ALICE), PHA, true));
	assert_ok!(LendMarket::update_liquidation_free_collateral(RuntimeOrigin::root(), vec![PHA]));
}
