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
#![allow(clippy::unused_unit)]
#![allow(deprecated)] // TODO: clear transaction

extern crate alloc;
use crate::types::{SigType, SignatureStruct};
use alloc::vec::Vec;
use frame_support::{ensure, pallet_prelude::*, sp_runtime::traits::Convert};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, FIL};
use orml_traits::MultiCurrency;
use sp_std::boxed::Box;
pub use types::ForeignAccountIdConverter;
pub use weights::WeightInfo;
use xcm::opaque::latest::{Junction, Junctions::X1, MultiLocation};

use bls_signatures::{PublicKey as BlsPubKey, Serialize, Signature as BlsSignature};
use data_encoding::Encoding;
use data_encoding_macro::new_encoding;
use libsecp256k1::{recover, Message, PublicKey, RecoveryId, Signature as EcsdaSignature};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod mock;
mod tests;
mod types;
pub mod weights;

pub use pallet::*;

type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::Balance;
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

/// defines the encoder for base32 encoding with the provided string with no padding
const ADDRESS_ENCODER: Encoding = new_encoding! {
	symbols: "abcdefghijklmnopqrstuvwxyz234567",
	padding: None,
};

/// Length of the checksum hash for string encodings.
pub const CHECKSUM_HASH_LEN: usize = 4;

/// Hash length of payload for Secp and Actor addresses.
pub const PAYLOAD_HASH_LEN: usize = 20;

const BLS_ADDRRESS_TEXT_LEN: usize = 86;
const SECP_ADDRRESS_TEXT_LEN: usize = 41;

/// BLS signature length in bytes.
pub const BLS_SIG_LEN: usize = 96;
/// BLS Public key length in bytes.
pub const BLS_PUB_LEN: usize = 48;

