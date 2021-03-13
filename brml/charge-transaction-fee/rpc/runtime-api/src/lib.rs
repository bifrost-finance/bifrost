// Copyright 2019-2021 Liebi Technologies.
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

use codec::Codec;
use sp_api::decl_runtime_apis;

decl_runtime_apis! {
	pub trait ChargeTransactionFeeRuntimeApi<CurrencyId, AccountId, Balance> where
		CurrencyId: Codec,
		AccountId: Codec,
		Balance: Codec,
	{
		/// get flexible fee token and amount to be deducted
		fn get_fee_token_and_amount(who: AccountId, fee: Balance) -> (CurrencyId, Balance);
	}
}
