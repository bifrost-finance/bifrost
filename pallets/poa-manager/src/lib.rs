// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	// Import various types used to declare pallet in scope.
	use frame_support::pallet_prelude::*;
	use frame_support::traits::ValidatorRegistration;
	use frame_system::pallet_prelude::*;
	use pallet_session::SessionManager;
	use sp_std::prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type ValidatorRegistrationChecker: ValidatorRegistration<Self::AccountId>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ValidatorAdded(T::AccountId),
		ValidatorRemoved(T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
		NotRegistered,
		NotValidator,
	}

	#[pallet::storage]
	#[pallet::getter(fn validators)]
	pub(super) type Validators<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub initial_validators: Vec<T::AccountId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				initial_validators: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for val_id in &self.initial_validators {
				<Validators<T>>::insert(val_id, true);
			}
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub(super) fn add_validator(
			origin: OriginFor<T>,
			validator_id: T::AccountId
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			ensure!(
				T::ValidatorRegistrationChecker::is_registered(&validator_id),
				<Error<T>>::NotRegistered
			);
			<Validators<T>>::insert(&validator_id, true);

			Self::deposit_event(Event::ValidatorAdded(validator_id));

			Ok(().into())
		}

		#[pallet::weight(0)]
		pub(super) fn remove_validator(
			origin: OriginFor<T>,
			validator_id: T::AccountId
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			ensure!(
				<Validators<T>>::contains_key(&validator_id),
				<Error<T>>::NotValidator
			);
			<Validators<T>>::remove(&validator_id);

			Self::deposit_event(Event::ValidatorRemoved(validator_id));

			Ok(().into())
		}
	}

	impl<T: Config> SessionManager<T::AccountId> for Pallet<T> {
		fn new_session(_: u32) -> Option<Vec<T::AccountId>> {
			Some(<Validators<T>>::iter().map(|(val_id, _)| val_id).collect())
		}

		fn end_session(_: u32) {}

		fn start_session(_: u32) {}
	}
}
