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

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> brml_vtoken_mint::WeightInfo for WeightInfo<T> {
    fn to_vtoken<U: brml_vtoken_mint::Config>() -> Weight {
        let referer_weight = 1000;
        let db = T::DbWeight::get();
        db.reads_writes(1, 1)
            .saturating_add(referer_weight) // memo length
    }
    fn to_token() -> Weight {
        (39_603_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
}