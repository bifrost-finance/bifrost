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

use srml_support::{decl_module, decl_event, decl_storage};
use system::{ensure_signed, ensure_none};

/// The module configuration trait.
pub trait Trait: system::Trait {
	/// The overarching event type.
	type Event: From<Event> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
	pub enum Event {

	}
);

decl_storage! {
	trait Store for Module<T: Trait> as Bridge {

	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {

		fn deposit_event() = default;

		/// Receive and map transaction from backing blockchain
		fn receive_tx(origin) {
			ensure_none(origin)?;

			Self::relay_tx_verify();

			Self::relay_tx_map();

			Self::receive_confirm_tx_gen();
		}

		/// Sign for the receiving confirm transaction
		fn receive_confirm_tx_sign(origin) {
			let _origin = ensure_signed(origin)?;
		}

		/// generate sending transaction
		fn send_tx_gen(origin) {
			let _origin = ensure_signed(origin)?;
		}

		/// Sign for the sending transaction
		fn send_tx_sign(origin) {
			ensure_none(origin)?;
		}

		fn on_initialize(_now_block: T::BlockNumber) {

		}

		fn on_finalize(_now_block: T::BlockNumber) {

		}

		// Runs after every block.
		fn offchain_worker(_now_block: T::BlockNumber) {

		}
	}
}

// The main implementation block for the module.
impl<T: Trait> Module<T> {

	/// verify transaction from bridge relayer
	fn relay_tx_verify() {

	}

	/// map transaction from bridge relayer
	fn relay_tx_map() {

	}

	/// generate receiving confirm transaction
	fn receive_confirm_tx_gen() {

	}

	/// send receiving confirm transaction
	fn receive_confirm_tx_send() {

	}

	/// send transaction finish
	fn send_tx_finish() {

	}
}
