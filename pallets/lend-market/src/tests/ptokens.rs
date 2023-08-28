// use crate::{
// 	mock::{
// 		market_mock, new_test_ext, Loans, RuntimeOrigin, Test, ALICE, DAVE, BNC, KSM, VBNC, PKSM,
// 		DOT_U, SDOT, DOT_U,
// 	},
// 	tests::unit,
// 	Error, *,
// };
// use frame_support::{assert_err, assert_noop, assert_ok, traits::tokens::fungibles::Inspect};
// use sp_runtime::{FixedPointNumber, TokenError};

// #[test]
// fn trait_inspect_methods_works() {
// 	new_test_ext().execute_with(|| {
// 		// No Deposits can't not withdraw
// 		assert_err!(Loans::can_withdraw(VBNC, &DAVE, 100).into_result(), TokenError::NoFunds);
// 		assert_eq!(Loans::total_issuance(VBNC), 0);
// 		assert_eq!(Loans::total_issuance(PKSM), 0);

// 		let minimum_balance = Loans::minimum_balance(VBNC);
// 		assert_eq!(minimum_balance, 0);

// 		assert_eq!(Loans::balance(VBNC, &DAVE), 0);

// 		// DAVE Deposit 100 BNC
// 		assert_ok!(Loans::mint(RuntimeOrigin::signed(DAVE), BNC, unit(100)));
// 		assert_eq!(Loans::balance(VBNC, &DAVE), unit(100) * 50);

// 		assert_eq!(
// 			Loans::reducible_balance(VBNC, &DAVE, Preservation::Expendable, Fortitude::Polite),
// 			unit(100) * 50
// 		);
// 		assert_ok!(Loans::collateral_asset(RuntimeOrigin::signed(DAVE), BNC, true));
// 		// Borrow 25 BNC will reduce 25 BNC liquidity for collateral_factor is 50%
// 		assert_ok!(Loans::borrow(RuntimeOrigin::signed(DAVE), BNC, unit(25)));

// 		assert_eq!(
// 			Loans::exchange_rate(BNC)
// 				.saturating_mul_int(Loans::account_deposits(BNC, DAVE).voucher_balance),
// 			unit(100)
// 		);

// 		// DAVE Deposit 100 BNC, Borrow 25 BNC
// 		// Liquidity BNC 25
// 		// Formula: ptokens = liquidity / price(1) / collateral(0.5) / exchange_rate(0.02)
// 		assert_eq!(
// 			Loans::reducible_balance(VBNC, &DAVE, Preservation::Expendable, Fortitude::Polite),
// 			unit(25) * 2 * 50
// 		);

// 		// Multi-asset case, additional deposit DOT_U
// 		// DAVE Deposit 100 BNC, 50 DOT_U, Borrow 25 BNC
// 		// Liquidity BNC = 25, DOT_U = 25
// 		// ptokens = dollar(25 + 25) / 1 / 0.5 / 0.02 = dollar(50) * 100
// 		assert_ok!(Loans::mint(RuntimeOrigin::signed(DAVE), DOT_U, unit(50)));
// 		assert_eq!(Loans::balance(DOT_U, &DAVE), unit(50) * 50);
// 		assert_eq!(
// 			Loans::reducible_balance(DOT_U, &DAVE, Preservation::Expendable, Fortitude::Polite),
// 			unit(25) * 2 * 50
// 		);
// 		// enable DOT_U collateral
// 		assert_ok!(Loans::collateral_asset(RuntimeOrigin::signed(DAVE), DOT_U, true));
// 		assert_eq!(
// 			Loans::reducible_balance(VBNC, &DAVE, Preservation::Expendable, Fortitude::Polite),
// 			unit(25 + 25) * 2 * 50
// 		);

// 		assert_ok!(Loans::borrow(RuntimeOrigin::signed(DAVE), BNC, unit(50)));
// 		assert_eq!(
// 			Loans::reducible_balance(VBNC, &DAVE, Preservation::Expendable, Fortitude::Polite),
// 			0
// 		);

// 		assert_eq!(Loans::total_issuance(VBNC), unit(100) * 50);
// 		assert_ok!(Loans::can_deposit(VBNC, &DAVE, 100, true).into_result());
// 		assert_ok!(Loans::can_withdraw(VBNC, &DAVE, 1000).into_result());
// 	})
// }

// #[test]
// fn ptoken_unique_works() {
// 	new_test_ext().execute_with(|| {
// 		// ptoken_id already exists in `UnderlyingAssetId`
// 		assert_noop!(
// 			Loans::add_market(RuntimeOrigin::root(), SDOT, market_mock(VBNC)),
// 			Error::<Test>::InvalidPtokenId
// 		);

