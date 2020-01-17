// Copyright 2019-2020 Liebi Technologies.
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

mod mock;
mod tests;

use frame_support::{Parameter, decl_module, decl_storage};
use sp_runtime::traits::{Member, SimpleArithmetic};
use system::ensure_root;

pub trait Trait: system::Trait {
	type ExchangeRate: Member + Parameter + SimpleArithmetic + Default + Copy;

	type RatePerBlock: Member + Parameter + SimpleArithmetic + Default + Copy;
}

decl_storage! {
	trait Store for Module<T: Trait> as Exchange {
		ExchangeRate get(fn get_exchange_rate): u64 = 1;
		RatePerBlock get(fn get_rate_per_block): u64 = 0;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		pub fn set_rate_per_block(origin, rate: u64) {
			let _sender = ensure_root(origin)?;
			<RatePerBlock>::put(rate);
		}

		pub fn set_exchange_rate(origin, exchange: u64) {
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
