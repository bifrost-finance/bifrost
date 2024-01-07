use crate::{
	mock::{
		market_mock, new_test_ext, LendMarket, RuntimeOrigin, Test, ACTIVE_MARKET_MOCK, ALICE, DOT,
		LDOT, LUSDT, MARKET_MOCK, VDOT,
	},
	Error, InterestRateModel, MarketState,
};
use bifrost_primitives::{Rate, Ratio};
use frame_support::{assert_noop, assert_ok, error::BadOrigin};
use sp_runtime::{traits::Zero, FixedPointNumber};

macro_rules! rate_model_sanity_check {
	($call:ident) => {
		new_test_ext().execute_with(|| {
			// Invalid base_rate
			assert_noop!(
				LendMarket::$call(RuntimeOrigin::root(), VDOT, {
					let mut market = MARKET_MOCK;
					market.rate_model = InterestRateModel::new_jump_model(
						Rate::saturating_from_rational(36, 100),
						Rate::saturating_from_rational(15, 100),
						Rate::saturating_from_rational(35, 100),
						Ratio::from_percent(80),
					);
					market
				}),
				Error::<Test>::InvalidRateModelParam
			);
			// Invalid jump_rate
			assert_noop!(
				LendMarket::$call(RuntimeOrigin::root(), VDOT, {
					let mut market = MARKET_MOCK;
					market.rate_model = InterestRateModel::new_jump_model(
						Rate::saturating_from_rational(5, 100),
						Rate::saturating_from_rational(36, 100),
						Rate::saturating_from_rational(37, 100),
						Ratio::from_percent(80),
					);
					market
				}),
				Error::<Test>::InvalidRateModelParam
			);
			// Invalid full_rate
			assert_noop!(
				LendMarket::$call(RuntimeOrigin::root(), VDOT, {
					let mut market = MARKET_MOCK;
					market.rate_model = InterestRateModel::new_jump_model(
						Rate::saturating_from_rational(5, 100),
						Rate::saturating_from_rational(15, 100),
						Rate::saturating_from_rational(57, 100),
						Ratio::from_percent(80),
					);
					market
				}),
				Error::<Test>::InvalidRateModelParam
			);
			// base_rate greater than jump_rate
			assert_noop!(
				LendMarket::$call(RuntimeOrigin::root(), VDOT, {
					let mut market = MARKET_MOCK;
					market.rate_model = InterestRateModel::new_jump_model(
						Rate::saturating_from_rational(10, 100),
						Rate::saturating_from_rational(9, 100),
						Rate::saturating_from_rational(14, 100),
						Ratio::from_percent(80),
					);
					market
				}),
				Error::<Test>::InvalidRateModelParam
			);
			// jump_rate greater than full_rate
			assert_noop!(
				LendMarket::$call(RuntimeOrigin::root(), VDOT, {
					let mut market = MARKET_MOCK;
					market.rate_model = InterestRateModel::new_jump_model(
						Rate::saturating_from_rational(5, 100),
						Rate::saturating_from_rational(15, 100),
						Rate::saturating_from_rational(14, 100),
						Ratio::from_percent(80),
					);
					market
				}),
				Error::<Test>::InvalidRateModelParam
			);
		})
	};
}

#[test]
fn active_market_sets_state_to_active() {
	new_test_ext().execute_with(|| {
		LendMarket::add_market(RuntimeOrigin::root(), VDOT, MARKET_MOCK).unwrap();
		assert_eq!(
			LendMarket::market(VDOT).unwrap().state,
			MarketState::Pending
		);
		LendMarket::activate_market(RuntimeOrigin::root(), VDOT).unwrap();
		assert_eq!(LendMarket::market(VDOT).unwrap().state, MarketState::Active);
	})
}

#[test]
fn active_market_does_not_modify_unknown_market_currencies() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			LendMarket::activate_market(RuntimeOrigin::root(), VDOT),
			Error::<Test>::MarketDoesNotExist
		);
	})
}

#[test]
fn add_market_can_only_be_used_by_root() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			LendMarket::add_market(RuntimeOrigin::signed(ALICE), DOT, MARKET_MOCK),
			BadOrigin
		);
	})
}

