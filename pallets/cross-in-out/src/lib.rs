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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// pub use crate::imbalances::{NegativeImbalance, PositiveImbalance};
extern crate alloc;

use alloc::{vec, vec::Vec};
use frame_support::{
	dispatch::DispatchErrorWithPostInfo, ensure, pallet_prelude::*,
	sp_runtime::traits::AccountIdConversion, PalletId,
};
use frame_system::pallet_prelude::*;
use node_primitives::{AssetId, BridgeOperator, CurrencyId, TryConvertFrom, XcmOperationType, FIL};
use orml_traits::MultiCurrency;
use pallet_bcmp::Message;
use sp_core::{H256, U256};
use sp_runtime::{traits::UniqueSaturatedFrom, SaturatedConversion};
use sp_std::boxed::Box;
pub use weights::WeightInfo;
use xcm::v3::prelude::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod migrations;
mod mock;
mod tests;
pub mod weights;

pub use pallet::*;

type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::Balance;
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

type CurrencyBalance<T> =
	<<T as pallet_bcmp::Config>::Currency as frame_support::traits::Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

const MAX_ACCOUNT_LENGTH: usize = 32;
const AMOUNT_LENGTH: usize = 32;
const MAX_CURRENCY_ID_LENGTH: usize = 32;

#[derive(RuntimeDebug, Clone, Eq, PartialEq, Encode, Decode, TypeInfo)]
pub struct Payload<T: Config> {
	pub amount: BalanceOf<T>,
	pub currency_id: CurrencyId,
	pub receiver: AccountIdOf<T>,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_bcmp::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Currecny operation handler
		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		/// The only origin that can edit token issuer list
		type ControlOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Entrance account Pallet Id
		type EntrancePalletId: Get<PalletId>;

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;

		#[pallet::constant]
		type MaxLengthLimit: Get<u32>;

