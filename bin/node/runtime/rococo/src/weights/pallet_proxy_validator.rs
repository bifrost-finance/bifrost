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

//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Trait> brml_proxy_validator::WeightInfo for WeightInfo<T> {
    fn set_global_asset() -> Weight {
        (0 as Weight)
            .saturating_add(T::DbWeight::get().reads(0 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn stake() -> Weight {
        (46665000 as Weight)
            //.saturating_add(T::DbWeight::get().reads(0 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn unstake() -> Weight {
        (27086000 as Weight)
            //.saturating_add(T::DbWeight::get().reads(0 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn validator_register() -> Weight {
        (39603000 as Weight)
            //.saturating_add(T::DbWeight::get().reads(0 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_need_amount() -> Weight {
        (110679000 as Weight)
            //.saturating_add(T::DbWeight::get().reads(0 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_reward_per_block() -> Weight {
        (110679000 as Weight)
            //.saturating_add(T::DbWeight::get().reads(0 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn deposit() -> Weight {
        (110679000 as Weight)
            //.saturating_add(T::DbWeight::get().reads(0 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn withdraw() -> Weight {
        (110679000 as Weight)
            //.saturating_add(T::DbWeight::get().reads(0 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
}