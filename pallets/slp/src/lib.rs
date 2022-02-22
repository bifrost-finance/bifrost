// This file is part of Bifrost.

// Copyright (C) 2019-2022 Liebi Technologies (UK) Ltd.
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

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, weights::Weight, PalletId};
use frame_system::pallet_prelude::OriginFor;
use node_primitives::CurrencyId;
use orml_traits::MultiCurrency;
pub use primitives::{Delays, Ledger, TimeUnit};
pub use weights::WeightInfo;
use xcm::latest::*;

use crate::{
	primitives::{SubstrateLedger, XcmOperation},
	traits::{DelegatorManager, StakingAgent, StakingFeeManager, ValidatorManager},
};

mod agents;
mod mock;
pub mod primitives;
mod tests;
pub mod traits;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub use pallet::*;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Currency operations handler
		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;
		/// The only origin that can modify pallet params
		type ControlOrigin: EnsureOrigin<Self::Origin>;

		/// Set default weight.
		type WeightInfo: WeightInfo;

		/// ModuleID for creating sub account
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Kusama agent
		type KusamaAgent: StakingAgent<MultiLocation, MultiLocation>
			+ StakingFeeManager<AccountIdOf<Self>>
			+ DelegatorManager<MultiLocation, SubstrateLedger<MultiLocation, BalanceOf<Self>>>
			+ ValidatorManager<MultiLocation>;
	}

	#[pallet::error]
	pub enum Error<T> {
		OverFlow,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// [CurrencyId, DelegatorId]
		DelegatorInitialized(CurrencyId, MultiLocation),
	}

	/// The dest weight limit and fee for execution XCM msg sended out. Must be
	/// sufficient, otherwise the execution of XCM msg on the dest chain will fail.
	///
	/// XcmDestWeightAndFee: DoubleMap: CurrencyId, XcmOperation => (Weight, Balance)
	#[pallet::storage]
	#[pallet::getter(fn xcm_dest_weight_and_fee)]
	pub type XcmDestWeightAndFee<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyId,
		Blake2_128Concat,
		XcmOperation,
		(Weight, BalanceOf<T>),
		OptionQuery,
	>;

	/// One operate origin(can be a multisig account) for a currency. An operating origins are
	/// normal account in Bifrost chain.
	#[pallet::storage]
	#[pallet::getter(fn get_operate_origin)]
	pub type OperateOrigins<T> = StorageMap<_, Blake2_128Concat, CurrencyId, AccountIdOf<T>>;

	/// Record current TimeUnit for a certain chain. For example, Kusama's current era is 808.
	#[pallet::storage]
	#[pallet::getter(fn get_current_time_unit)]
	pub type CurrentTimeUnit<T> = StorageMap<_, Blake2_128Concat, CurrencyId, TimeUnit>;

	/// Currency delays for payouts, delegate, unbond and so on.
	#[pallet::storage]
	#[pallet::getter(fn get_currency_delays)]
	pub type CurrencyDelays<T> = StorageMap<_, Blake2_128Concat, CurrencyId, Delays>;
	/// Origins and Amounts for the staking operating account fee supplement. An operating account
	/// is identified in MultiLocation format.
	#[pallet::storage]
	#[pallet::getter(fn get_fee_source)]
	pub type FeeSources<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, (MultiLocation, BalanceOf<T>)>;

	/// Delegators in service. A delegator is identified in MultiLocation format.
	#[pallet::storage]
	#[pallet::getter(fn get_delegator)]
	pub type Delegators<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyId,
		Blake2_128Concat,
		u16,
		MultiLocation,
		OptionQuery,
	>;

	/// Next index of different currency delegators.
	#[pallet::storage]
	#[pallet::getter(fn get_delegator_next_index)]
	pub type DelegatorNextIndex<T> = StorageMap<_, Blake2_128Concat, CurrencyId, u16, ValueQuery>;

	/// Validator in service. A validator is identified in MultiLocation format.
	#[pallet::storage]
	#[pallet::getter(fn get_validators)]
	pub type Validators<T> = StorageMap<_, Blake2_128Concat, CurrencyId, Vec<MultiLocation>>;

	/// Delegator ledgers. A delegator is identified in MultiLocation format.
	#[pallet::storage]
	#[pallet::getter(fn get_delegator_ledger)]
	pub type DelegatorLedgers<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyId,
		Blake2_128Concat,
		MultiLocation,
		Ledger<MultiLocation, BalanceOf<T>>,
		OptionQuery,
	>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_initialize(_n: T::BlockNumber) -> Weight {
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Update storage XcmDestWeightAndFee<T>.
		#[pallet::weight(T::WeightInfo::set_xcm_dest_weight_and_fee())]
		pub fn set_xcm_dest_weight_and_fee(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			operation: XcmOperation,
			weight: Weight,
			fee: BalanceOf<T>,
		) -> DispatchResult {
			unimplemented!()
		}

		/// Update storage OperateOrigins<T>.
		#[pallet::weight(T::WeightInfo::set_operate_origin())]
		pub fn set_operate_origin(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: AccountIdOf<T>,
		) -> DispatchResult {
			unimplemented!()
		}

		/// Update storage CurrentTimeUnit<T>.
		#[pallet::weight(T::WeightInfo::set_current_time_unit())]
		pub fn set_current_time_unit(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			time_unit: TimeUnit,
		) -> DispatchResult {
			unimplemented!()
		}

		/// Update storage CurrencyDelays<T>.
		#[pallet::weight(T::WeightInfo::set_currency_delays())]
		pub fn set_currency_delays(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			delays: Delays,
		) -> DispatchResult {
			unimplemented!()
		}

		/// Update storage FeeSources<T>.
		#[pallet::weight(T::WeightInfo::set_fee_source())]
		pub fn set_fee_source(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			fee: BalanceOf<T>,
		) -> DispatchResult {
			unimplemented!()
		}

		/// Update storage Delegators<T>.
		#[pallet::weight(T::WeightInfo::set_delegators())]
		pub fn set_delegators(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			index: u16,
			who: MultiLocation,
		) -> DispatchResult {
			unimplemented!()
		}

		/// Update storage Validators<T>.
		#[pallet::weight(T::WeightInfo::set_validators())]
		pub fn set_validators(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			validators: Vec<MultiLocation>,
		) -> DispatchResult {
			unimplemented!()
		}

		/// Update storage DelegatorLedgers<T>.
		#[pallet::weight(T::WeightInfo::set_delegator_ledger())]
		pub fn set_delegator_ledger(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			ledger: Ledger<MultiLocation, BalanceOf<T>>,
		) -> DispatchResult {
			unimplemented!()
		}
	}
}