/// Secp256k1 signature length in bytes.
pub const SECP_SIG_LEN: usize = 65;
/// Secp256k1 Public key length in bytes.
pub const SECP_PUB_LEN: usize = 65;
/// Length of the signature input message hash in bytes (32).
pub const SECP_SIG_MESSAGE_HASH_SIZE: usize = 32;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Currecny operation handler
		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		/// The only origin that can edit token issuer list
		type ControlOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		// convert multi location to foreign account Id	string
		type ForeignAccountIdConverter: Convert<Box<MultiLocation>, Option<Vec<u8>>>;

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;
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
		SignatureVerificationFailed,
		SignatureNotProvided,
		InvalidSignature,
		InvalidSignatureLength,
		NotSupportedCurrencyId,
		AccountIdConversionFailed,
		Unexpected,
		InvalidPublicKeyLength,
		InvalidPublicKey,
		ConversionFailed,
		EcrecoverFailed,
		InvalidChecksum,
		InvalidPayload,
		InvalidLength,
		InvalidAddress,
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
			privileged: Option<bool>,
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
			cross_in_minimum: BalanceOf<T>,
			cross_out_minimum: BalanceOf<T>,
		},
	}

	/// To store currencies that support indirect cross-in and cross-out.
	///
	/// 【currenyId, privilaged】, privilaged is bool type, meaning whether it is open for every
	/// account to register account connection. Or it is a privilaged action for some accounts.
	#[pallet::storage]
	#[pallet::getter(fn get_cross_currency_registry)]
	pub type CrossCurrencyRegistry<T> = StorageMap<_, Blake2_128Concat, CurrencyId, Option<bool>>;

	/// Accounts in the whitelist can issue the corresponding Currency.
	#[pallet::storage]
	#[pallet::getter(fn get_issue_whitelist)]
	pub type IssueWhiteList<T> = StorageMap<_, Blake2_128Concat, CurrencyId, Vec<AccountIdOf<T>>>;

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

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::cross_in())]
		pub fn cross_in(
			origin: OriginFor<T>,
			location: Box<MultiLocation>,
			currency_id: CurrencyId,
			#[pallet::compact] amount: BalanceOf<T>,
			remark: Option<Vec<u8>>,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;

			ensure!(
				CrossCurrencyRegistry::<T>::contains_key(currency_id),
				Error::<T>::CurrencyNotSupportCrossInAndOut
			);

			let crossing_minimum_amount = Self::get_crossing_minimum_amount(currency_id)
				.ok_or(Error::<T>::NoCrossingMinimumSet)?;
			ensure!(amount >= crossing_minimum_amount.0, Error::<T>::AmountLowerThanMinimum);

			let issue_whitelist =
				Self::get_issue_whitelist(currency_id).ok_or(Error::<T>::NotAllowed)?;
			ensure!(issue_whitelist.contains(&issuer), Error::<T>::NotAllowed);

			let dest = Self::outer_multilocation_to_account(currency_id, location.clone())
				.ok_or(Error::<T>::NoAccountIdMapping)?;

			T::MultiCurrency::deposit(currency_id, &dest, amount)?;

			Self::deposit_event(Event::CrossedIn {
				dest,
				currency_id,
				location: *location,
				amount,
				remark,
			});
			Ok(())
		}

		/// Destroy some balance from an account and issue cross-out event.
		#[pallet::weight(T::WeightInfo::cross_out())]
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

			Self::deposit_event(Event::CrossedOut { currency_id, crosser, location, amount });
			Ok(())
		}

		// Register the mapping relationship of Bifrost account and publicKey from other chains
		#[pallet::weight(T::WeightInfo::register_linked_account())]
		pub fn register_linked_account(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: AccountIdOf<T>,
			foreign_location: Box<MultiLocation>,
			signature: Option<SignatureStruct>,
		) -> DispatchResult {
			let registerer = ensure_signed(origin)?;

			ensure!(
				CrossCurrencyRegistry::<T>::contains_key(currency_id),
				Error::<T>::CurrencyNotSupportCrossInAndOut
			);

			if let Some(Some(priviliged)) = CrossCurrencyRegistry::<T>::get(currency_id) {
				// If it is only allowed to be registered by the priviliged account. And not allowed
				// to be repeated registered.
				if priviliged {
					ensure!(
						!AccountToOuterMultilocation::<T>::contains_key(&currency_id, who.clone()),
						Error::<T>::AlreadyExist
					);

					let register_whitelist =
						Self::get_register_whitelist(currency_id).ok_or(Error::<T>::NotAllowed)?;
					ensure!(register_whitelist.contains(&registerer), Error::<T>::NotAllowed);
				// If the registration is open for all accounts.
				} else {
					ensure!(registerer == who, Error::<T>::NotAllowed);

					if let Some(sig) = signature {
						if currency_id == FIL {
							// Filecoin address
							let address =
								T::ForeignAccountIdConverter::convert(foreign_location.clone())
									.ok_or(Error::<T>::AccountIdConversionFailed)?;

							let message = Self::get_message(&who, &address)?;

							let valid = Self::filecoin_verify_signature(&message, &sig, &address)?;

							ensure!(valid, Error::<T>::InvalidSignature);
						} else {
							Err(Error::<T>::NotSupportedCurrencyId)?;
						}
					} else {
						Err(Error::<T>::SignatureNotProvided)?;
					}
				}

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
			}

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::register_currency_for_cross_in_out())]
		pub fn register_currency_for_cross_in_out(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			privileged: Option<bool>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let privilage_op = if let Some(_) = privileged { Some(privileged) } else { None };

			CrossCurrencyRegistry::<T>::mutate_exists(currency_id, |old_privilaged| {
				*old_privilaged = privilage_op;
			});

			Self::deposit_event(Event::CurrencyRegistered { currency_id, privileged });

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::add_to_issue_whitelist())]
		pub fn add_to_issue_whitelist(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			account: AccountIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let empty_vec: Vec<AccountIdOf<T>> = Vec::new();
			if Self::get_issue_whitelist(currency_id) == None {
				IssueWhiteList::<T>::insert(currency_id, empty_vec);
			}

			IssueWhiteList::<T>::mutate(currency_id, |issue_whitelist| -> Result<(), Error<T>> {
				match issue_whitelist {
					Some(issue_list) if !issue_list.contains(&account) => {
						issue_list.push(account.clone());
						Self::deposit_event(Event::AddedToIssueList { account, currency_id });
						Ok(())
					},
					_ => Err(Error::<T>::NotAllowed),
				}
			})?;

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::remove_from_issue_whitelist())]
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

		#[pallet::weight(T::WeightInfo::add_to_register_whitelist())]
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

		#[pallet::weight(T::WeightInfo::remove_from_register_whitelist())]
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

		#[pallet::weight(T::WeightInfo::set_crossing_minimum_amount())]
		pub fn set_crossing_minimum_amount(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			cross_in_minimum: BalanceOf<T>,
			cross_out_minimum: BalanceOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			CrossingMinimumAmount::<T>::insert(currency_id, (cross_in_minimum, cross_out_minimum));

			Self::deposit_event(Event::CrossingMinimumAmountSet {
				currency_id,
				cross_in_minimum,
				cross_out_minimum,
			});

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		// message = Bifrost public key + Filecoin address(start with f1, f2, f3...)
		fn get_message(
			who: &AccountIdOf<T>,
			// filecoin public key
			address: &Vec<u8>,
		) -> Result<Vec<u8>, Error<T>> {
			let mut message = Vec::new();
			// Bifrost public key
			message.extend_from_slice(&who.encode());
			// Filecoin address
			message.extend_from_slice(address);

			Ok(message)
		}

		fn filecoin_verify_signature(
			message: &Vec<u8>,
			signature: &SignatureStruct,
			address: &Vec<u8>,
		) -> Result<bool, Error<T>> {
			let add_str =
				std::str::from_utf8(address).map_err(|_e| Error::<T>::ConversionFailed)?;
			let pubkey_payload = Self::parse_address(add_str)?;

			match signature.sig_type {
				SigType::FilecoinBLS =>
					Self::verify_bls_sig(&signature.bytes[..], &message[..], &pubkey_payload),
				SigType::FilecoinSecp256k1 =>
					Self::verify_secp256k1_sig(&signature.bytes[..], &message[..], &pubkey_payload),
			}
		}

		/// Returns `String` error if a bls signature is invalid.
		pub fn verify_bls_sig(
			signature: &[u8],
			data: &[u8],
			pubkey: &[u8],
		) -> Result<bool, Error<T>> {
			// ensure pubkey is correct length
			ensure!(pubkey.len() == BLS_PUB_LEN, Error::<T>::InvalidPublicKeyLength);

			let pub_k = pubkey.to_vec();

			// generate public key object from bytes
			let pk = BlsPubKey::from_bytes(&pub_k).map_err(|_e| Error::<T>::InvalidPublicKey)?;

			// generate signature struct from bytes
			let sig =
				BlsSignature::from_bytes(signature).map_err(|_e| Error::<T>::InvalidSignature)?;

			// BLS verify hash against key
			if bls_signatures::verify_messages(&sig, &[data], &[pk]) {
				Ok(true)
			} else {
				Ok(false)
			}
		}

		/// Returns `String` error if a secp256k1 signature is invalid.
		pub fn verify_secp256k1_sig(
			signature: &[u8],
			data: &[u8],
			pubkey_hash: &[u8],
		) -> Result<bool, Error<T>> {
			// check signature length
			ensure!(signature.len() == SECP_SIG_LEN, Error::<T>::InvalidSignatureLength);

			// blake2b 256 hash
			let hash =
				blake2b_simd::Params::new().hash_length(32).to_state().update(data).finalize();
			let hash = hash.as_bytes().try_into().map_err(|_| Error::<T>::ConversionFailed)?;

			// Ecrecover with hash and signature
			let mut sig = [0u8; SECP_SIG_LEN];
			sig[..].copy_from_slice(signature);
			let rec_pubkey_hash = Self::ecrecover(hash, &sig)?;

			// check address against recovered address
			if &rec_pubkey_hash == pubkey_hash {
				Ok(true)
			} else {
				Ok(false)
			}
		}

		/// Return Address for a message given it's signing bytes hash and signature.
		pub fn ecrecover(
			hash: &[u8; SECP_SIG_MESSAGE_HASH_SIZE],
			signature: &[u8; SECP_SIG_LEN],
		) -> Result<[u8; 20], Error<T>> {
			// recover public key from a message hash and secp signature.
			let key = Self::recover_secp_public_key(hash, signature)?;
			let ret = key.serialize();
			let addr_hash = Self::address_hash(&ret);

			Ok(addr_hash)
		}

		/// Return the public key used for signing a message given it's signing bytes hash and
		/// signature.
		pub fn recover_secp_public_key(
			hash: &[u8; 32],
			signature: &[u8; SECP_SIG_LEN],
		) -> Result<libsecp256k1::PublicKey, Error<T>> {
			// generate types to recover key from
			let rec_id =
				RecoveryId::parse(signature[64]).map_err(|_| Error::<T>::EcrecoverFailed)?;
			let message = Message::parse(hash);

			// Signature value without recovery byte
			let mut s = [0u8; 64];
			s.clone_from_slice(signature[..64].as_ref());

			// generate Signature
			let sig =
				EcsdaSignature::parse_standard(&s).map_err(|_| Error::<T>::EcrecoverFailed)?;
			let recovered =
				recover(&message, &sig, &rec_id).map_err(|_| Error::<T>::EcrecoverFailed)?;
			Ok(recovered)
		}

		/// 这个用来地址转public key, f1和f2是41个字符， f3是86个字符.先不支持f2
		fn parse_address(addr: &str) -> Result<Vec<u8>, Error<T>> {
			// ensure the address has the correct length
			if addr.len() != SECP_ADDRRESS_TEXT_LEN && addr.len() != BLS_ADDRRESS_TEXT_LEN {
				return Err(Error::InvalidLength);
			}

			// ensure the address starts with "f"
			let prefix = addr.get(0..1).ok_or(Error::<T>::InvalidAddress)?;
			ensure!(prefix == "f", Error::<T>::InvalidAddress);

			// get protocol from second character
			let protocol: u8 = match addr.get(1..2).ok_or(Error::<T>::InvalidAddress)? {
				// SECP256K1 key addressing
				"1" => 1,
				// Actor protocol addressing
				"2" => 2,
				// BLS key addressing
				"3" => 3,
				_ => {
					return Err(Error::<T>::InvalidAddress);
				},
			};

			// bytes after the protocol character is the data payload of the address
			let raw = addr.get(2..).ok_or(Error::<T>::InvalidPayload)?;
			// decode using byte32 encoding
			let payload_csum = ADDRESS_ENCODER
				.decode(raw.as_bytes())
				.map_err(|_| Error::<T>::ConversionFailed)?;
			// validate and split payload.
			let payload = Self::validate_and_split_checksum(protocol, None, &payload_csum)?;

			// sanity check to make sure address hash values are correct length
			if match protocol {
				1 | 2 => PAYLOAD_HASH_LEN,
				3 => BLS_PUB_LEN,
				_ => unreachable!(),
			} != payload.len()
			{
				return Err(Error::<T>::InvalidPayload);
			}

			Ok(payload.to_vec())
		}

		fn validate_and_split_checksum<'a>(
			protocol: u8,
			prefix: Option<&[u8]>,
			payload: &'a [u8],
		) -> Result<&'a [u8], Error<T>> {
			if payload.len() < CHECKSUM_HASH_LEN {
				return Err(Error::<T>::InvalidLength);
			}
			let (payload, csum) = payload.split_at(payload.len() - CHECKSUM_HASH_LEN);
			let mut hasher = blake2b_simd::Params::new().hash_length(CHECKSUM_HASH_LEN).to_state();
			hasher.update(&[protocol as u8]);
			if let Some(prefix) = prefix {
				hasher.update(prefix);
			}
			hasher.update(payload);
			if hasher.finalize().as_bytes() != csum {
				return Err(Error::<T>::InvalidChecksum);
			}
			Ok(payload)
		}

		/// Returns an address hash for given data
		fn address_hash(ingest: &[u8]) -> [u8; 20] {
			let digest = blake2b_simd::Params::new()
				.hash_length(PAYLOAD_HASH_LEN)
				.to_state()
				.update(ingest)
				.finalize();

			let mut hash = [0u8; 20];
			hash.copy_from_slice(digest.as_bytes());
			hash
		}
	}
}
