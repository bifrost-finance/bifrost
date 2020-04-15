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

use core::convert::{TryFrom, TryInto};
use frame_support::{decl_module, decl_event, decl_storage, ensure, Parameter};
use sp_runtime::traits::{AtLeast32Bit, Member, StaticLookup, Zero};
use frame_system::{self as system, ensure_root};

pub trait Trait: system::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	/// BNC Balance
	type Balance: Member + Parameter + Default + From<u128> + AtLeast32Bit + Copy + Zero + Into<u128>;
}

decl_event! {
	pub enum Event<T> where <T as system::Trait>::AccountId, <T as Trait>::Balance {
		/// A event indicate user receives transaction.
		IssuedVoucher(AccountId, Balance),
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Voucher {
		/// How much voucher you have
		pub BalancesVoucher get(fn voucher): map hasher(blake2_128_concat) T::AccountId => T::Balance;
		/// Total BNC in mainnet
		TotalSuppliedBNC get(fn toal_bnc): T::Balance = (80_000_000u128 * 10u128.pow(12)).try_into().map_err(|_| "failed to u128 conversion").unwrap();
		/// Current remaining BNC adds all others vouchers, equaling to TotalSuppliedBNC
		RemainingBNC get(fn remaining_bnc): T::Balance = (80_000_000u128 * 10u128.pow(12)).try_into().map_err(|_| "failed to u128 conversion").unwrap();
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		#[weight = frame_support::weights::SimpleDispatchInfo::default()]
		pub fn issue_voucher(
			origin,
			dest: <T::Lookup as StaticLookup>::Source,
			#[compact]
			amount: T::Balance,
		) {
			ensure_root(origin)?;

			let balance = <RemainingBNC<T>>::get();
			ensure!(balance >= amount, "the balance you transfer cannot bigger than all you have.");

			// ensure this address added into bifrost node
			let dest = T::Lookup::lookup(dest)?;

			// do not send balances for a account multiple times, just for one time
			if <BalancesVoucher<T>>::contains_key(&dest) {
				// increase balances for this account
				<BalancesVoucher<T>>::mutate(&dest, |balance| {
					*balance += amount;
				});
			} else {
				<BalancesVoucher<T>>::insert(&dest, amount);
			}

			// reduce from total BNC
			<RemainingBNC<T>>::mutate(|balance| {
					*balance -= amount;
			});

			Self::deposit_event(RawEvent::IssuedVoucher(dest, amount));
		}
	}
}
