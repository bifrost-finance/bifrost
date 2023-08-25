use super::{Pallet as Oracle, *};
use crate::OracleKey;
use frame_benchmarking::v2::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use pallet_timestamp::Pallet as Timestamp;
use primitives::{CurrencyId::Token, TokenSymbol::*};
use sp_runtime::FixedPointNumber;
use sp_std::prelude::*;

type MomentOf<T> = <T as pallet_timestamp::Config>::Moment;

#[benchmarks]
pub mod benchmarks {
	use super::*;

	#[benchmark]
	fn on_initialize(u: Linear<1, 1000>) {
		let origin: T::AccountId = account("origin", 0, 0);
		<AuthorizedOracles<T>>::insert(
			origin.clone(),
			BoundedVec::try_from(vec![0; T::MaxNameLength::get() as usize]).unwrap(),
		);

		let values: Vec<_> = (0..u)
			.map(|x| {
				(
					OracleKey::ExchangeRate(CurrencyId::ForeignAsset(x)),
					UnsignedFixedPoint::<T>::checked_from_rational(1, x + 1).unwrap(),
				)
			})
			.collect();

		let valid_until: MomentOf<T> = 100u32.into();
		ValidUntil::<T>::insert(OracleKey::ExchangeRate(Token(DOT)), valid_until);

		Timestamp::<T>::set_timestamp(1000u32.into());

		assert_ok!(crate::Pallet::<T>::feed_values(RawOrigin::Signed(origin).into(), values));

		#[block]
		{
			crate::Pallet::<T>::begin_block(1u32.into());
		}

		for i in 0..u {
			assert!(
				Aggregate::<T>::get(OracleKey::ExchangeRate(CurrencyId::ForeignAsset(i))).is_some()
			);
		}
	}

	#[benchmark]
	fn feed_values(u: Linear<1, 1000>) {
		let origin: T::AccountId = account("origin", 0, 0);
		<AuthorizedOracles<T>>::insert(
			origin.clone(),
			BoundedVec::try_from(vec![0; T::MaxNameLength::get() as usize]).unwrap(),
		);

		let values: Vec<_> = (0..u)
			.map(|x| {
				(
					OracleKey::ExchangeRate(CurrencyId::ForeignAsset(x)),
					UnsignedFixedPoint::<T>::checked_from_rational(1, x + 1).unwrap(),
				)
			})
			.collect();

		#[extrinsic_call]
		feed_values(RawOrigin::Signed(origin), values);

		crate::Pallet::<T>::begin_block(0u32.into());

		for i in 0..u {
			assert!(
				Aggregate::<T>::get(OracleKey::ExchangeRate(CurrencyId::ForeignAsset(i))).is_some()
			);
		}
	}

	#[benchmark]
	fn insert_authorized_oracle() {
		let origin: T::AccountId = account("origin", 0, 0);

		#[extrinsic_call]
		insert_authorized_oracle(
			RawOrigin::Root,
			origin.clone(),
			BoundedVec::try_from(vec![0; T::MaxNameLength::get() as usize]).unwrap(),
		);
		assert_eq!(Oracle::<T>::is_authorized(&origin), true);
	}

	#[benchmark]
	fn remove_authorized_oracle() {
		let origin: T::AccountId = account("origin", 0, 0);
		Oracle::<T>::insert_oracle(
			origin.clone(),
			BoundedVec::try_from(vec![0; T::MaxNameLength::get() as usize]).unwrap(),
		);

		#[extrinsic_call]
		remove_authorized_oracle(RawOrigin::Root, origin.clone());

		assert_eq!(Oracle::<T>::is_authorized(&origin), false);
	}

	impl_benchmark_test_suite!(Oracle, crate::mock::ExtBuilder::build(), crate::mock::Test);
}