#[test]
fn add_market_ensures_that_market_state_must_be_pending() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			LendMarket::add_market(RuntimeOrigin::root(), VDOT, ACTIVE_MARKET_MOCK),
			Error::<Test>::NewMarketMustHavePendingState
		);
	})
}

#[test]
fn add_market_has_sanity_checks_for_rate_models() {
	rate_model_sanity_check!(add_market);
}

#[test]
fn add_market_successfully_stores_a_new_market() {
	new_test_ext().execute_with(|| {
		LendMarket::add_market(RuntimeOrigin::root(), VDOT, MARKET_MOCK).unwrap();
		assert_eq!(LendMarket::market(VDOT).unwrap(), MARKET_MOCK);
	})
}

#[test]
fn add_market_ensures_that_market_does_not_exist() {
	new_test_ext().execute_with(|| {
		assert_ok!(LendMarket::add_market(
			RuntimeOrigin::root(),
			VDOT,
			MARKET_MOCK
		));
		assert_noop!(
			LendMarket::add_market(RuntimeOrigin::root(), VDOT, MARKET_MOCK),
			Error::<Test>::MarketAlreadyExists
		);
	})
}

#[test]
fn force_update_market_can_only_be_used_by_root() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			LendMarket::force_update_market(RuntimeOrigin::signed(ALICE), DOT, MARKET_MOCK),
			BadOrigin
		);
	})
}

#[test]
fn force_update_market_works() {
	new_test_ext().execute_with(|| {
		let mut new_market = market_mock(LDOT);
		new_market.state = MarketState::Active;
		LendMarket::force_update_market(RuntimeOrigin::root(), DOT, new_market).unwrap();
		assert_eq!(LendMarket::market(DOT).unwrap().state, MarketState::Active);
		assert_eq!(LendMarket::market(DOT).unwrap().lend_token_id, LDOT);

		// New lend_token_id must not be in use
		assert_noop!(
			LendMarket::force_update_market(RuntimeOrigin::root(), DOT, market_mock(LUSDT)),
			Error::<Test>::InvalidPtokenId
		);
		assert_ok!(LendMarket::force_update_market(
			RuntimeOrigin::root(),
			DOT,
			market_mock(LDOT)
		));
		assert_eq!(LendMarket::market(DOT).unwrap().lend_token_id, LDOT);
	})
}

#[test]
fn force_update_market_ensures_that_it_is_not_possible_to_modify_unknown_market_currencies() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			LendMarket::force_update_market(RuntimeOrigin::root(), LDOT, MARKET_MOCK),
			Error::<Test>::MarketDoesNotExist
		);
	})
}

#[test]
fn update_market_has_sanity_checks_for_rate_models() {
	rate_model_sanity_check!(force_update_market);
}

#[test]
fn update_market_ensures_that_it_is_not_possible_to_modify_unknown_market_currencies() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			LendMarket::update_market(
				RuntimeOrigin::root(),
				VDOT,
				None,
				None,
				None,
				None,
				None,
				None,
				None,
				None,
			),
			Error::<Test>::MarketDoesNotExist
		);
	})
}

#[test]
fn update_market_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(
			LendMarket::market(DOT).unwrap().close_factor,
			Ratio::from_percent(50)
		);

		let market = MARKET_MOCK;
		assert_ok!(LendMarket::update_market(
			RuntimeOrigin::root(),
			DOT,
			None,
			None,
			None,
			Some(Default::default()),
			None,
			None,
			None,
			None,
		));

		assert_eq!(
			LendMarket::market(DOT).unwrap().close_factor,
			Default::default()
		);
		assert_eq!(
			LendMarket::market(DOT).unwrap().supply_cap,
			market.supply_cap
		);
	})
}

