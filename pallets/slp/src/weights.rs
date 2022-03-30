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

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for the pallet.
pub trait WeightInfo {
	// Outer calls
	fn initialize_delegator() -> Weight;
	fn bond() -> Weight;
	fn bond_extra() -> Weight;
	fn unbond() -> Weight;
	fn unbond_all() -> Weight;
	fn rebond() -> Weight;
	fn delegate() -> Weight;
	fn redelegate() -> Weight;
	fn undelegate() -> Weight;
	fn payout() -> Weight;
	fn liquidize() -> Weight;
	fn chill() -> Weight;
	fn transfer_back() -> Weight;
	fn transfer_to() -> Weight;
	fn increase_token_pool() -> Weight;
	fn decrease_token_pool() -> Weight;
	fn update_ongoing_time_unit() -> Weight;
	fn refund_currency_due_unbond() -> Weight;
	fn increase_token_to_add() -> Weight;
	fn decrease_token_to_add() -> Weight;
	fn increase_token_to_deduct() -> Weight;
	fn decrease_token_to_deduct() -> Weight;
	fn supplement_fee_reserve() -> Weight;
	fn charge_host_fee_and_tune_vtoken_exchange_rate() -> Weight;
	fn confirm_delegator_ledger_query_response() -> Weight;
	fn fail_delegator_ledger_query_response() -> Weight;
	fn confirm_validators_by_delegator_query_response() -> Weight;
	fn fail_validators_by_delegator_query_response() -> Weight;

	// Storage setters
	fn set_xcm_dest_weight_and_fee() -> Weight;
	fn set_operate_origin() -> Weight;
	fn set_current_time_unit() -> Weight;
	fn set_fee_source() -> Weight;
	fn add_delegator() -> Weight;
	fn remove_delegator() -> Weight;
	fn add_validator() -> Weight;
	fn remove_validator() -> Weight;
	fn set_validators_by_delegator() -> Weight;
	fn set_delegator_ledger() -> Weight;
	fn set_minimums_and_maximums() -> Weight;
	fn set_currency_delays() -> Weight;
	fn set_hosting_fees() -> Weight;
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Outer calls
	fn initialize_delegator() -> Weight {
		(50_000_000 as Weight)
	}

	fn bond() -> Weight {
		(50_000_000 as Weight)
	}

	fn bond_extra() -> Weight {
		(50_000_000 as Weight)
	}

	fn unbond() -> Weight {
		(50_000_000 as Weight)
	}

	fn unbond_all() -> Weight {
		(50_000_000 as Weight)
	}

	fn rebond() -> Weight {
		(50_000_000 as Weight)
	}

	fn delegate() -> Weight {
		(50_000_000 as Weight)
	}

	fn redelegate() -> Weight {
		(50_000_000 as Weight)
	}

	fn undelegate() -> Weight {
		(50_000_000 as Weight)
	}

	fn payout() -> Weight {
		(50_000_000 as Weight)
	}

	fn liquidize() -> Weight {
		(50_000_000 as Weight)
	}

	fn chill() -> Weight {
		(50_000_000 as Weight)
	}

	fn transfer_back() -> Weight {
		(50_000_000 as Weight)
	}

	fn transfer_to() -> Weight {
		(50_000_000 as Weight)
	}

	fn increase_token_pool() -> Weight {
		(50_000_000 as Weight)
	}

	fn decrease_token_pool() -> Weight {
		(50_000_000 as Weight)
	}

	fn update_ongoing_time_unit() -> Weight {
		(50_000_000 as Weight)
	}

	fn refund_currency_due_unbond() -> Weight {
		(50_000_000 as Weight)
	}

	fn increase_token_to_add() -> Weight {
		(50_000_000 as Weight)
	}

	fn decrease_token_to_add() -> Weight {
		(50_000_000 as Weight)
	}

	fn increase_token_to_deduct() -> Weight {
		(50_000_000 as Weight)
	}

	fn decrease_token_to_deduct() -> Weight {
		(50_000_000 as Weight)
	}

	fn supplement_fee_reserve() -> Weight {
		(50_000_000 as Weight)
	}

	fn charge_host_fee_and_tune_vtoken_exchange_rate() -> Weight {
		(50_000_000 as Weight)
	}

	// Storage setters
	fn set_xcm_dest_weight_and_fee() -> Weight {
		(50_000_000 as Weight)
	}

	fn set_operate_origin() -> Weight {
		(50_000_000 as Weight)
	}

	fn set_current_time_unit() -> Weight {
		(50_000_000 as Weight)
	}

	fn set_fee_source() -> Weight {
		(50_000_000 as Weight)
	}

	fn add_delegator() -> Weight {
		(50_000_000 as Weight)
	}

	fn remove_delegator() -> Weight {
		(50_000_000 as Weight)
	}

	fn add_validator() -> Weight {
		(50_000_000 as Weight)
	}

	fn remove_validator() -> Weight {
		(50_000_000 as Weight)
	}

	fn set_validators_by_delegator() -> Weight {
		(50_000_000 as Weight)
	}

	fn set_delegator_ledger() -> Weight {
		(50_000_000 as Weight)
	}

	fn set_minimums_and_maximums() -> Weight {
		(50_000_000 as Weight)
	}

	fn set_currency_delays() -> Weight {
		(50_000_000 as Weight)
	}

	fn set_hosting_fees() -> Weight {
		(50_000_000 as Weight)
	}

	fn confirm_delegator_ledger_query_response() -> Weight {
		(50_000_000 as Weight)
	}

	fn fail_delegator_ledger_query_response() -> Weight {
		(50_000_000 as Weight)
	}

	fn confirm_validators_by_delegator_query_response() -> Weight {
		(50_000_000 as Weight)
	}

	fn fail_validators_by_delegator_query_response() -> Weight {
		(50_000_000 as Weight)
	}
}
