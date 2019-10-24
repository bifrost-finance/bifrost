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
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use system::ensure_root;
use sr_primitives::traits::{Member, SimpleArithmetic};
use srml_support::{StorageValue, Parameter, decl_module, decl_event, decl_storage};


pub trait Trait: system::Trait {
	type Event: From<Event> + Into<<Self as system::Trait>::Event>;
	type ExchangeRate: Member + Parameter + SimpleArithmetic + Default + Copy;
	type RatePerBlock: Member + Parameter + SimpleArithmetic + Default + Copy;
}

decl_storage! {
	trait Store for Module<T: Trait> as ExchangeStore {
		ExchangeRate get(fn get_exchange_rate): u64 = 1;
		RatePerBlock get(fn get_rate_per_block): u64 = 0;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		pub fn set_rate(origin, rate: u64) {
			let _sender = ensure_root(origin)?;
			<RatePerBlock>::put(rate);
		}

		pub fn set_exchange(origin, exchange: u64) {
			let _sender = ensure_root(origin)?;
			<ExchangeRate>::put(exchange);
		}

		fn on_finalize() {
			let rate_per_block = RatePerBlock::get();
			<ExchangeRate>::mutate(|rate| {
				*rate  = rate.saturating_sub(rate_per_block);
			});
		}
	}
}

decl_event!(
	pub enum Event {}
);
