// Copyright 2019 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use core::result;

use codec::{Decode, Encode};
use rstd::prelude::*;
use sr_primitives::traits::{Member, SaturatedConversion, SimpleArithmetic};
use sr_primitives::transaction_validity::{TransactionLongevity, TransactionValidity, UnknownTransaction, ValidTransaction};
use srml_support::{decl_event, decl_module, decl_storage, Parameter};
use substrate_primitives::offchain::Timestamp;
use system::{ensure_none, ensure_root, ensure_signed};
use system::offchain::SubmitUnsignedTransaction;

use node_primitives::{AssetIssue, AssetRedeem};
use transaction::*;

mod transaction;
mod mock;
mod tests;

#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug)]
pub struct Bank {
	account: Vec<u8>,
	authorities: Vec<Vec<u8>>,
	threshold: u32,
}

#[cfg(feature = "std")]
#[derive(Debug)]
pub enum Error {
	EosPrimitivesError(eos_primitives::Error),
	// Todo, failure::Error isn't compatible with std error,
	// it's difficult to convert failure::Error to std error.
	// after drop failure lib in eos_rpc, change this variant.
	//	HttpResponseError(eos_rpc::Error),
	HttpResponseError,
	ParseUtf8Error(core::str::Utf8Error),
	SecretKeyError(eos_keys::error::Error),
}

#[cfg(feature = "std")]
impl core::convert::From<eos_primitives::symbol::ParseSymbolError> for Error {
	fn from(err: eos_primitives::symbol::ParseSymbolError) -> Self {
		Self::EosPrimitivesError(eos_primitives::Error::ParseSymbolError(err))
	}
}

/// The module configuration trait.
pub trait Trait: system::Trait {
	/// The overarching event type.
	type Event: From<Event> + Into<<Self as system::Trait>::Event>;

	/// The units in which we record balances.
	type Balance: Member + Parameter + SimpleArithmetic + Default + Copy;

	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + SimpleArithmetic + Default + Copy;

	/// Assets issue handler
	type AssetIssue: AssetIssue<Self::AssetId, Self::AccountId, Self::Balance>;

	/// A dispatchable call type.
	type Call: From<Call<Self>>;

	/// A transaction submitter.
	type SubmitTransaction: SubmitUnsignedTransaction<Self, <Self as Trait>::Call>;
}

decl_event!(
	pub enum Event {
		/// Transaction from another blockchain was mapped.
		BridgeTxMapped,
		/// Transaction from another blockchain was received.
		BridgeTxReceived,
		/// Transaction received from another blockchain was confirmed.
		BridgeTxReceiveConfirmed,
		/// Transaction to another blockchain was sent.
		BridgeTxSent,
	}
);

