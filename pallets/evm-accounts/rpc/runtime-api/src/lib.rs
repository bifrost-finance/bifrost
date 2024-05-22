// Copyright (C) 2020-2022  Intergalactic, Limited (GIB).
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Runtime API definition for the EVM accounts pallet.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;

sp_api::decl_runtime_apis! {
	/// The API to query EVM account conversions.
	pub trait EvmAccountsApi<AccountId, EvmAddress> where
		AccountId: Codec,
		EvmAddress: Codec,
	{
		/// get the EVM address from the substrate address.
		fn evm_address(account_id: AccountId) -> EvmAddress;

		/// Return the Substrate address bound to the EVM account. If not bound, returns `None`.
		fn bound_account_id(evm_address: EvmAddress) -> Option<AccountId>;

		/// Get the Substrate address from the EVM address.
		/// Returns the truncated version of the address if the address wasn't bind.
		fn account_id(evm_address: EvmAddress) -> AccountId;
	}
}
