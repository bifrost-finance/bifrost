use crate::{mock::*, OracleKey};
use frame_support::{assert_err, assert_ok, dispatch::DispatchError, BoundedVec};
use mocktopus::mocking::*;
use sp_arithmetic::FixedU128;
use sp_runtime::FixedPointNumber;

type Event = crate::Event<Test>;

// use macro to avoid messing up stack trace
macro_rules! assert_emitted {
	($event:expr) => {
		let test_event = TestEvent::Oracle($event);
		assert!(System::events().iter().any(|a| a.event == test_event));
	};
}

macro_rules! assert_not_emitted {
	($event:expr) => {
		let test_event = TestEvent::Oracle($event);
		assert!(!System::events().iter().any(|a| a.event == test_event));
	};
}

fn mine_block() {
	crate::Pallet::<Test>::begin_block(0);
}

#[test]
fn feed_values_succeeds() {
	run_test(|| {
		let key = OracleKey::ExchangeRate(Token(DOT));
		let rate = FixedU128::checked_from_rational(100, 1).unwrap();

		Oracle::is_authorized.mock_safe(|_| MockResult::Return(true));
		let result = Oracle::feed_values(RuntimeOrigin::signed(3), vec![(key.clone(), rate)]);
		assert_ok!(result);

		mine_block();

		let exchange_rate = Oracle::get_price(key.clone()).unwrap();
		assert_eq!(exchange_rate, rate);

		assert_emitted!(Event::FeedValues { oracle_id: 3, values: vec![(key.clone(), rate)] });
	});
}

#[test]
fn feed_values_fails_with_invalid_oracle_source() {
	run_test(|| {
		let key = OracleKey::ExchangeRate(Token(DOT));
		let successful_rate = FixedU128::checked_from_rational(20, 1).unwrap();
		let failed_rate = FixedU128::checked_from_rational(100, 1).unwrap();

		Oracle::is_authorized.mock_safe(|_| MockResult::Return(true));
		assert_ok!(Oracle::feed_values(
			RuntimeOrigin::signed(4),
			vec![(key.clone(), successful_rate)]
		));

		mine_block();

		Oracle::is_authorized.mock_safe(|_| MockResult::Return(false));
		assert_err!(
			Oracle::feed_values(RuntimeOrigin::signed(3), vec![(key.clone(), failed_rate)]),
			TestError::InvalidOracleSource
		);

		mine_block();

		let exchange_rate = Oracle::get_price(key.clone()).unwrap();
		assert_eq!(exchange_rate, successful_rate);

		assert_not_emitted!(Event::FeedValues {
			oracle_id: 3,
			values: vec![(key.clone(), failed_rate)]
		});
		assert_not_emitted!(Event::FeedValues {
			oracle_id: 4,
			values: vec![(key.clone(), failed_rate)]
		});
	});
}

// #[test]
// fn getting_exchange_rate_fails_with_missing_exchange_rate() {
// 	run_test(|| {
// 		let key = OracleKey::ExchangeRate(Token(DOT));
// 		assert_err!(Oracle::get_price(key), TestError::MissingExchangeRate);
// 		assert_err!(Oracle::wrapped_to_collateral(0, Token(DOT)), TestError::MissingExchangeRate);
// 		assert_err!(Oracle::collateral_to_wrapped(0, Token(DOT)), TestError::MissingExchangeRate);
// 	});
// }

// #[test]
// fn wrapped_to_collateral() {
// 	run_test(|| {
// 		Oracle::get_price
// 			.mock_safe(|_| MockResult::Return(Ok(FixedU128::checked_from_rational(2, 1).unwrap())));
// 		let test_cases = [(0, 0), (2, 4), (10, 20)];
// 		for (input, expected) in test_cases.iter() {
// 			let result = Oracle::wrapped_to_collateral(*input, Token(DOT));
// 			assert_ok!(result, *expected);
// 		}
// 	});
// }

// #[test]
// fn collateral_to_wrapped() {
// 	run_test(|| {
// 		Oracle::get_price
// 			.mock_safe(|_| MockResult::Return(Ok(FixedU128::checked_from_rational(2, 1).unwrap())));
// 		let test_cases = [(0, 0), (4, 2), (20, 10), (21, 10)];
// 		for (input, expected) in test_cases.iter() {
// 			let result = Oracle::collateral_to_wrapped(*input, Token(DOT));
// 			assert_ok!(result, *expected);
// 		}
// 	});
// }

