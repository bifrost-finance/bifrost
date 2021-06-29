// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_api::decl_runtime_apis;

decl_runtime_apis! {
	pub trait BancorRuntimeApi<CurrencyId, Balance> where
		CurrencyId: Codec,
		Balance: Codec
	{
		/// pass in vstoken_amount and the token_id that wants to exchange for. And we can get the number of token accquired.
		fn get_bancor_token_amount_out(token_id: CurrencyId, vstoken_amount: Balance) -> Balance;

		/// pass in token_amount and the token_id to exchange for vstoken. And we can get the number of vstoken accquired.
		fn get_bancor_vstoken_amount_out(token_id: CurrencyId, token_amount: Balance) -> Balance;

		// get unit vstoken price in the form of token presentation.
		fn get_instant_vstoken_price(currency_id: CurrencyId) -> (Balance, Balance);

		// get unit token price in the form of vstoken presentation.
		fn get_instant_token_price(currency_id: CurrencyId) -> (Balance, Balance);
	}
}
