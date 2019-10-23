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

mod transaction;
mod mock;
mod tests;

use core::result;

use codec::{Decode, Encode};
use inherents::{InherentData, InherentIdentifier, MakeFatalError, ProvideInherent, RuntimeString};
#[cfg(feature = "std")]
use inherents::ProvideInherentData;
use rstd::prelude::*;
use sr_primitives::traits::{Member, SimpleArithmetic, SaturatedConversion};
use sr_primitives::transaction_validity::{TransactionValidity, UnknownTransaction, ValidTransaction, TransactionLongevity};
use srml_support::{decl_event, decl_module, decl_storage, Parameter};
use substrate_primitives::offchain::Timestamp;
use system::{ensure_none, ensure_root, ensure_signed};
use system::offchain::SubmitUnsignedTransaction;

use node_primitives::{AssetIssue, AssetRedeem};
use transaction::*;

/// The identifier for the `bridge` inherent.
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"bridge01";

#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug)]
pub struct Bank {
	account: Vec<u8>,
	authorities: Vec<Vec<u8>>,
	threshold: u32,
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

		UnsignedReceiveConfirms get(fn unsigned_recv_cfms): Vec<TransactionOut>;

		SignedReceiveConfirms get(fn signed_recv_cfms): Vec<TransactionOut>;

		UnsignedSends get(fn unsigned_sends): Vec<TransactionOut>;

		SignedSends get(fn signed_sends): Vec<TransactionOut>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {

		fn deposit_event() = default;

		fn relay_tx(origin, amount: T::Balance) {
			let origin = ensure_signed(origin)?;

			Self::receive_tx(origin, amount);
		}

		fn relay_tx_confirmed(origin) {
			let _origin = ensure_root(origin)?;
		}

		fn relay_back_confirmed(origin) {
			let _origin = ensure_root(origin)?;
		}

		fn send_tx_update(origin, block_num: T::BlockNumber) {
			ensure_none(origin)?;

			UnsignedSends::kill();
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

#[cfg(feature = "std")]
pub struct InherentDataProvider;

#[cfg(feature = "std")]
impl ProvideInherentData for InherentDataProvider {
	fn inherent_identifier(&self) -> &'static InherentIdentifier {
		&INHERENT_IDENTIFIER
	}

	fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), RuntimeString> {
		let data = 1;
		inherent_data.put_data(INHERENT_IDENTIFIER, &data)
	}

	fn error_to_string(&self, error: &[u8]) -> Option<String> {
		None
	}
}

impl<T: Trait> ProvideInherent for Module<T> {
	type Call = Call<T>;
	type Error = MakeFatalError<RuntimeString>;
	const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

	fn create_inherent(_data: &InherentData) -> Option<Self::Call> {
		Some(Call::handle())
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
		let count = UnsignedSends::decode_len().unwrap_or(0) as u32;
		if count > 0 {
			let sends = UnsignedSends::get();
			for send in sends {
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

	/// Map transaction from bridge relayer
	fn relay_tx_map(target: T::AccountId, amount: T::Balance) {
		let asset_id: T::AssetId = 0.into();
		T::AssetIssue::asset_issue(asset_id, target.clone(), amount);
	}

	/// Receive and map transaction from backing blockchain
	fn receive_tx(target: T::AccountId, amount: T::Balance) {
		// Check if the relay transaction is verified
		match Self::relay_tx_verify() {
			Ok(_) => {
				// Relay transaction from backing blockchain
				Self::relay_tx_map(target, amount);

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
		let tx = TransactionOut::new();

		// Record transaction
		UnsignedReceiveConfirms::append([tx].into_iter());
	}

	/// Sign the receiving confirm transaction by validator
	fn receive_confirm_tx_sign() {
		let tx_vec: Vec<TransactionOut> = Self::unsigned_recv_cfms();
		let tx_vec = tx_vec.into_iter().filter_map(|item| {
			// Sign the transaction
			// Check if signature threshold is reached to submit transaction to backing blockchain
			match item.reach_threshold() {
				true => {
					match SignedReceiveConfirms::append([item.clone()].into_iter()) {
						Ok(_) => None,
						Err(_) => Some(item),
					}
				},
				false => Some(item),
			}
		}).collect::<Vec<_>>();

		UnsignedReceiveConfirms::put(tx_vec);
	}

	/// Send receiving confirm transaction
	fn receive_confirm_tx_send(t: Timestamp) {
		let tx_vec: Vec<TransactionOut> = Self::signed_recv_cfms();
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
		let mut tx = TransactionOut::new();
		tx.amount = amount.saturated_into::<u64>();
		tx.to_name = to_name;
		UnsignedSends::append([tx].into_iter());
	}

	/// Sign the sending transaction by validator
	fn send_tx_sign() {
		let tx_vec: Vec<TransactionOut> = Self::unsigned_sends();
		let tx_vec = tx_vec.into_iter().filter_map(|item| {
			// Sign the transaction
			// Check if signature threshold is reached to submit transaction to backing blockchain
			match item.reach_threshold() {
				true => {
					match SignedSends::append([item.clone()].into_iter()) {
						Ok(_) => None,
						Err(_) => Some(item),
					}
				},
				false => Some(item),
			}
		}).collect::<Vec<_>>();

		UnsignedSends::put(tx_vec);
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
