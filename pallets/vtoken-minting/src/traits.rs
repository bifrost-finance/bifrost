// This file is part of Bifrost.

// Copyright (C) 2019-2022 Liebi Technologies (UK) Ltd.
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

// Ensure we're `no_std` when compiling for Wasm.

pub trait OnRedeemSuccess<AccountId, CurrencyId, Balance> {
	fn on_redeem_success(
		token_id: CurrencyId,
		to: AccountId,
		token_amount: Balance,
	) -> frame_support::pallet_prelude::Weight;
}

impl<AccountId, CurrencyId, Balance> OnRedeemSuccess<AccountId, CurrencyId, Balance> for () {
	fn on_redeem_success(
		_token_id: CurrencyId,
		_to: AccountId,
		_token_amount: Balance,
	) -> frame_support::pallet_prelude::Weight {
		0
	}
}
