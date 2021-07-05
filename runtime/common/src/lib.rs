#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::sp_runtime::offchain::storage_lock::BlockNumberProvider;
use node_primitives::BlockNumber;

pub mod xcm_impl;

pub struct RelaychainBlockNumberProvider<T>(sp_std::marker::PhantomData<T>);

impl<T: cumulus_pallet_parachain_system::Config> BlockNumberProvider
	for RelaychainBlockNumberProvider<T>
{
	type BlockNumber = BlockNumber;

	fn current_block_number() -> Self::BlockNumber {
		cumulus_pallet_parachain_system::Pallet::<T>::validation_data()
			.map(|d| d.relay_parent_number)
			.unwrap_or_default()
	}
}