decl_storage! {
	trait Store for Module<T: Trait> as Bridge {

		NextBankId get(fn next_bank_id): u32;

		Banks get(fn banks): map u32 => Bank;

		BridgeTxs get(fn bridge_txs): Vec<BridgeTransaction>;

		UnsignedReceiveConfirms get(fn unsigned_recv_cfms): Vec<TransactionOut<T::Balance>>;

		SignedReceiveConfirms get(fn signed_recv_cfms): Vec<TransactionOut<T::Balance>>;

		UnsignedSends get(fn unsigned_sends): Vec<TransactionOut<T::Balance>>;

		SignedSends get(fn signed_sends): Vec<TransactionOut<T::Balance>>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {

		fn deposit_event() = default;

		fn relay_tx(origin, target: T::AccountId, amount: T::Balance) {
			let _origin = ensure_root(origin)?;

			Self::receive_tx(target, amount);
		}

		fn relay_tx_confirmed(origin) {
			let _origin = ensure_root(origin)?;
		}

		fn relay_back_confirmed(origin) {
			let _origin = ensure_root(origin)?;
		}

		fn send_tx_update(origin, block_num: T::BlockNumber) {
			ensure_none(origin)?;

			<UnsignedSends<T>>::kill();
		}

		fn handle(origin) {
			ensure_none(origin)?;

//			Self::receive_tx();

			Self::send_tx_sign();

			Self::receive_confirm_tx_sign();
		}

		// Runs after every block.
		fn offchain_worker(now_block: T::BlockNumber) {
			Self::offchain(now_block);
		}
	}
}


impl<T: Trait> AssetRedeem<T::AssetId, T::AccountId, T::Balance> for Module<T> {
	fn asset_redeem(asset_id: T::AssetId, target: T::AccountId, amount: T::Balance, to_name: Vec<u8>) {
		Self::send_tx_gen(asset_id, target, amount, to_name);
	}
}

// The main implementation block for the module.
impl<T: Trait> Module<T> {

	fn offchain(now_block: T::BlockNumber) {
		#[cfg(feature = "std")]
		Self::do_unsigned_recv_tx(now_block);

//		let now_time = sr_io::timestamp();
//		Self::receive_confirm_tx_send(now_time);

//		Self::send_tx_finish(now_time);
	}

	#[cfg(feature = "std")]
	fn do_unsigned_recv_tx(now_block: T::BlockNumber) {
		let count = <UnsignedSends<T>>::decode_len().unwrap_or(0) as u32;
		if count > 0 {
			let sends = <UnsignedSends<T>>::get();
			for send in sends {
				// TODO handler result
				send.generate_unsigned_recv_tx();
			}

			let call = Call::send_tx_update(now_block);
			T::SubmitTransaction::submit_unsigned(call).map_err(|_| ());
		}
	}

	/// Verify transaction from bridge relayer by validator
	fn relay_tx_verify() -> result::Result<(), ()> {
		Ok(())
	}

	/// Receive and map transaction from backing blockchain
	fn receive_tx(target: T::AccountId, amount: T::Balance) {
		// Check if the relay transaction is verified
		match Self::relay_tx_verify() {
			Ok(_) => {
				let asset_id: T::AssetId = 0.into();
				// Map transaction from bridge relayer
				T::AssetIssue::asset_issue(asset_id, target.clone(), amount);

				// Generate receive confirmation transaction for blockchain
				Self::receive_confirm_tx_gen();

				Self::deposit_event(Event::BridgeTxMapped);
			},
			Err(_) => {}
		}
	}

	/// Generate receiving confirm transaction
	fn receive_confirm_tx_gen() {
		// Generate transaction
		let tx = <TransactionOut<T::Balance>>::new();

		// Record transaction
		<UnsignedReceiveConfirms<T>>::append([tx].into_iter());
	}

	/// Sign the receiving confirm transaction by validator
	fn receive_confirm_tx_sign() {
		let tx_vec: Vec<TransactionOut<T::Balance>> = Self::unsigned_recv_cfms();
		let tx_vec = tx_vec.into_iter().filter_map(|item| {
			// Sign the transaction
			// Check if signature threshold is reached to submit transaction to backing blockchain
			match item.reach_threshold() {
				true => {
					match <SignedReceiveConfirms<T>>::append([item.clone()].into_iter()) {
						Ok(_) => None,
						Err(_) => Some(item),
					}
				},
				false => Some(item),
			}
		}).collect::<Vec<_>>();

		<UnsignedReceiveConfirms<T>>::put(tx_vec);
	}

	/// Send receiving confirm transaction
	fn receive_confirm_tx_send(t: Timestamp) {
		let tx_vec: Vec<TransactionOut<T::Balance>> = Self::signed_recv_cfms();
		let tx_vec = tx_vec.into_iter().filter_map(|item| {
			// Check if time is reached to submit transaction to backing blockchain
			match item.reach_timestamp(t) {
				true => Some(item),
				false => None,
			}
		}).collect::<Vec<_>>();

		for tx in tx_vec {
			// Generate http request
		}

		// Send http request
		// Handler http response
		// Update data
	}

	/// Generate sending transaction
	pub fn send_tx_gen(asset_id: T::AssetId, target: T::AccountId, amount: T::Balance, to_name: Vec<u8>) {
		let mut tx = <TransactionOut<T::Balance>>::new();
		tx.amount = amount.clone();
		tx.to_name = to_name.clone();
		<UnsignedSends<T>>::append([tx].into_iter());
	}

	/// Sign the sending transaction by validator
	fn send_tx_sign() {
		let tx_vec: Vec<TransactionOut<T::Balance>> = Self::unsigned_sends();
		let tx_vec = tx_vec.into_iter().filter_map(|item| {
			// Sign the transaction
			// Check if signature threshold is reached to submit transaction to backing blockchain
			match item.reach_threshold() {
				true => {
					match <SignedSends<T>>::append([item.clone()].into_iter()) {
						Ok(_) => None,
						Err(_) => Some(item),
					}
				},
				false => Some(item),
			}
		}).collect::<Vec<_>>();

		<UnsignedSends<T>>::put(tx_vec);
	}

	/// Send transaction finish
	fn send_tx_finish(t: Timestamp) {

	}
}

impl<T: Trait> srml_support::unsigned::ValidateUnsigned for Module<T> {
	type Call = Call<T>;

	fn validate_unsigned(call: &Self::Call) -> TransactionValidity {
		match call {
			Call::send_tx_update(block_num) => {
				let now_block = <system::Module<T>>::block_number().saturated_into::<u64>();
				Ok(ValidTransaction {
					priority: 0,
					requires: vec![],
					provides: vec![(now_block).encode()],
					longevity: TransactionLongevity::max_value(),
					propagate: true,
				})
			},
			_ => UnknownTransaction::NoUnsignedValidator.into(),
		}
	}
}
