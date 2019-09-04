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

use codec::{Encode, Decode};
use core::result;
use inherents::{RuntimeString, InherentIdentifier, ProvideInherent, InherentData, MakeFatalError};
#[cfg(feature = "std")]
use inherents::ProvideInherentData;
use node_primitives::AssetIssue;
use rstd::prelude::*;
use sr_primitives::traits::StaticLookup;
use srml_support::{decl_module, decl_event, decl_storage, StorageValue};
use substrate_primitives::offchain::{HttpRequestStatus, Duration, Timestamp, HttpError};
use system::{ensure_signed, ensure_none};


/// The identifier for the `bridge` inherent.
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"bridge01";

type TransactionSignature = Vec<u8>;

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
pub enum TransactionStatus {
	Generated,
	Signed,
	Sent,
	GenerateError,
	SignError,
	SendError,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
pub struct Transaction {
	tx: Vec<u8>,
	direction: BridgeTransactionDirection,
	signatures: Vec<TransactionSignature>,
	status: TransactionStatus,
	threshold: u32,
}

impl Transaction {
	fn new() -> Self {
		Self {
			tx: Default::default(),
			direction: BridgeTransactionDirection::In,
			signatures: Default::default(),
			status: TransactionStatus::Generated,
			threshold: 5,
		}
	}

	fn reach_threshold(&self) -> bool {
		self.signatures.len() >= self.threshold as usize
	}

	fn reach_timestamp(&self, t: Timestamp) -> bool { true }
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
pub enum BridgeTransactionDirection {
	In,
	Out,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
pub struct BridgeTransaction {
	direction: BridgeTransactionDirection,
	transaction: Vec<u8>,
	signatures: Vec<TransactionSignature>
}

#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug)]
pub struct Bank {
	account: Vec<u8>,
	authorities: Vec<Vec<u8>>,
	threshold: u32,
}

/// The module configuration trait.
pub trait Trait: system::Trait + brml_assets::Trait {
	/// The overarching event type.
	type Event: From<Event> + Into<<Self as system::Trait>::Event>;

	// Assets issue handler
	type AssetIssue: AssetIssue<Self::AssetId, Self::AccountId, Self::Balance>;
}

decl_event!(
	pub enum Event {
		BridgeTxMapping,

		BridgeTxReceived,

		BridgeTxReceiveConfirmed,

		BridgeTxSent,
	}
);

decl_storage! {
	trait Store for Module<T: Trait> as Bridge {

		NextBankId get(next_bank_id): u32;

		Banks get(banks): map u32 => Bank;

		BridgeTxs get(bridge_txs): Vec<BridgeTransaction>;

		UnsignedReceiveConfirms get(unsigned_recv_cfms): Vec<Transaction>;

		SignedReceiveConfirms get(signed_recv_cfms): Vec<Transaction>;

		UnsignedSends get(unsigned_sends): Vec<Transaction>;

		SignedSends get(signed_sends): Vec<Transaction>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {

		fn deposit_event() = default;

		fn handle(origin) {
			ensure_none(origin)?;

			Self::receive_tx();

			Self::send_tx_sign();

			Self::receive_confirm_tx_sign();
		}

		// Runs after every block.
		fn offchain_worker(_now_block: T::BlockNumber) {
			Self::offchain();
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

// The main implementation block for the module.
impl<T: Trait> Module<T> {

	fn offchain() {
		let now_time = sr_io::timestamp();
		Self::receive_confirm_tx_send(now_time);

		Self::send_tx_finish(now_time);
	}

	/// Verify transaction from bridge relayer by validator
	fn relay_tx_verify() -> result::Result<(), ()> {
		Ok(())
	}

	/// Map transaction from bridge relayer
	fn relay_tx_map() {
		let asset_id: T::AssetId = T::AssetId::default();
		let amount: T::Balance = T::Balance::default();
//		let target: <T::Lookup as StaticLookup>::Source = T::AccountId::default();
//		let target = T::Lookup::lookup(target).unwrap();
		let target = T::AccountId::default();
		T::AssetIssue::asset_issue(asset_id, target.clone(), amount);
	}

	/// Receive and map transaction from backing blockchain
	fn receive_tx() {
		// Check if the relay transaction is verified
		match Self::relay_tx_verify() {
			Ok(_) => {
				// Relay transaction from backing blockchain
				Self::relay_tx_map();

				// Generate receive confirmation transaction for blockchain
				Self::receive_confirm_tx_gen();

				Self::deposit_event(Event::BridgeTxMapping);
			},
			Err(_) => {}
		}
	}

	/// Generate receiving confirm transaction
	fn receive_confirm_tx_gen() {
		// Generate transaction
		let tx = Transaction::new();

		// Record transaction
		UnsignedReceiveConfirms::append([tx].into_iter());
	}

	/// Sign the receiving confirm transaction by validator
	fn receive_confirm_tx_sign() {
		let tx_vec = Self::unsigned_recv_cfms();
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
		let tx_vec = Self::signed_recv_cfms();
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
	pub fn send_tx_gen() {
		let tx = Transaction::new();
		UnsignedSends::append([tx].into_iter());
	}

	/// Sign the sending transaction by validator
	fn send_tx_sign() {
		let tx_vec = Self::unsigned_sends();
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