// 		// ptoken_id cannot as the same as the asset id in `Markets`
// 		assert_noop!(
// 			Loans::add_market(RuntimeOrigin::root(), SDOT, market_mock(KSM)),
// 			Error::<Test>::InvalidPtokenId
// 		);
// 	})
// }

// #[test]
// fn transfer_ptoken_works() {
// 	new_test_ext().execute_with(|| {
// 		// DAVE Deposit 100 BNC
// 		assert_ok!(Loans::mint(RuntimeOrigin::signed(DAVE), BNC, unit(100)));

// 		// DAVE BNC collateral: deposit = 100
// 		// BNC: cash - deposit = 1000 - 100 = 900
// 		assert_eq!(
// 			Loans::exchange_rate(BNC)
// 				.saturating_mul_int(Loans::account_deposits(BNC, DAVE).voucher_balance),
// 			unit(100)
// 		);

// 		// ALICE BNC collateral: deposit = 0
// 		assert_eq!(
// 			Loans::exchange_rate(BNC)
// 				.saturating_mul_int(Loans::account_deposits(BNC, ALICE).voucher_balance),
// 			unit(0)
// 		);

// 		// Transfer ptokens from DAVE to ALICE
// 		Loans::transfer(VBNC, &DAVE, &ALICE, unit(50) * 50).unwrap();
// 		// Loans::transfer_ptokens(RuntimeOrigin::signed(DAVE), ALICE, BNC, dollar(50) *
// 		// 50).unwrap();

// 		// DAVE BNC collateral: deposit = 50
// 		assert_eq!(
// 			Loans::exchange_rate(BNC)
// 				.saturating_mul_int(Loans::account_deposits(BNC, DAVE).voucher_balance),
// 			unit(50)
// 		);
// 		// DAVE Redeem 51 BNC should cause InsufficientDeposit
// 		assert_noop!(
// 			Loans::redeem_allowed(BNC, &DAVE, unit(51) * 50),
// 			Error::<Test>::InsufficientDeposit
// 		);

// 		// ALICE BNC collateral: deposit = 50
// 		assert_eq!(
// 			Loans::exchange_rate(BNC)
// 				.saturating_mul_int(Loans::account_deposits(BNC, ALICE).voucher_balance),
// 			unit(50)
// 		);
// 		// ALICE Redeem 50 BNC should be succeeded
// 		assert_ok!(Loans::redeem_allowed(BNC, &ALICE, unit(50) * 50));
// 	})
// }

// #[test]
// fn transfer_ptokens_under_collateral_works() {
// 	new_test_ext().execute_with(|| {
// 		// DAVE Deposit 100 BNC
// 		assert_ok!(Loans::mint(RuntimeOrigin::signed(DAVE), BNC, unit(100)));
// 		assert_ok!(Loans::collateral_asset(RuntimeOrigin::signed(DAVE), BNC, true));

// 		// Borrow 50 BNC will reduce 50 BNC liquidity for collateral_factor is 50%
// 		assert_ok!(Loans::borrow(RuntimeOrigin::signed(DAVE), BNC, unit(50)));
// 		// Repay 40 BNC
// 		assert_ok!(Loans::repay_borrow(RuntimeOrigin::signed(DAVE), BNC, unit(40)));

// 		// Transfer 20 ptokens from DAVE to ALICE
// 		Loans::transfer(VBNC, &DAVE, &ALICE, unit(20) * 50).unwrap();

// 		// DAVE Deposit BNC = 100 - 20 = 80
// 		// DAVE Borrow BNC = 0 + 50 - 40 = 10
// 		// DAVE liquidity BNC = 80 * 0.5 - 10 = 30
// 		assert_eq!(
// 			Loans::exchange_rate(BNC)
// 				.saturating_mul_int(Loans::account_deposits(BNC, DAVE).voucher_balance),
// 			unit(80)
// 		);
// 		// DAVE Borrow 31 BNC should cause InsufficientLiquidity
// 		assert_noop!(
// 			Loans::borrow(RuntimeOrigin::signed(DAVE), BNC, unit(31)),
// 			Error::<Test>::InsufficientLiquidity
// 		);
// 		assert_ok!(Loans::borrow(RuntimeOrigin::signed(DAVE), BNC, unit(30)));

// 		// Assert ALICE Supply BNC 20
// 		assert_eq!(
// 			Loans::exchange_rate(BNC)
// 				.saturating_mul_int(Loans::account_deposits(BNC, ALICE).voucher_balance),
// 			unit(20)
// 		);
// 		// ALICE Redeem 20 BNC should be succeeded
// 		// Also means that transfer ptoken succeed
// 		assert_ok!(Loans::redeem_allowed(BNC, &ALICE, unit(20) * 50,));
// 	})
// }