#[test]
fn test_is_invalidated() {
	run_test(|| {
		let now = 1585776145;
		Oracle::get_current_time.mock_safe(move || MockResult::Return(now));
		Oracle::get_max_delay.mock_safe(|| MockResult::Return(3600));
		Oracle::is_authorized.mock_safe(|_| MockResult::Return(true));

		let key = OracleKey::ExchangeRate(Token(DOT));
		let rate = FixedU128::checked_from_rational(100, 1).unwrap();

		Oracle::is_authorized.mock_safe(|_| MockResult::Return(true));
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(3), vec![(key.clone(), rate)]));
		mine_block();

		// max delay is 60 minutes, 60+ passed
		assert!(Oracle::is_outdated(&key, now + 3601));

		// max delay is 60 minutes, 30 passed
		Oracle::get_current_time.mock_safe(move || MockResult::Return(now + 1800));
		assert!(!Oracle::is_outdated(&key, now + 3599));
	});
}

#[test]
fn oracle_names_have_genesis_info() {
	run_test(|| {
		let actual = String::from_utf8(Oracle::authorized_oracles(0).to_vec()).unwrap();
		let expected = "test".to_owned();
		assert_eq!(actual, expected);
	});
}

#[test]
fn insert_authorized_oracle_succeeds() {
	run_test(|| {
		let oracle = 1;
		let key = OracleKey::ExchangeRate(Token(DOT));
		let rate = FixedU128::checked_from_rational(1, 1).unwrap();
		let name = BoundedVec::default();
		assert_err!(
			Oracle::feed_values(RuntimeOrigin::signed(oracle), vec![]),
			TestError::InvalidOracleSource
		);
		assert_err!(
			Oracle::insert_authorized_oracle(RuntimeOrigin::signed(oracle), oracle, name.clone()),
			DispatchError::BadOrigin
		);
		assert_ok!(Oracle::insert_authorized_oracle(RuntimeOrigin::root(), oracle, name.clone()));
		assert_emitted!(Event::OracleAdded { oracle_id: 1, name });
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(oracle), vec![(key, rate)]));
	});
}

#[test]
fn remove_authorized_oracle_succeeds() {
	run_test(|| {
		let oracle = 1;
		Oracle::insert_oracle(oracle, BoundedVec::default());
		assert_err!(
			Oracle::remove_authorized_oracle(RuntimeOrigin::signed(oracle), oracle),
			DispatchError::BadOrigin
		);
		assert_ok!(Oracle::remove_authorized_oracle(RuntimeOrigin::root(), oracle,));
		assert_emitted!(Event::OracleRemoved { oracle_id: 1 });
	});
}

#[test]
fn set_btc_tx_fees_per_byte_succeeds() {
	run_test(|| {
		Oracle::is_authorized.mock_safe(|_| MockResult::Return(true));

		let keys = vec![OracleKey::FeeEstimation];

		let values: Vec<_> = keys
			.iter()
			.enumerate()
			.map(|(idx, key)| {
				(key.clone(), FixedU128::checked_from_rational(idx as u32, 1).unwrap())
			})
			.collect();

		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(3), values.clone()));
		mine_block();

		for (key, value) in values {
			assert_eq!(Oracle::get_price(key).unwrap(), value);
		}
	});
}

#[test]
fn test_median() {
	let test_cases = [
		(vec![], None),
		(vec![2], Some(2.0)),
		(vec![2, 1], Some(1.5)),
		(vec![1, 2, 3], Some(2.0)),
		(vec![2, 1, 3], Some(2.0)),
		(vec![10, 2, 1, 3], Some(2.5)),
		(vec![10, 2, 1, 3, 0], Some(2.0)),
	];
	for (input, output) in test_cases {
		let input_fixedpoint = input.into_iter().map(|x| FixedU128::from(x)).collect();
		let output_fixedpoint = output.map(|x| FixedU128::from_float(x));

		assert_eq!(Oracle::median(input_fixedpoint), output_fixedpoint);
	}
}
