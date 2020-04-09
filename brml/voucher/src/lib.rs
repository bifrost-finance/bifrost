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

use frame_support::{
	traits::{Currency, ExistenceRequirement, WithdrawReason, WithdrawReasons}, decl_module, decl_event, decl_storage, ensure
};
use sp_runtime::traits::StaticLookup;
use frame_system::{self as system, ensure_root};

pub trait Trait: system::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	/// inherit Currency trait
	type Currency: Currency<Self::AccountId>;
	/// Balance
	type Balance: From<<Self::Currency as Currency<Self::AccountId>>::Balance>;
}

decl_event! {
	pub enum Event<T> where <T as system::Trait>::AccountId, <T as Trait>::Balance {
		/// A event indicate user receives transaction.
		Transferred(AccountId, Balance),
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Voucher {
		pub BalancesVoucher get(fn voucher): map hasher(blake2_128_concat) T::AccountId =>
			<T::Currency as Currency<T::AccountId>>::Balance;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		pub fn issue_voucher(
			origin,
			source: T::AccountId,
			dest: <T::Lookup as StaticLookup>::Source,
			#[compact]
			amount: <T::Currency as Currency<T::AccountId>>::Balance,
		) {
			ensure_root(origin)?;

			let balance = <T::Currency as Currency<T::AccountId>>::free_balance(&source);
			ensure!(balance >= amount, "the balance you transfer cannot bigger than all you have.");

			// ensure this address added into bifrost node
			let dest = T::Lookup::lookup(dest)?;

			// create a reason mask
			let mut reason = WithdrawReasons::none();
			reason.set(WithdrawReason::Transfer);
			// reduce balances from source account
			<T::Currency as Currency<T::AccountId>>::withdraw(&source, amount, reason, ExistenceRequirement::AllowDeath)?;

			// do not send balances for a account multiple times, just for one time
			if <BalancesVoucher<T>>::contains_key(&dest) {
				// increase balances for this account
				<BalancesVoucher<T>>::mutate(&dest, |balance| {
					*balance += amount;
				});
			} else {
				<BalancesVoucher<T>>::insert(&dest, amount);
			}

			Self::deposit_event(RawEvent::Transferred(dest, T::Balance::from(amount)));
		}
	}
}
