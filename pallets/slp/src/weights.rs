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
	fn rebond() -> Weight;
	fn delegate() -> Weight;
	fn redelegate() -> Weight;
	fn undelegate() -> Weight;

	// Storage setters
	fn set_xcm_dest_weight_and_fee() -> Weight;
	fn set_operate_origin() -> Weight;
	fn set_current_time_unit() -> Weight;
	fn set_currency_delays() -> Weight;
	fn set_fee_source() -> Weight;
	fn set_delegators() -> Weight;
	fn set_validators() -> Weight;
	fn set_validators_by_delegator() -> Weight;
	fn set_delegator_ledger() -> Weight;
	fn set_minimums_and_maximums() -> Weight;
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

	fn set_currency_delays() -> Weight {
		(50_000_000 as Weight)
	}

	fn set_fee_source() -> Weight {
		(50_000_000 as Weight)
	}

	fn set_delegators() -> Weight {
		(50_000_000 as Weight)
	}

	fn set_validators() -> Weight {
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
}
