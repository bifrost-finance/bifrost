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

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;


pub trait WeightInfo {
    fn add_to_issue_whitelist() -> Weight;
    fn remove_from_issue_whitelist() -> Weight;
    fn add_to_transfer_whitelist() -> Weight;
    fn remove_from_transfer_whitelist() -> Weight;
    fn issue() -> Weight;
    fn transfer() -> Weight;
}

impl WeightInfo for () {
    fn add_to_issue_whitelist() -> Weight {
        (50_000_000 as Weight)
    }

    fn remove_from_issue_whitelist() -> Weight {
        (50_000_000 as Weight)
    }
    
    fn add_to_transfer_whitelist() -> Weight {
        (50_000_000 as Weight)
    }

    fn remove_from_transfer_whitelist() -> Weight {
        (50_000_000 as Weight)
    }
    
    fn issue() -> Weight {
        (50_000_000 as Weight)
    }

    fn transfer() -> Weight {
        (50_000_000 as Weight)
    }
}