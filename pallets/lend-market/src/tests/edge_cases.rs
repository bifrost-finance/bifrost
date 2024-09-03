use crate::{mock::*, tests::LendMarket, Error};
use frame_support::{assert_err, assert_noop, assert_ok};
use sp_runtime::FixedPointNumber;

#[test]
fn exceeded_supply_cap() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tokens::set_balance(RuntimeOrigin::root(), ALICE, DOT, million_unit(1001), 0,));
		let amount = million_unit(501);
		assert_ok!(LendMarket::mint(RuntimeOrigin::signed(ALICE), DOT, amount));
		// Exceed upper bound.
		assert_err!(
			LendMarket::mint(RuntimeOrigin::signed(ALICE), DOT, amount),
			Error::<Test>::SupplyCapacityExceeded
		);

		LendMarket::redeem(RuntimeOrigin::signed(ALICE), DOT, amount).unwrap();
		// Here should work, cause we redeemed already.
		assert_ok!(LendMarket::mint(RuntimeOrigin::signed(ALICE), DOT, amount));
	})
}

#[test]
fn repay_borrow_all_no_underflow() {
	new_test_ext().execute_with(|| {
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), KSM, vec![KSM]));
		// Alice deposits 200 KSM as collateral
		assert_ok!(LendMarket::mint(RuntimeOrigin::signed(ALICE), KSM, unit(200)));
		assert_ok!(LendMarket::collateral_asset(RuntimeOrigin::signed(ALICE), KSM, true));

		// Alice borrow only 1/1e5 KSM which is hard to accrue total borrows interest in 100 seconds
		assert_ok!(LendMarket::borrow(RuntimeOrigin::signed(ALICE), KSM, 10_u128.pow(7)));

		accrue_interest_per_block(KSM, 100, 9);

		assert_eq!(LendMarket::current_borrow_balance(&ALICE, KSM), Ok(10000005));
		// FIXME since total_borrows is too small and we accrue internal on it every 100 seconds
		// accrue_interest fails every time
		// as you can see the current borrow balance is not equal to total_borrows anymore
		assert_eq!(TotalBorrows::<Test>::get(KSM), 10000000);

		// Alice repay all borrow balance. total_borrows = total_borrows.saturating_sub(10000005) =
		// 0.
		assert_ok!(LendMarket::repay_borrow_all(RuntimeOrigin::signed(ALICE), KSM));

		assert_eq!(<Test as Config>::Assets::balance(KSM, &ALICE), unit(800) - 5);

		assert_eq!(
			ExchangeRate::<Test>::get(DOT)
				.saturating_mul_int(AccountDeposits::<Test>::get(KSM, ALICE).voucher_balance),
			unit(200)
		);

		let borrow_snapshot = AccountBorrows::<Test>::get(KSM, ALICE);
		assert_eq!(borrow_snapshot.principal, 0);
		assert_eq!(borrow_snapshot.borrow_index, BorrowIndex::<Test>::get(KSM));
	})
}

#[test]
fn ensure_capacity_fails_when_market_not_existed() {
	new_test_ext().execute_with(|| {
		assert_err!(
			LendMarket::ensure_under_supply_cap(VDOT, unit(100)),
			Error::<Test>::MarketDoesNotExist
		);
	});
}

#[test]
fn redeem_all_should_be_accurate() {
	new_test_ext().execute_with(|| {
		assert_ok!(LendMarket::add_market_bond(RuntimeOrigin::root(), KSM, vec![KSM]));
		assert_ok!(LendMarket::mint(RuntimeOrigin::signed(ALICE), KSM, unit(200)));
		assert_ok!(LendMarket::collateral_asset(RuntimeOrigin::signed(ALICE), KSM, true));
		assert_ok!(LendMarket::borrow(RuntimeOrigin::signed(ALICE), KSM, unit(50)));

		// let exchange_rate greater than 0.02
		accrue_interest_per_block(KSM, 6, 2);
		assert_eq!(ExchangeRate::<Test>::get(KSM), Rate::from_inner(20000000036387000));

		assert_ok!(LendMarket::repay_borrow_all(RuntimeOrigin::signed(ALICE), KSM));
		// It failed with InsufficientLiquidity before #839
		assert_ok!(LendMarket::redeem_all(RuntimeOrigin::signed(ALICE), KSM));
	})
}

#[test]
fn prevent_the_exchange_rate_attack() {
	new_test_ext().execute_with(|| {
		// Initialize Eve's balance
		assert_ok!(<Test as Config>::Assets::transfer(
			RuntimeOrigin::signed(ALICE),
			EVE,
			DOT.into(),
			unit(200)
		));
		// Eve deposits a small amount
		assert_ok!(LendMarket::mint(RuntimeOrigin::signed(EVE), DOT, 1));
		// !!! Eve transfer a big amount to LendMarket::account_id
		assert_ok!(<Test as Config>::Assets::transfer(
			RuntimeOrigin::signed(EVE),
			LendMarket::account_id(),
			DOT.into(),
			unit(100),
		));
		assert_eq!(<Test as Config>::Assets::balance(DOT, &EVE), 99999999999999);
		assert_eq!(
			<Test as Config>::Assets::balance(DOT, &LendMarket::account_id()),
			100000000000001
		);
		assert_eq!(
			TotalSupply::<Test>::get(DOT),
			1 * 50, // 1 / 0.02
		);
		TimestampPallet::set_timestamp(12000);
		// Eve can not let the exchange rate greater than 1
		assert!(LendMarket::accrue_interest(DOT).is_err());

		// Mock a BIG exchange_rate: 100000000000.02
		ExchangeRate::<Test>::insert(
			DOT,
			Rate::saturating_from_rational(100000000000020u128, 20 * 50),
		);
		// Bob can not deposit 0.1 DOT because the voucher_balance can not be 0.
		assert_noop!(
			LendMarket::mint(RuntimeOrigin::signed(BOB), DOT, 100000000000),
			Error::<Test>::InvalidExchangeRate
		);
	})
}
