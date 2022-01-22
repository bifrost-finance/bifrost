use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use hex_literal::hex;
use node_primitives::{CurrencyId, TokenSymbol};
use sp_core::H256;
use sp_runtime::AccountId32;
use sp_std::vec;

use super::*;
use crate::{Pallet as MerkleDistributor, *};

const REWARD_2: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);

pub fn lookup_of_account<T: Config>(
	who: T::AccountId,
) -> <<T as frame_system::Config>::Lookup as StaticLookup>::Source {
	<T as frame_system::Config>::Lookup::unlookup(who)
}

benchmarks! {
	where_clause {  where T::CurrencyId : From<CurrencyId>, T::Balance : From<u32>, T::AccountId: From<AccountId32>, T::MerkleDistributorId:From<u32>}

	create_merkle_distributor{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(MerkleDistributor::<T>::add_to_create_whitelist((RawOrigin::Root).into(), caller.clone()));
	}:_(RawOrigin::Signed(caller.clone()), H256::from(&hex!(
				"056980ee78588f3d5ceab5645b2dc2838c19f938151bc1c70547664c6bf57932"
			)),
			Vec::from("test"),
			T::CurrencyId::from(REWARD_2),
			T::Balance::from(1000))

	claim{
		let caller: T::AccountId = whitelisted_caller();

		assert_ok!(MerkleDistributor::<T>::add_to_create_whitelist((RawOrigin::Root).into(), caller.clone()));
		assert_ok!(MerkleDistributor::<T>::create_merkle_distributor(
		   RawOrigin::Signed(caller.clone()),
			H256::from(&hex!(
				"c5a4b4dbe724bfb5aac5879fa145e98686e3e77aacacfc7e6dbea5daa587af3f"
			)),
			Vec::from("test"),
			T::CurrencyId::from(REWARD_2),
			T::Balance::from(1_000_000)
		));

		assert_ok!(<T as Config>::MultiCurrency::deposit(T::CurrencyId::from(REWARD_2), &caller, T::Balance::from(1_000_000_000)));

		assert_ok!(MerkleDistributor::<T>::charge(RawOrigin::Signed(caller.clone()).into(), 0u32.into()));

		let owner = T::AccountId::from(AccountId32::new([
			2, 59, 109, 111, 93, 159, 178, 207, 61, 193, 214, 44, 30, 24, 172, 6, 166, 86, 208, 19,
			81, 244, 212, 48, 252, 107, 222, 166, 182, 88, 246, 56,
		]));

		let owner_look_up = lookup_of_account::<T>(owner.clone());

		let owner_proof:Vec<H256> = vec![
			H256::from(hex![
				"fb4c1fdb961b33fe34628c4a3a99f05d26c06f053000f0eab04ddd2b7857b29d"
			]),
			H256::from(hex![
				"db9586d9476f100d3d63c9fd04925abe451eee1416358de45576cedce9c7b197"
			]),
			H256::from(hex![
				"0564e3219c5663052dbc56d34a194628e134eb3852025202acacfa5be20995a2"
			]),
			H256::from(hex![
				"246dcb49ecfe475d689d26a428d7904a28689c72fb35229ac5484ea9b08baefb"
			]),
		];
	}:_(RawOrigin::Signed(caller.clone()), 0u32.into(), 1, owner_look_up.into(), 291, owner_proof)

	charge{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(MerkleDistributor::<T>::add_to_create_whitelist((RawOrigin::Root).into(), caller.clone()));
		assert_ok!(MerkleDistributor::<T>::create_merkle_distributor(
		   RawOrigin::Signed(caller.clone()),
			H256::from(&hex!(
				"c5a4b4dbe724bfb5aac5879fa145e98686e3e77aacacfc7e6dbea5daa587af3f"
			)),
			Vec::from("test"),
			T::CurrencyId::from(REWARD_2),
			T::Balance::from(1_000_000)
		));

		assert_ok!(<T as Config>::MultiCurrency::deposit(T::CurrencyId::from(REWARD_2), &caller, T::Balance::from(1_000_000_000)));

	}:_(RawOrigin::Signed(caller.clone()), 0u32.into())

	add_to_create_whitelist{
		let caller: T::AccountId = whitelisted_caller();
	}:_((RawOrigin::Root).into(), caller.clone())

	remove_from_create_whitelist{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(MerkleDistributor::<T>::add_to_create_whitelist((RawOrigin::Root).into(), caller.clone()));
	}:_((RawOrigin::Root).into(), caller.clone())
}

impl_benchmark_test_suite!(MerkleDistributor, crate::mock::new_test_ext(), crate::mock::Runtime);