		/// Address represent this pallet, ie 'keccak256(&b"PALLET_CONSUMER"))'
		#[pallet::constant]
		type AnchorAddress: Get<H256>;
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughBalance,
		NotExist,
		NotAllowed,
		CurrencyNotSupportCrossInAndOut,
		NoMultilocationMapping,
		NoAccountIdMapping,
		AlreadyExist,
		NoCrossingMinimumSet,
		AmountLowerThanMinimum,
		ExceedMaxLengthLimit,
		FailedToConvert,
		InvalidDestinationMultilocation,
		CrossOutFeeNotSet,
		InvalidCrossInPath,
		FailedToSendMessage,
		InvalidPayloadLength,
		Unexpected,
		NotSupported,
		CrossOutInfoNotSet,
		ChainNetworkIdNotExist,
		ReceiverNotProvided
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		CrossedOut {
			currency_id: CurrencyId,
			crosser: AccountIdOf<T>,
			location: MultiLocation,
			amount: BalanceOf<T>,
		},
		CrossedIn {
			currency_id: CurrencyId,
			dest: AccountIdOf<T>,
			location: MultiLocation,
			amount: BalanceOf<T>,
			remark: Option<Vec<u8>>,
		},
		CurrencyRegistered {
			currency_id: CurrencyId,
		},
		CurrencyDeregistered {
			currency_id: CurrencyId,
		},
		AddedToIssueList {
			account: AccountIdOf<T>,
			currency_id: CurrencyId,
		},
		RemovedFromIssueList {
			account: AccountIdOf<T>,
			currency_id: CurrencyId,
		},
		LinkedAccountRegistered {
			currency_id: CurrencyId,
			who: AccountIdOf<T>,
			foreign_location: MultiLocation,
		},
		AddedToRegisterList {
			account: AccountIdOf<T>,
			currency_id: CurrencyId,
		},
		RemovedFromRegisterList {
			account: AccountIdOf<T>,
			currency_id: CurrencyId,
		},
		CrossingMinimumAmountSet {
			currency_id: CurrencyId,
			cross_in_and_cross_out_minimum: Option<(BalanceOf<T>, BalanceOf<T>)>,
		},
		CrossOutInfoSet {
			network_id: NetworkId,
			operation: XcmOperationType,
			src_dst_anchor_and_fee: Option<(H256, H256, BalanceOf<T>)>,
		},
		ChainNetworkIdSet {
			chain_native_currency_id: CurrencyId,
			network_id: Option<NetworkId>,
		},
	}

	/// The current storage version, we set to 2 our new version(after migrate stroage from vec t
	/// boundedVec).
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	/// To store currencies that support indirect cross-in and cross-out.
	#[pallet::storage]
	#[pallet::getter(fn get_cross_currency_registry)]
	pub type CrossCurrencyRegistry<T> = StorageMap<_, Blake2_128Concat, CurrencyId, ()>;

	/// Accounts in the whitelist can issue the corresponding Currency.
	#[pallet::storage]
	#[pallet::getter(fn get_issue_whitelist)]
	pub type IssueWhiteList<T: Config> =
		StorageMap<_, Blake2_128Concat, CurrencyId, BoundedVec<AccountIdOf<T>, T::MaxLengthLimit>>;

	/// Accounts in the whitelist can register the mapping between a multilocation and an accountId.
	#[pallet::storage]
	#[pallet::getter(fn get_register_whitelist)]
	pub type RegisterWhiteList<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, Vec<AccountIdOf<T>>>;

	/// Mapping a Bifrost account to a multilocation of a outer chain
	#[pallet::storage]
	#[pallet::getter(fn account_to_outer_multilocation)]
	pub type AccountToOuterMultilocation<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyId,
		Blake2_128Concat,
		AccountIdOf<T>,
		MultiLocation,
		OptionQuery,
	>;

	/// Mapping a multilocation of a outer chain to a Bifrost account
	#[pallet::storage]
	#[pallet::getter(fn outer_multilocation_to_account)]
	pub type OuterMultilocationToAccount<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyId,
		Blake2_128Concat,
		MultiLocation,
		AccountIdOf<T>,
		OptionQuery,
	>;

	/// minimum crossin and crossout amount【crossinMinimum, crossoutMinimum】
	#[pallet::storage]
	#[pallet::getter(fn get_crossing_minimum_amount)]
	pub type CrossingMinimumAmount<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, (BalanceOf<T>, BalanceOf<T>)>;

	/// 【 NetworkId, Operation 】 => (src-anchor, dst-anchor, crossout-fee)
	/// cross-out fee amount in BNC
	#[pallet::storage]
	#[pallet::getter(fn get_cross_out_info)]
	pub type CrossOutInfo<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		NetworkId,
		Blake2_128Concat,
		XcmOperationType,
		(H256, H256, BalanceOf<T>),
		OptionQuery,
	>;

	/// chain native currency id => chain network id
	#[pallet::storage]
	#[pallet::getter(fn get_chain_network_id)]
	pub type ChainNetworkId<T> = StorageMap<_, Blake2_128Concat, CurrencyId, NetworkId>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::cross_in())]
		pub fn cross_in(
			origin: OriginFor<T>,
			location: Box<MultiLocation>,
			currency_id: CurrencyId,
			#[pallet::compact] amount: BalanceOf<T>,
			remark: Option<Vec<u8>>,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;

			// currency_id must not be FIL, FIL should be cross-in by pallet-bcmp
			ensure!(currency_id != FIL, Error::<T>::InvalidCrossInPath);

			let issue_whitelist =
				Self::get_issue_whitelist(currency_id).ok_or(Error::<T>::NotAllowed)?;
			ensure!(issue_whitelist.contains(&issuer), Error::<T>::NotAllowed);

			Self::inner_cross_in(location, currency_id, amount, remark)?;

			Ok(())
		}

		/// Destroy some balance from an account and issue cross-out event.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::cross_out())]
		pub fn cross_out(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			let crosser = ensure_signed(origin)?;

			ensure!(
				CrossCurrencyRegistry::<T>::contains_key(currency_id),
				Error::<T>::CurrencyNotSupportCrossInAndOut
			);

			let crossing_minimum_amount = Self::get_crossing_minimum_amount(currency_id)
				.ok_or(Error::<T>::NoCrossingMinimumSet)?;
			ensure!(amount >= crossing_minimum_amount.1, Error::<T>::AmountLowerThanMinimum);

			let balance = T::MultiCurrency::free_balance(currency_id, &crosser);
			ensure!(balance >= amount, Error::<T>::NotEnoughBalance);

			let location = AccountToOuterMultilocation::<T>::get(currency_id, &crosser)
				.ok_or(Error::<T>::NoMultilocationMapping)?;

			T::MultiCurrency::withdraw(currency_id, &crosser, amount)?;

			// if currecny_id is FIL, send message to pallet-bcmp
			if currency_id == FIL {
				Self::send_message(crosser.clone(), currency_id, amount, Box::new(location))?;
			}

			Self::deposit_event(Event::CrossedOut { currency_id, crosser, location, amount });
			Ok(())
		}

		// Register the mapping relationship of Bifrost account and account from other chains
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::register_linked_account())]
		pub fn register_linked_account(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: AccountIdOf<T>,
			foreign_location: Box<MultiLocation>,
		) -> DispatchResult {
			let registerer = ensure_signed(origin)?;

			let register_whitelist =
				Self::get_register_whitelist(currency_id).ok_or(Error::<T>::NotAllowed)?;
			ensure!(register_whitelist.contains(&registerer), Error::<T>::NotAllowed);

			ensure!(
				CrossCurrencyRegistry::<T>::contains_key(currency_id),
				Error::<T>::CurrencyNotSupportCrossInAndOut
			);

			ensure!(
				!AccountToOuterMultilocation::<T>::contains_key(&currency_id, who.clone()),
				Error::<T>::AlreadyExist
			);

			AccountToOuterMultilocation::<T>::insert(
				currency_id,
				who.clone(),
				foreign_location.clone(),
			);
			OuterMultilocationToAccount::<T>::insert(
				currency_id,
				foreign_location.clone(),
				who.clone(),
			);

			Pallet::<T>::deposit_event(Event::LinkedAccountRegistered {
				currency_id,
				who,
				foreign_location: *foreign_location,
			});

			Ok(())
		}

		// Change originally registered linked outer chain multilocation
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::change_outer_linked_account())]
		pub fn change_outer_linked_account(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			foreign_location: Box<MultiLocation>,
			account: AccountIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			ensure!(
				CrossCurrencyRegistry::<T>::contains_key(currency_id),
				Error::<T>::CurrencyNotSupportCrossInAndOut
			);

			let original_location =
				Self::account_to_outer_multilocation(currency_id, account.clone())
					.ok_or(Error::<T>::NotExist)?;
			ensure!(original_location != *foreign_location.clone(), Error::<T>::AlreadyExist);

			AccountToOuterMultilocation::<T>::insert(
				currency_id,
				account.clone(),
				foreign_location.clone(),
			);
			OuterMultilocationToAccount::<T>::insert(
				currency_id,
				foreign_location.clone(),
				account.clone(),
			);

			Pallet::<T>::deposit_event(Event::LinkedAccountRegistered {
				currency_id,
				who: account,
				foreign_location: *foreign_location,
			});

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::register_currency_for_cross_in_out())]
		pub fn register_currency_for_cross_in_out(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			CrossCurrencyRegistry::<T>::mutate_exists(currency_id, |registration| {
				if registration.is_none() {
					*registration = Some(());

					Self::deposit_event(Event::CurrencyRegistered { currency_id });
				}
			});

			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::deregister_currency_for_cross_in_out())]
		pub fn deregister_currency_for_cross_in_out(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			if CrossCurrencyRegistry::<T>::take(currency_id).is_some() {
				Self::deposit_event(Event::CurrencyDeregistered { currency_id });
			};

			Ok(())
		}

		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::add_to_issue_whitelist())]
		pub fn add_to_issue_whitelist(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			account: AccountIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let rs = Self::get_issue_whitelist(currency_id);
			let mut issue_whitelist;
			if let Some(bounded_vec) = rs {
				issue_whitelist = bounded_vec.to_vec();
				ensure!(
					issue_whitelist.len() < T::MaxLengthLimit::get() as usize,
					Error::<T>::ExceedMaxLengthLimit
				);
				ensure!(!issue_whitelist.contains(&account), Error::<T>::AlreadyExist);

				issue_whitelist.push(account.clone());
			} else {
				issue_whitelist = vec![account.clone()];
			}

			let bounded_issue_whitelist =
				BoundedVec::try_from(issue_whitelist).map_err(|_| Error::<T>::FailedToConvert)?;

			IssueWhiteList::<T>::insert(currency_id, bounded_issue_whitelist);

			Self::deposit_event(Event::AddedToIssueList { account, currency_id });

			Ok(())
		}

		#[pallet::call_index(7)]
		#[pallet::weight(<T as Config>::WeightInfo::remove_from_issue_whitelist())]
		pub fn remove_from_issue_whitelist(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			account: AccountIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			IssueWhiteList::<T>::mutate(currency_id, |issue_whitelist| -> Result<(), Error<T>> {
				match issue_whitelist {
					Some(issue_list) if issue_list.contains(&account) => {
						issue_list.retain(|x| x.clone() != account);
						Self::deposit_event(Event::RemovedFromIssueList { account, currency_id });
						Ok(())
					},
					_ => Err(Error::<T>::NotExist),
				}
			})?;

			Ok(())
		}

		#[pallet::call_index(8)]
		#[pallet::weight(<T as Config>::WeightInfo::add_to_register_whitelist())]
		pub fn add_to_register_whitelist(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			account: AccountIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let empty_vec: Vec<AccountIdOf<T>> = Vec::new();
			if Self::get_register_whitelist(currency_id) == None {
				RegisterWhiteList::<T>::insert(currency_id, empty_vec);
			}

			RegisterWhiteList::<T>::mutate(
				currency_id,
				|register_whitelist| -> Result<(), Error<T>> {
					match register_whitelist {
						Some(register_list) if !register_list.contains(&account) => {
							register_list.push(account.clone());
							Self::deposit_event(Event::AddedToRegisterList {
								account,
								currency_id,
							});
							Ok(())
						},
						_ => Err(Error::<T>::NotAllowed),
					}
				},
			)?;

			Ok(())
		}

		#[pallet::call_index(9)]
		#[pallet::weight(<T as Config>::WeightInfo::remove_from_register_whitelist())]
		pub fn remove_from_register_whitelist(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			account: AccountIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			RegisterWhiteList::<T>::mutate(
				currency_id,
				|register_whitelist| -> Result<(), Error<T>> {
					match register_whitelist {
						Some(register_list) if register_list.contains(&account) => {
							register_list.retain(|x| x.clone() != account);
							Self::deposit_event(Event::RemovedFromRegisterList {
								account,
								currency_id,
							});
							Ok(())
						},
						_ => Err(Error::<T>::NotExist),
					}
				},
			)?;

			Ok(())
		}

		#[pallet::call_index(10)]
		#[pallet::weight(<T as Config>::WeightInfo::set_crossing_minimum_amount())]
		pub fn set_crossing_minimum_amount(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			cross_in_and_cross_out_minimum: Option<(BalanceOf<T>, BalanceOf<T>)>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			CrossingMinimumAmount::<T>::mutate_exists(currency_id, |old_min_info| {
				*old_min_info = cross_in_and_cross_out_minimum;
			});

			Self::deposit_event(Event::CrossingMinimumAmountSet {
				currency_id,
				cross_in_and_cross_out_minimum,
			});

			Ok(())
		}

		#[pallet::call_index(11)]
		#[pallet::weight(<T as Config>::WeightInfo::set_cross_out_info())]
		pub fn set_cross_out_info(
			origin: OriginFor<T>,
			network_id: NetworkId,
			operation: XcmOperationType,
			src_dst_anchor_and_fee: Option<(H256, H256, BalanceOf<T>)>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			CrossOutInfo::<T>::mutate_exists(network_id, operation, |old_info| {
				*old_info = src_dst_anchor_and_fee;
			});

			Self::deposit_event(Event::CrossOutInfoSet {
				network_id,
				operation,
				src_dst_anchor_and_fee,
			});

			Ok(())
		}

		#[pallet::call_index(12)]
		#[pallet::weight(<T as Config>::WeightInfo::set_chain_network_id())]
		pub fn set_chain_network_id(
			origin: OriginFor<T>,
			chain_native_currency_id: CurrencyId,
			network_id: Option<NetworkId>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			ChainNetworkId::<T>::mutate_exists(chain_native_currency_id, |old_network_id| {
				*old_network_id = network_id;
			});

			Self::deposit_event(Event::ChainNetworkIdSet { chain_native_currency_id, network_id });

			Ok(())
		}
	}

	impl<T: Config> pallet_bcmp::ConsumerLayer<T> for Pallet<T> {
		/// Called by 'Bcmp::receive_message', has already verified committee's signature.
		fn receive_op(message: &Message) -> DispatchResultWithPostInfo {
			let payload = Self::parse_payload(&message.payload)?;

			let accuount_u8_array = Self::get_account_id_u8_array(payload.receiver);

			// receiver account to native location
			let location = Box::new(MultiLocation {
				parents: 0,
				interior: X1(AccountId32 { network: None, id: accuount_u8_array }),
			});

			let remark_op = if &message.extra_feed.len() == &0usize {
				None
			} else {
				Some(message.extra_feed.clone())
			};

			Self::inner_cross_in(location, payload.currency_id, payload.amount, remark_op)?;

			Ok(().into())
		}

		fn anchor_addr() -> H256 {
			T::AnchorAddress::get()
		}
	}

	impl<T: Config> BridgeOperator<AccountIdOf<T>, BalanceOf<T>, CurrencyId> for Pallet<T> {
		type Error = Error<T>;
		fn send_crossout_message(
			fee_payer: AccountIdOf<T>,
			fee: BalanceOf<T>,
			src_anchor: H256,
			payload: Vec<u8>,
			network_id: NetworkId,
		) -> Result<(), Error<T>> {
			let dst_chain: u32 = if let NetworkId::Ethereum { chain_id } = network_id {
				chain_id.saturated_into::<u32>()
			} else {
				return Err(Error::<T>::Unexpected);
			};

			// transform fee type
			let fee = CurrencyBalance::<T>::unique_saturated_from(fee.saturated_into::<u128>());

			pallet_bcmp::Pallet::<T>::send_message(fee_payer, fee, src_anchor, dst_chain, payload)
				.map_err(|_| Error::<T>::FailedToSendMessage)?;

			Ok(())
		}

		fn get_crossout_information(
			network_id: NetworkId,
			operation: XcmOperationType,
		) -> Result<(H256, H256, BalanceOf<T>), Error<T>> {
			let info = Self::get_cross_out_info(network_id, operation)
				.ok_or(Error::<T>::CrossOutInfoNotSet)?;

			Ok(info)
		}

		fn get_chain_network(chain_native_currency_id: CurrencyId) -> Result<NetworkId, Error<T>> {
			let network_id = Self::get_chain_network_id(chain_native_currency_id)
				.ok_or(Error::<T>::ChainNetworkIdNotExist)?;

			Ok(network_id)
		}

		fn get_cross_out_payload(
			operation: XcmOperationType,
			currency_id: CurrencyId,
			amount: BalanceOf<T>,
			receiver_op: Option<&[u8]>,
		) -> Result<Vec<u8>, Error<T>> {
			// the first byte is xcm operation type
			let mut payload = Vec::new();
			payload.push(XcmOperationType::TransferTo as u8);

			// following 32 bytes is currency_id
			let currency_id_asset: AssetId = AssetId::try_convert_from(currency_id, 0u32)
				.map_err(|_| Error::<T>::FailedToConvert)?;
			let mut fixed_currency_id = [0u8; 32];
			U256::from(currency_id_asset.asset_index).to_big_endian(&mut fixed_currency_id);
			payload.append(&mut fixed_currency_id.to_vec());

			// following 32 bytes is amount
			let mut fixed_amount = [0u8; 32];
			U256::from(amount.saturated_into::<u128>()).to_big_endian(&mut fixed_amount);
			payload.append(&mut fixed_amount.to_vec());

			if operation == XcmOperationType::TransferTo {
				let receiver = receiver_op.ok_or(Error::<T>::ReceiverNotProvided)?;
				// following 32 bytes is receiver
				let mut fixed_address = Self::extend_to_bytes32(receiver, 32);
				payload.append(&mut fixed_address);
			}

			Ok(payload)
		}

		fn get_receiver_from_multilocation(
			dest_native_currecny_id: CurrencyId,
			location: &MultiLocation,
		) -> Result<Vec<u8>, Error<T>> {
			// if it is FIL, get account20 from multilocation
			if dest_native_currecny_id == FIL {
				match location {
					MultiLocation {
						parents: _,
						interior: X1(AccountKey20 { network: _, key: account_20 }),
					} => {
						let receiver = account_20.to_vec();
						return Ok(receiver);
					},

					_ => Err(Error::<T>::InvalidDestinationMultilocation),
				}
			} else {
				Err(Error::<T>::NotSupported)?
			}
		}

		fn get_network_id_from_multilocation(
			dest_native_currecny_id: CurrencyId,
			location: &MultiLocation,
		) -> Result<NetworkId, Error<T>> {
			// if it is FIL, get account20 from multilocation
			if dest_native_currecny_id == FIL {
				match location {
					MultiLocation {
						parents: _,
						interior: X1(AccountKey20 { network: Some(network_id), key: _ }),
					} => {
						return Ok(*network_id);
					},

					_ => Err(Error::<T>::InvalidDestinationMultilocation),
				}
			} else {
				Err(Error::<T>::NotSupported)?
			}
		}

		fn get_registered_account_from_outer_multilocation(
			currency_id: CurrencyId,
			dest_location: &MultiLocation,
		) -> Result<AccountIdOf<T>, Error<T>> {
			let account = Self::outer_multilocation_to_account(currency_id, &dest_location)
				.ok_or(Error::<T>::NoAccountIdMapping)?;
				Ok(account)
		}

		fn get_registered_outer_multilocation_from_account(
			currency_id: CurrencyId,
			account: AccountIdOf<T>,
		) -> Result<MultiLocation, Error<T>> {
			let location = Self::account_to_outer_multilocation(currency_id, account)
				.ok_or(Error::<T>::NoMultilocationMapping)?;

				Ok(location)
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn send_message(
			fee_payer: AccountIdOf<T>,
			currency_id: CurrencyId,
			amount: BalanceOf<T>,
			dest_location: Box<MultiLocation>,
		) -> Result<(), Error<T>> {
			let receiver = Self::get_receiver_from_multilocation(currency_id, &dest_location)?;
			let network_id = Self::get_network_id_from_multilocation(currency_id, &dest_location)?;

			let payload = Self::get_cross_out_payload(
				XcmOperationType::TransferTo,
				currency_id,
				amount,
				Some(&receiver),
			)?;

			let transfer_crossout_info =
				Self::get_crossout_information(network_id, XcmOperationType::TransferTo)?;
			let src_anchor = transfer_crossout_info.0;
			let fee = transfer_crossout_info.2;

			Self::send_crossout_message(fee_payer, fee, src_anchor, payload, network_id)?;

			Ok(())
		}

		/// Example for parsing payload from Evm payload, contains 'amount' and 'receiver'.
		pub(crate) fn parse_payload(raw: &[u8]) -> Result<Payload<T>, DispatchErrorWithPostInfo> {
			return if raw.len() == MAX_ACCOUNT_LENGTH + AMOUNT_LENGTH + MAX_CURRENCY_ID_LENGTH {
				let amount: u128 = U256::from_big_endian(&raw[..32])
					.try_into()
					.map_err(|_| Error::<T>::FailedToConvert)?;

				// decode currency_id
				let currency_id_u64: u64 = U256::from_big_endian(&raw[32..64])
					.try_into()
					.map_err(|_| Error::<T>::FailedToConvert)?;
				let currency_id = CurrencyId::try_from(currency_id_u64)
					.map_err(|_| Error::<T>::FailedToConvert)?;

				// account id decode may different, ie. 'AccountId20', 'AccountId32', ..
				let account_len = T::AccountId::max_encoded_len();
				if account_len >= raw.len() {
					return Err(Error::<T>::FailedToConvert.into());
				}
				let receiver = T::AccountId::decode(&mut raw[raw.len() - account_len..].as_ref())
					.map_err(|_| Error::<T>::FailedToConvert)?;
				Ok(Payload {
					amount: SaturatedConversion::saturated_from(amount),
					currency_id,
					receiver,
				})
			} else {
				Err(Error::<T>::InvalidPayloadLength.into())
			};
		}

		/// Extend bytes to target length.
		pub(crate) fn extend_to_bytes32(data: &[u8], size: usize) -> Vec<u8> {
			let mut append = Vec::new();
			let mut len = data.len();
			while len < size {
				append.push(0);
				len += 1;
			}
			append.append(&mut data.to_vec());
			append
		}

		pub(crate) fn inner_cross_in(
			location: Box<MultiLocation>,
			currency_id: CurrencyId,
			amount: BalanceOf<T>,
			remark: Option<Vec<u8>>,
		) -> Result<(), Error<T>> {
			ensure!(
				CrossCurrencyRegistry::<T>::contains_key(currency_id),
				Error::<T>::CurrencyNotSupportCrossInAndOut
			);

			let crossing_minimum_amount = Self::get_crossing_minimum_amount(currency_id)
				.ok_or(Error::<T>::NoCrossingMinimumSet)?;
			ensure!(amount >= crossing_minimum_amount.0, Error::<T>::AmountLowerThanMinimum);

			let entrance_account_mutlilcaition = Box::new(MultiLocation {
				parents: 0,
				interior: X1(AccountId32 {
					network: None,
					id: T::EntrancePalletId::get().into_account_truncating(),
				}),
			});

			// If the cross_in destination is entrance account, it is not required to be registered.
			let dest = if entrance_account_mutlilcaition == location {
				T::EntrancePalletId::get().into_account_truncating()
			} else {
				Self::outer_multilocation_to_account(currency_id, location.clone())
					.ok_or(Error::<T>::NoAccountIdMapping)?
			};

			T::MultiCurrency::deposit(currency_id, &dest, amount)
				.map_err(|_| Error::<T>::Unexpected)?;

			Self::deposit_event(Event::CrossedIn {
				dest,
				currency_id,
				location: *location,
				amount,
				remark,
			});

			Ok(())
		}

		pub(crate) fn get_account_id_u8_array(account_id: AccountIdOf<T>) -> [u8; 32] {
			let mut account_id_u8_array = [0u8; 32];
			let account_id_u8_vec = account_id.encode();
			account_id_u8_array.copy_from_slice(&account_id_u8_vec);
			account_id_u8_array
		}
	}
}