#[test]
fn update_market_should_not_work_if_with_invalid_params() {
	new_test_ext().execute_with(|| {
		assert_eq!(
			LendMarket::market(DOT).unwrap().close_factor,
			Ratio::from_percent(50)
		);

		// check error code while collateral_factor is [0%, 100%)
		assert_ok!(LendMarket::update_market(
			RuntimeOrigin::root(),
			DOT,
			Some(Ratio::zero()),
			None,
			None,
			Some(Default::default()),
			None,
			None,
			None,
			None,
		));
		assert_noop!(
			LendMarket::update_market(
				RuntimeOrigin::root(),
				DOT,
				Some(Ratio::one()),
				None,
				None,
				Some(Default::default()),
				None,
				None,
				None,
				None,
			),
			Error::<Test>::InvalidFactor
		);
		// check error code while reserve_factor is 0% or bigger than 100%
		assert_noop!(
			LendMarket::update_market(
				RuntimeOrigin::root(),
				DOT,
				None,
				None,
				Some(Ratio::zero()),
				Some(Default::default()),
				None,
				None,
				None,
				None,
			),
			Error::<Test>::InvalidFactor
		);
		assert_noop!(
			LendMarket::update_market(
				RuntimeOrigin::root(),
				DOT,
				None,
				None,
				Some(Ratio::one()),
				Some(Default::default()),
				None,
				None,
				None,
				None,
			),
			Error::<Test>::InvalidFactor
		);
		// check error code while cap is zero
		assert_noop!(
			LendMarket::update_market(
				RuntimeOrigin::root(),
				DOT,
				None,
				None,
				None,
				Some(Default::default()),
				None,
				Some(Rate::from_inner(Rate::DIV / 100 * 90)),
				Some(Zero::zero()),
				None,
			),
			Error::<Test>::InvalidSupplyCap
		);
	})
}

#[test]
fn update_rate_model_works() {
	new_test_ext().execute_with(|| {
		let new_rate_model = InterestRateModel::new_jump_model(
			Rate::saturating_from_rational(6, 100),
			Rate::saturating_from_rational(15, 100),
			Rate::saturating_from_rational(35, 100),
			Ratio::from_percent(80),
		);
		assert_ok!(LendMarket::update_rate_model(
			RuntimeOrigin::root(),
			DOT,
			new_rate_model,
		));
		assert_eq!(LendMarket::market(DOT).unwrap().rate_model, new_rate_model);

		// Invalid base_rate
		assert_noop!(
			LendMarket::update_rate_model(
				RuntimeOrigin::root(),
				VDOT,
				InterestRateModel::new_jump_model(
					Rate::saturating_from_rational(36, 100),
					Rate::saturating_from_rational(15, 100),
					Rate::saturating_from_rational(35, 100),
					Ratio::from_percent(80),
				)
			),
			Error::<Test>::InvalidRateModelParam
		);
		// Invalid jump_rate
		assert_noop!(
			LendMarket::update_rate_model(
				RuntimeOrigin::root(),
				VDOT,
				InterestRateModel::new_jump_model(
					Rate::saturating_from_rational(5, 100),
					Rate::saturating_from_rational(36, 100),
					Rate::saturating_from_rational(37, 100),
					Ratio::from_percent(80),
				)
			),
			Error::<Test>::InvalidRateModelParam
		);
		// Invalid full_rate
		assert_noop!(
			LendMarket::update_rate_model(
				RuntimeOrigin::root(),
				VDOT,
				InterestRateModel::new_jump_model(
					Rate::saturating_from_rational(5, 100),
					Rate::saturating_from_rational(15, 100),
					Rate::saturating_from_rational(57, 100),
					Ratio::from_percent(80),
				)
			),
			Error::<Test>::InvalidRateModelParam
		);
		// base_rate greater than jump_rate
		assert_noop!(
			LendMarket::update_rate_model(
				RuntimeOrigin::root(),
				VDOT,
				InterestRateModel::new_jump_model(
					Rate::saturating_from_rational(10, 100),
					Rate::saturating_from_rational(9, 100),
					Rate::saturating_from_rational(14, 100),
					Ratio::from_percent(80),
				)
			),
			Error::<Test>::InvalidRateModelParam
		);
		// jump_rate greater than full_rate
		assert_noop!(
			LendMarket::update_rate_model(
				RuntimeOrigin::root(),
				VDOT,
				InterestRateModel::new_jump_model(
					Rate::saturating_from_rational(5, 100),
					Rate::saturating_from_rational(15, 100),
					Rate::saturating_from_rational(14, 100),
					Ratio::from_percent(80),
				)
			),
			Error::<Test>::InvalidRateModelParam
		);
	})
}
