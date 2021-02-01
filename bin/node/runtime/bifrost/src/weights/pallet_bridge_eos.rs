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

//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

use frame_support::{
    traits::Get, weights::Weight
};
use sp_std::marker::PhantomData;
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> brml_bridge_eos::WeightInfo for WeightInfo<T> {
    fn clear_cross_trade_times() -> Weight {
        (65949000 as Weight)
            // .saturating_add(T::DbWeight::get().reads(0 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn bridge_enable() -> Weight {
        (46665000 as Weight)
            // .saturating_add(T::DbWeight::get().reads(0 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn save_producer_schedule() -> Weight {
        (27086000 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    fn init_schedule() -> Weight {
        (39603000 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn grant_crosschain_privilege() -> Weight {
        (110679000 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn remove_crosschain_privilege() -> Weight {
        (0 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_contract_accounts() -> Weight {
        (0 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn change_schedule() -> Weight {
        (0 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn prove_action() -> Weight {
        (0 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn bridge_tx_report() -> Weight {
        (0 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn update_bridge_trx_status() -> Weight {
        (0 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn trial_on_trx_status() -> Weight {
        (0 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn cross_to_eos(weight:Weight) -> Weight {
        let db = T::DbWeight::get();
        db.writes(1) // put task to tx_out
            .saturating_add(db.reads(1)) // token exists or not
            .saturating_add(db.reads(1)) // get token
            .saturating_add(db.reads(1)) // get account asset
            .saturating_add(weight.saturating_add(10000)) // memo length
            .saturating_mul(1000)
    }
}